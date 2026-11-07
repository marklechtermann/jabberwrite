//! Small always-on-top indicator that must never take focus.

#[cfg(target_os = "linux")]
mod layer;

use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, WebviewUrl, WebviewWindowBuilder};

const LABEL: &str = "overlay";
// Windows enforces a minimum window width (~130 px), which stretched the badge
// into an ellipse. The window is therefore larger than the badge; the badge is
// drawn centered at BADGE size (see overlay.css) on a transparent background.
const SIZE: f64 = 160.0;
const BADGE: f64 = 56.0;
const BOTTOM_MARGIN: f64 = 80.0;

/// What the badge shows. Serialized for the webview as
/// `{ "state": "downloading", "progress": 0.42 }`.
#[derive(Clone, Copy, PartialEq, Debug, Serialize)]
#[serde(tag = "state", content = "progress", rename_all = "lowercase")]
pub enum State {
    /// Model download; progress between 0.0 and 1.0.
    Downloading(f32),
    /// Model is being loaded.
    Loading,
    Recording,
    Processing,
    Error,
}

/// Last state shown, `None` while hidden. The overlay webview reads it on
/// mount, since events emitted before its listener exists are lost.
static CURRENT: Mutex<Option<State>> = Mutex::new(None);

#[tauri::command]
pub fn overlay_state() -> Option<State> {
    *CURRENT.lock().unwrap()
}

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    // Wayland: no webview window, the badge is a layer-shell surface (ADR 0008).
    #[cfg(target_os = "linux")]
    if crate::session::is_wayland() {
        app.manage(layer::start());
        return Ok(());
    }
    WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("overlay.html".into()))
        .title("Jabberwrite")
        .inner_size(SIZE, SIZE)
        .resizable(false)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .focused(false)
        .focusable(false)
        .visible(false)
        .build()?;
    Ok(())
}

/// Show the overlay in `state`.
pub fn show(app: &AppHandle, state: State) {
    *CURRENT.lock().unwrap() = Some(state);
    #[cfg(target_os = "linux")]
    if crate::session::is_wayland() {
        if let Some(handle) = app.try_state::<layer::Handle>() {
            handle.show(state);
        }
        return;
    }
    let _ = app.emit_to(LABEL, "overlay-state", state);
    let Some(window) = app.get_webview_window(LABEL) else {
        return;
    };
    if let Ok(Some(monitor)) = window.primary_monitor() {
        let scale = monitor.scale_factor();
        let (pos, size) = (monitor.position(), monitor.size());
        let win = SIZE * scale;
        let x = pos.x as f64 + (size.width as f64 - win) / 2.0;
        let inset = (SIZE - BADGE) / 2.0;
        let y = pos.y as f64 + size.height as f64 - win - (BOTTOM_MARGIN - inset) * scale;
        let _ = window.set_position(PhysicalPosition::new(x as i32, y as i32));
    }
    let _ = window.show();
    // Must be set after the window exists on screen (GTK panics otherwise).
    let _ = window.set_ignore_cursor_events(true);
}

pub fn hide(app: &AppHandle) {
    *CURRENT.lock().unwrap() = None;
    #[cfg(target_os = "linux")]
    if crate::session::is_wayland() {
        if let Some(handle) = app.try_state::<layer::Handle>() {
            handle.hide();
        }
        return;
    }
    if let Some(window) = app.get_webview_window(LABEL) {
        let _ = window.hide();
    }
}
