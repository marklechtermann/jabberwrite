//! In-app updates from GitHub Releases (ADR 0005). Checks only run on request, never in the background.
//! Installs from the Arch package update through `pacman` (ADR 0010).

use std::path::{Path, PathBuf};
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

/// The package format of this install, when it differs from what the binary was bundled as.
#[derive(Clone, Copy, Debug, PartialEq)]
enum PackageFormat {
    /// The Arch package, a repackaged `.deb`: the updater would otherwise try `dpkg`.
    Pacman,
}

/// `latest.json` entry for the Arch package; the release workflow adds it.
const PACMAN_TARGET: &str = "linux-x86_64-pacman";

/// A package ships this file next to the binary's prefix, e.g. `/usr/share/jabberwrite/package-format`
/// for `/usr/bin/jabberwrite` (ADR 0010).
fn marker_path(exe: &Path) -> Option<PathBuf> {
    Some(
        exe.parent()?
            .parent()?
            .join("share/jabberwrite/package-format"),
    )
}

fn read_format(marker: &Path) -> Option<PackageFormat> {
    match std::fs::read_to_string(marker).ok()?.trim() {
        "pacman" => Some(PackageFormat::Pacman),
        other => {
            tracing::warn!("unknown package format {other:?} in {}", marker.display());
            None
        }
    }
}

fn package_format() -> Option<PackageFormat> {
    // An AppImage runs from its own mount and always updates itself.
    if !cfg!(target_os = "linux") || std::env::var_os("APPIMAGE").is_some() {
        return None;
    }
    let exe = std::env::current_exe().ok()?;
    read_format(&marker_path(&exe)?)
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
    let mut builder = app.handle.updater_builder();
    if package_format() == Some(PackageFormat::Pacman) {
        builder = builder.target(PACMAN_TARGET);
    }
    builder
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
    let on_chunk = |chunk: usize, total: Option<u64>| {
        downloaded += chunk as u64;
        let percent = total.map(|t| (downloaded * 100 / t.max(1)).min(100));
        if percent != last_percent {
            last_percent = percent;
            if let Some(p) = percent {
                app.set_status(&format!("Downloading update {p}%"));
            }
        }
    };
    let on_finish = || app.set_status("Installing update");
    let result = match package_format() {
        // `download` verifies the signature; the updater cannot install a pacman package itself.
        Some(PackageFormat::Pacman) => match update.download(on_chunk, on_finish).await {
            Ok(bytes) => {
                let version = update.version.clone();
                tauri::async_runtime::spawn_blocking(move || install_pacman(&bytes, &version))
                    .await
                    .map_err(|e| e.to_string())
                    .and_then(|r| r)
            }
            Err(e) => Err(e.to_string()),
        },
        None => update
            .download_and_install(on_chunk, on_finish)
            .await
            .map_err(|e| e.to_string()),
    };

    match result {
        // Windows never gets here: the installer takes over and the app exits.
        Ok(()) => {
            tracing::info!("update installed, restarting");
            app.handle.restart()
        }
        Err(message) => {
            tracing::error!("update failed: {message}");
            app.end_update();
            app.set_status("Update failed");
            set_status(
                app,
                Status::Error {
                    message: message.clone(),
                },
            );
            Err(message)
        }
    }
}

#[cfg(not(unix))]
fn install_pacman(_bytes: &[u8], _version: &str) -> Result<(), String> {
    unreachable!("package_format() is None outside Linux")
}

/// Install a verified Arch package with `pkexec pacman -U`, like the updater does for `.deb`.
#[cfg(unix)]
fn install_pacman(bytes: &[u8], version: &str) -> Result<(), String> {
    use std::os::unix::fs::DirBuilderExt;

    // A fresh directory only we can write to: root installs whatever file lies there.
    let dir = std::env::temp_dir().join(format!("jabberwrite-update-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(&dir)
        .map_err(|e| format!("Could not create {}: {e}", dir.display()))?;
    let package = dir.join(format!("jabberwrite-{version}-x86_64.pkg.tar.zst"));
    let result = std::fs::write(&package, bytes)
        .map_err(|e| format!("Could not write {}: {e}", package.display()))
        .and_then(|()| run_pacman(&package));
    let _ = std::fs::remove_dir_all(&dir);
    result
}

#[cfg(unix)]
fn run_pacman(package: &Path) -> Result<(), String> {
    let status = std::process::Command::new("pkexec")
        .args(["pacman", "-U", "--noconfirm"])
        .arg(package)
        .status()
        .map_err(|e| format!("Could not run pkexec: {e}"))?;
    match status.code() {
        Some(0) => Ok(()),
        // pkexec: 126 = authentication dismissed, 127 = not authorized or no polkit agent.
        Some(126) | Some(127) => {
            Err("Installing the update needs administrator rights, which were not granted.".into())
        }
        _ => Err(format!("pacman could not install the update ({status}).")),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marker_sits_under_the_install_prefix() {
        assert_eq!(
            marker_path(Path::new("/usr/bin/jabberwrite")),
            Some(PathBuf::from("/usr/share/jabberwrite/package-format"))
        );
        assert_eq!(
            marker_path(Path::new("/usr/local/bin/jabberwrite")),
            Some(PathBuf::from("/usr/local/share/jabberwrite/package-format"))
        );
    }

    #[test]
    fn format_comes_from_the_marker() {
        let dir =
            std::env::temp_dir().join(format!("jabberwrite-format-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let marker = dir.join("package-format");

        assert_eq!(read_format(&marker), None);
        std::fs::write(&marker, "pacman\n").unwrap();
        assert_eq!(read_format(&marker), Some(PackageFormat::Pacman));
        std::fs::write(&marker, "rpm\n").unwrap();
        assert_eq!(read_format(&marker), None);

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
