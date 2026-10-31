//! Application state machine: idle → recording → processing → idle.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex, OnceLock};
use std::thread::JoinHandle;
use std::time::Duration;

use serde::Serialize;
use tauri::menu::MenuItem;
use tauri::tray::TrayIcon;
use tauri::{AppHandle, Emitter, Manager, Wry};

use crate::audio::Recording;
use crate::config::Config;
use crate::history::{Entry, History};
use crate::keys::Shortcut;
use crate::paste::Clipboard;
use crate::stt::Engine;
use crate::update;
use crate::{overlay, paste};

/// Recordings shorter than this are treated as accidental taps.
const MIN_RECORDING_SECS: f32 = 0.3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Loading,
    Idle,
    Recording,
    Processing,
}

pub struct Paths {
    pub config: PathBuf,
    pub logs: PathBuf,
    pub models: PathBuf,
    pub history: PathBuf,
    /// Restore token of the Wayland RemoteDesktop portal (ADR 0007).
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    pub portal_token: PathBuf,
}

pub struct App {
    pub handle: AppHandle,
    pub paths: Paths,
    history: History,
    pub config: Mutex<Config>,
    pub config_error: Option<String>,
    phase: Mutex<Phase>,
    /// The running recording and whether to paste its text.
    recording: Mutex<Option<(Recording, bool)>>,
    engine: OnceLock<Engine>,
    clipboard: Mutex<Option<Clipboard>>,
    status: Mutex<String>,
    tray: OnceLock<(TrayIcon, MenuItem<Wry>)>,
    pub update: update::State,
    updating: AtomicBool,
}

impl App {
    pub fn new(
        handle: AppHandle,
        paths: Paths,
        config: Config,
        config_error: Option<String>,
    ) -> App {
        App {
            handle,
            history: History::new(paths.history.clone()),
            paths,
            config: Mutex::new(config),
            config_error,
            phase: Mutex::new(Phase::Loading),
            recording: Mutex::new(None),
            engine: OnceLock::new(),
            clipboard: Mutex::new(None),
            status: Mutex::new("Starting".into()),
            tray: OnceLock::new(),
            update: update::State::default(),
            updating: AtomicBool::new(false),
        }
    }

    pub fn set_tray(&self, tray: TrayIcon, status_item: MenuItem<Wry>) {
        let _ = self.tray.set((tray, status_item));
        self.set_status(&self.status());
    }

    pub fn hide_tray(&self) {
        if let Some((tray, _)) = self.tray.get() {
            let _ = tray.set_visible(false);
        }
    }

    /// Claim the app for an update: refused while dictating or while another update runs.
    /// The hotkey is ignored until `end_update`.
    pub fn begin_update(&self) -> Result<(), String> {
        let phase = self.phase.lock().unwrap();
        if matches!(*phase, Phase::Recording | Phase::Processing) {
            return Err("Finish the current dictation before updating.".into());
        }
        self.updating
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .map(|_| ())
            .map_err(|_| "An update is already being installed.".to_string())
    }

    pub fn end_update(&self) {
        self.updating.store(false, Ordering::SeqCst);
    }

    pub fn status(&self) -> String {
        self.status.lock().unwrap().clone()
    }

    pub fn set_status(&self, status: &str) {
        *self.status.lock().unwrap() = status.to_string();
        if let Some((tray, item)) = self.tray.get() {
            let _ = item.set_text(format!("Jabberwrite: {status}"));
            let _ = tray.set_tooltip(Some(format!("Jabberwrite: {status}")));
        }
        let _ = self.handle.emit("status", status);
    }

    /// Download (if needed) and load the model. Runs on a background thread.
    pub fn load_model(&self) {
        // Bind first: a lock guard in the match scrutinee would be held during the download.
        let model_path = self.config.lock().unwrap().model_path.clone();
        let dir = match model_path {
            Some(dir) => Ok(dir),
            None => crate::stt::ensure_model(&self.paths.models, |p| {
                self.set_status(&format!("Downloading model {:.0}%", p * 100.0))
            }),
        };
        self.set_status("Loading model");
        match dir.and_then(|d| Engine::load(&d)) {
            Ok(engine) => {
                let _ = self.engine.set(engine);
                *self.phase.lock().unwrap() = Phase::Idle;
                self.set_status("Ready");
                tracing::info!("model loaded");
            }
            Err(e) => {
                tracing::error!("{e}");
                self.set_status(&format!("Error: {e}"));
            }
        }
    }

