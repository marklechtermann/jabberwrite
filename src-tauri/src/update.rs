//! In-app updates from GitHub Releases (ADR 0005). Checks only run on request, never in the background.

use std::sync::{Arc, Mutex};

use serde::Serialize;
use tauri::{Emitter, Manager};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tauri_plugin_updater::{Update, UpdaterExt};

use crate::app::App;

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Status {
    #[default]
    Idle,
    Checking,
    UpToDate,
    Available {
        version: String,
    },
    Installing,
    Error {
        message: String,
    },
}

#[derive(Default)]
pub struct State {
    status: Mutex<Status>,
    pending: Mutex<Option<Update>>,
}

impl State {
    pub fn status(&self) -> Status {
        self.status.lock().unwrap().clone()
    }
}

fn set_status(app: &App, status: Status) {
    *app.update.status.lock().unwrap() = status.clone();
    let _ = app.handle.emit("update", status);
}

/// Dev builds report `…-dev+sha` and would see every release as an update.
fn is_dev_build() -> bool {
    crate::VERSION.contains("-dev")
}

/// Check GitHub for a newer release and publish the result as the update status.
pub async fn check(app: &App) -> Status {
    {
        // Don't start a second check or touch the state while an install runs.
        let mut status = app.update.status.lock().unwrap();
        if matches!(*status, Status::Checking | Status::Installing) {
            return status.clone();
        }
        *status = Status::Checking;
    }
    let _ = app.handle.emit("update", Status::Checking);

    let status = match find_update(app).await {
        Ok(Some(update)) => {
            tracing::info!("update available: {}", update.version);
            let version = update.version.clone();
            *app.update.pending.lock().unwrap() = Some(update);
            Status::Available { version }
        }
        Ok(None) => {
            tracing::info!("no update available");
            *app.update.pending.lock().unwrap() = None;
            Status::UpToDate
        }
        Err(message) => {
            tracing::error!("update check failed: {message}");
            Status::Error { message }
        }
    };
    set_status(app, status.clone());
    status
}

async fn find_update(app: &App) -> Result<Option<Update>, String> {
    if is_dev_build() {
        return Err("Updates are only available in release builds.".into());
    }
    let handle = app.handle.clone();
    app.handle
        .updater_builder()
        // The Windows installer replaces our files, so the app exits right after launching it
        // without running any cleanup; hide the tray icon to avoid a ghost icon.
        .on_before_exit(move || {
            if let Some(app) = handle.try_state::<Arc<App>>() {
                app.hide_tray();
            }
        })
        .build()
        .map_err(|e| e.to_string())?
        .check()
        .await
        .map_err(|e| e.to_string())
}

/// Download and install the pending update, then restart. Only returns on failure.
pub async fn install(app: &Arc<App>) -> Result<(), String> {
    let update = app
        .update
        .pending
        .lock()
        .unwrap()
        .clone()
        .ok_or("No update available. Check for updates first.")?;
    app.begin_update()?;
    set_status(app, Status::Installing);
    tracing::info!("installing update {}", update.version);

    let mut downloaded = 0u64;
    let mut last_percent = None;
    let result = update
        .download_and_install(
            |chunk, total| {
                downloaded += chunk as u64;
                let percent = total.map(|t| (downloaded * 100 / t.max(1)).min(100));
                if percent != last_percent {
                    last_percent = percent;
                    if let Some(p) = percent {
                        app.set_status(&format!("Downloading update {p}%"));
                    }
                }
            },
            || app.set_status("Installing update"),
        )
        .await;

    match result {
        // Windows never gets here: the installer takes over and the app exits.
        Ok(()) => {
            tracing::info!("update installed, restarting");
            app.handle.restart()
        }
        Err(e) => {
            let message = e.to_string();
            tracing::error!("update failed: {message}");
            app.end_update();
            app.set_status("Update failed");
            set_status(app, Status::Error { message });
            Err(e.to_string())
        }
    }
}

/// Tray flow: check, then ask in a native dialog.
pub async fn check_with_dialogs(app: Arc<App>) {
    match check(&app).await {
        Status::Available { version } => {
            let text = format!(
                "Jabberwrite {version} is available (you have {}).\n\nInstall it now? Jabberwrite will restart.",
                crate::VERSION
            );
            if confirm(&app, text).await {
                if let Err(e) = install(&app).await {
                    notify(
                        &app,
                        MessageDialogKind::Error,
                        format!("Update failed: {e}"),
                    )
                    .await;
                }
            }
        }
        Status::UpToDate => {
            let text = format!("Jabberwrite {} is up to date.", crate::VERSION);
            notify(&app, MessageDialogKind::Info, text).await;
        }
        Status::Error { message } => {
            let text = format!("Could not check for updates: {message}");
            notify(&app, MessageDialogKind::Error, text).await;
        }
        // A check or install is already running; the status shows its progress.
        Status::Idle | Status::Checking | Status::Installing => {}
    }
}

async fn confirm(app: &App, text: String) -> bool {
    let handle = app.handle.clone();
    tauri::async_runtime::spawn_blocking(move || {
        handle
            .dialog()
            .message(text)
            .title("Jabberwrite update")
            .buttons(MessageDialogButtons::OkCancelCustom(
                "Install".into(),
                "Later".into(),
            ))
            .blocking_show()
    })
    .await
    .unwrap_or(false)
}

async fn notify(app: &App, kind: MessageDialogKind, text: String) {
    let handle = app.handle.clone();
    let _ = tauri::async_runtime::spawn_blocking(move || {
        handle
            .dialog()
            .message(text)
            .title("Jabberwrite update")
            .kind(kind)
            .blocking_show()
    })
    .await;
}

#[tauri::command]
pub async fn check_update(app: tauri::State<'_, Arc<App>>) -> Result<Status, String> {
    Ok(check(&app).await)
}

#[tauri::command]
pub async fn install_update(app: tauri::State<'_, Arc<App>>) -> Result<(), String> {
    install(&app).await
}
