//! Application state machine: idle → recording → processing → idle.

use std::path::PathBuf;
use std::sync::{mpsc, Arc, Mutex, OnceLock};
use std::time::Duration;

use serde::Serialize;
use tauri::menu::MenuItem;
use tauri::tray::TrayIcon;
use tauri::{AppHandle, Emitter, Manager, Wry};

use crate::audio::Recording;
use crate::config::Config;
use crate::keys::Shortcut;
use crate::paste::Clipboard;
use crate::stt::Engine;
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
}

pub struct App {
    pub handle: AppHandle,
    pub paths: Paths,
    pub config: Mutex<Config>,
    pub config_error: Option<String>,
    phase: Mutex<Phase>,
    recording: Mutex<Option<Recording>>,
    engine: OnceLock<Engine>,
    clipboard: Mutex<Option<Clipboard>>,
    status: Mutex<String>,
    tray: OnceLock<(TrayIcon, MenuItem<Wry>)>,
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
            paths,
            config: Mutex::new(config),
            config_error,
            phase: Mutex::new(Phase::Loading),
            recording: Mutex::new(None),
            engine: OnceLock::new(),
            clipboard: Mutex::new(None),
            status: Mutex::new("Starting".into()),
            tray: OnceLock::new(),
        }
    }

    pub fn set_tray(&self, tray: TrayIcon, status_item: MenuItem<Wry>) {
        let _ = self.tray.set((tray, status_item));
        self.set_status(&self.status());
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
        let mut phase = self.phase.lock().unwrap();
        match *phase {
            Phase::Idle => match Recording::start() {
                Ok(rec) => {
                    *self.recording.lock().unwrap() = Some(rec);
                    *phase = Phase::Recording;
                    overlay::show(&self.handle, "recording");
                }
                Err(e) => self.fail(e),
            },
            Phase::Recording => {
                let Some(rec) = self.recording.lock().unwrap().take() else {
                    return;
                };
                *phase = Phase::Processing;
                overlay::show(&self.handle, "processing");
                let app = self.clone();
                std::thread::spawn(move || app.finish(rec));
            }
            Phase::Loading | Phase::Processing => {}
        }
    }

    fn finish(self: &Arc<Self>, rec: Recording) {
        let result = self.transcribe_and_paste(rec);
        *self.phase.lock().unwrap() = Phase::Idle;
        match result {
            Ok(()) => overlay::hide(&self.handle),
            Err(e) => self.fail(e),
        }
    }

    fn transcribe_and_paste(&self, rec: Recording) -> Result<(), String> {
        let audio = rec.stop()?;
        if audio.duration_secs() < MIN_RECORDING_SECS {
            return Ok(());
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
            return Ok(());
        }

        {
            let mut clipboard = self.clipboard.lock().unwrap();
            if clipboard.is_none() {
                *clipboard = Some(Clipboard::new()?);
            }
            clipboard.as_mut().unwrap().set(&text)?;
        }
        std::thread::sleep(Duration::from_millis(50));

        let shortcut = Shortcut::parse(&self.config.lock().unwrap().paste_shortcut)?;
        let (tx, rx) = mpsc::channel();
        self.handle
            .run_on_main_thread(move || {
                let _ = tx.send(paste::send(&shortcut));
            })
            .map_err(|e| e.to_string())?;
        rx.recv().map_err(|e| e.to_string())?
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
    config_error: Option<String>,
    status: String,
    shortcut: String,
    platform: &'static str,
}

type State<'a> = tauri::State<'a, Arc<App>>;

#[tauri::command]
pub fn app_info(app: State) -> AppInfo {
    AppInfo {
        version: crate::VERSION,
        repo_url: crate::REPO_URL,
        config_path: app.paths.config.display().to_string(),
        log_dir: app.paths.logs.display().to_string(),
        config_error: app.config_error.clone(),
        status: app.status(),
        shortcut: app.config.lock().unwrap().shortcut.clone(),
        platform: std::env::consts::OS,
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
pub fn set_hotkey_paused(paused: bool) {
    crate::hotkey::set_paused(paused);
}

#[tauri::command]
pub fn open_path(app: State, kind: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let path = match kind.as_str() {
        "config" => app.paths.config.clone(),
        "logs" => app.paths.logs.clone(),
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