    /// Hotkey handler. Never call on the hook thread; it may block briefly.
    pub fn toggle(self: &Arc<Self>) {
        let phase = *self.phase.lock().unwrap();
        match phase {
            Phase::Idle => self.start(true),
            Phase::Recording => {
                self.stop();
            }
            Phase::Loading | Phase::Processing => {}
        }
    }

    /// Start recording if idle. With `paste`, the text is pasted into the
    /// focused window when the recording stops.
    pub fn start(self: &Arc<Self>, paste: bool) {
        let mut phase = self.phase.lock().unwrap();
        if self.updating.load(Ordering::SeqCst) || *phase != Phase::Idle {
            return;
        }
        match Recording::start() {
            Ok(rec) => {
                *self.recording.lock().unwrap() = Some((rec, paste));
                *phase = Phase::Recording;
                overlay::show(&self.handle, "recording");
            }
            Err(e) => self.fail(e),
        }
    }

    /// Stop recording and recognize in the background. The handle yields the
    /// recognized text; `None` if nothing was recording.
    pub fn stop(self: &Arc<Self>) -> Option<JoinHandle<Result<Option<String>, String>>> {
        let mut phase = self.phase.lock().unwrap();
        if *phase != Phase::Recording {
            return None;
        }
        let (rec, paste) = self.recording.lock().unwrap().take()?;
        *phase = Phase::Processing;
        overlay::show(&self.handle, "processing");
        let app = self.clone();
        Some(std::thread::spawn(move || app.finish(rec, paste)))
    }

    fn finish(self: &Arc<Self>, rec: Recording, paste: bool) -> Result<Option<String>, String> {
        let result = self.transcribe(rec, paste);
        *self.phase.lock().unwrap() = Phase::Idle;
        match &result {
            Ok(_) => overlay::hide(&self.handle),
            Err(e) => self.fail(e.clone()),
        }
        result
    }

    fn transcribe(&self, rec: Recording, paste: bool) -> Result<Option<String>, String> {
        let audio = rec.stop()?;
        if audio.duration_secs() < MIN_RECORDING_SECS {
            return Ok(None);
        }
        if audio.is_silent() {
            return Err("the microphone delivered only silence; check the input device".into());
        }
        let engine = self.engine.get().ok_or("model not loaded")?;
        let started = std::time::Instant::now();
        let text = engine.transcribe(&audio.samples, audio.sample_rate)?;
        tracing::info!(
            "transcribed {:.1}s of audio in {:.2}s",
            audio.duration_secs(),
            started.elapsed().as_secs_f32()
        );
        if text.is_empty() {
            return Ok(None);
        }

        // Logged before pasting so the text survives a failed paste.
        match self.history.append(&text) {
            Ok(entry) => {
                let _ = self.handle.emit("history", entry);
            }
            Err(e) => tracing::error!("{e}"),
        }

        if paste {
            self.paste_text(&text)?;
        }
        Ok(Some(text))
    }

    /// Copy to the clipboard and send the paste shortcut to the focused window.
    fn paste_text(&self, text: &str) -> Result<(), String> {
        self.copy(text)?;
        std::thread::sleep(Duration::from_millis(50));

        let shortcut = Shortcut::parse(&self.config.lock().unwrap().paste_shortcut)?;
        #[cfg(target_os = "linux")]
        if paste::wayland::active() {
            return paste::wayland::send(&shortcut, &self.paths.portal_token);
        }
        let (tx, rx) = mpsc::channel();
        self.handle
            .run_on_main_thread(move || {
                let _ = tx.send(paste::send(&shortcut));
            })
            .map_err(|e| e.to_string())?;
        rx.recv().map_err(|e| e.to_string())?
    }

    fn copy(&self, text: &str) -> Result<(), String> {
        let mut clipboard = self.clipboard.lock().unwrap();
        if clipboard.is_none() {
            *clipboard = Some(Clipboard::new()?);
        }
        clipboard.as_mut().unwrap().set(text)
    }

    pub fn clear_history_if_configured(&self) {
        if self.config.lock().unwrap().clear_history_on_exit {
            if let Err(e) = self.history.clear() {
                tracing::error!("{e}");
            }
        }
    }

