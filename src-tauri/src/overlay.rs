//! Small always-on-top indicator that must never take focus.

use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, WebviewUrl, WebviewWindowBuilder};

const LABEL: &str = "overlay";
const SIZE: f64 = 56.0;
const BOTTOM_MARGIN: f64 = 80.0;

pub fn create(app: &AppHandle) -> tauri::Result<()> {
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

/// Show the overlay in `state` ("recording", "processing" or "error").
pub fn show(app: &AppHandle, state: &str) {
    let _ = app.emit_to(LABEL, "overlay-state", state);
    let Some(window) = app.get_webview_window(LABEL) else {
        return;
    };
    if let Ok(Some(monitor)) = window.primary_monitor() {
        let scale = monitor.scale_factor();
        let (pos, size) = (monitor.position(), monitor.size());
        let win = SIZE * scale;
        let x = pos.x as f64 + (size.width as f64 - win) / 2.0;
        let y = pos.y as f64 + size.height as f64 - win - BOTTOM_MARGIN * scale;
        let _ = window.set_position(PhysicalPosition::new(x as i32, y as i32));
    }
    let _ = window.show();
    // Must be set after the window exists on screen (GTK panics otherwise).
    let _ = window.set_ignore_cursor_events(true);
}

pub fn hide(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(LABEL) {
        let _ = window.hide();
    }
}