    fn fail(self: &Arc<Self>, error: String) {
        tracing::error!("{error}");
        self.set_status(&format!("Error: {error}"));
        overlay::show(&self.handle, "error");
        let app = self.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_secs(2));
            if matches!(*app.phase.lock().unwrap(), Phase::Idle | Phase::Loading) {
                overlay::hide(&app.handle);
            }
        });
    }

    pub fn show_settings(&self) {
        if let Some(window) = self.handle.get_webview_window("settings") {
            let _ = window.show();
            let _ = window.unminimize();
            let _ = window.set_focus();
        }
    }
}

// ---- Commands for the settings window ----

#[derive(Serialize)]
pub struct AppInfo {
    version: &'static str,
    repo_url: &'static str,
    config_path: String,
    log_dir: String,
    history_path: String,
    clear_history_on_exit: bool,
    config_error: Option<String>,
    status: String,
    shortcut: String,
    platform: &'static str,
    update: update::Status,
}

type State<'a> = tauri::State<'a, Arc<App>>;

#[tauri::command]
pub fn app_info(app: State) -> AppInfo {
    let config = app.config.lock().unwrap().clone();
    AppInfo {
        version: crate::VERSION,
        repo_url: crate::REPO_URL,
        config_path: app.paths.config.display().to_string(),
        log_dir: app.paths.logs.display().to_string(),
        history_path: app.paths.history.display().to_string(),
        clear_history_on_exit: config.clear_history_on_exit,
        config_error: app.config_error.clone(),
        status: app.status(),
        shortcut: config.shortcut,
        platform: std::env::consts::OS,
        update: app.update.status(),
    }
}

/// Validate, persist and apply a new shortcut; returns its normalized form.
#[tauri::command]
pub fn save_shortcut(app: State, shortcut: String) -> Result<String, String> {
    if app.config_error.is_some() {
        return Err("The config file is invalid. Fix it by hand and restart.".into());
    }
    let parsed = Shortcut::parse(&shortcut)?;
    let mut config = app.config.lock().unwrap();
    let mut updated = config.clone();
    updated.shortcut = parsed.to_string();
    crate::config::save(&app.paths.config, &updated)?;
    *config = updated;
    crate::hotkey::set_shortcut(parsed);
    tracing::info!("shortcut changed to {parsed}");
    Ok(parsed.to_string())
}

#[tauri::command]
pub fn save_clear_history_on_exit(app: State, enabled: bool) -> Result<(), String> {
    if app.config_error.is_some() {
        return Err("The config file is invalid. Fix it by hand and restart.".into());
    }
    let mut config = app.config.lock().unwrap();
    let mut updated = config.clone();
    updated.clear_history_on_exit = enabled;
    crate::config::save(&app.paths.config, &updated)?;
    *config = updated;
    Ok(())
}

#[tauri::command]
pub fn history(app: State) -> Vec<Entry> {
    app.history.list()
}

#[tauri::command]
pub fn clear_history(app: State) -> Result<(), String> {
    app.history.clear()
}

#[tauri::command]
pub fn copy_text(app: State, text: String) -> Result<(), String> {
    app.copy(&text)
}

/// Start a test recording from the settings window; its text is not pasted.
#[tauri::command]
pub fn record_start(app: State) {
    app.start(false);
}

/// Stop the test recording and return the recognized text.
#[tauri::command]
pub async fn record_stop(app: State<'_>) -> Result<Option<String>, String> {
    let app = app.inner().clone();
    tauri::async_runtime::spawn_blocking(move || match app.stop() {
        Some(handle) => handle
            .join()
            .map_err(|_| "recognition panicked".to_string())?,
        None => Ok(None),
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn set_hotkey_paused(paused: bool) {
    crate::hotkey::set_paused(paused);
}

#[tauri::command]
pub fn open_path(app: State, kind: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let path = match kind.as_str() {
        "config" => app.paths.config.clone(),
        "logs" => app.paths.logs.clone(),
        "history" => app.paths.history.clone(),
        _ => return Err(format!("unknown path {kind}")),
    };
    app.handle
        .opener()
        .reveal_item_in_dir(path)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_url(app: State, url: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    if !url.starts_with("https://") {
        return Err("only https links can be opened".into());
    }
    app.handle
        .opener()
        .open_url(url, None::<&str>)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn permissions() -> Option<crate::permissions::Permissions> {
    crate::permissions::check()
}

#[tauri::command]
pub fn open_permission_settings(app: State, kind: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let url = crate::permissions::settings_url(&kind).ok_or("unknown permission")?;
    app.handle
        .opener()
        .open_url(url, None::<&str>)
        .map_err(|e| e.to_string())
}
