//! Linux session type detection (ADR 0008).

/// True when the current Linux session runs on Wayland.
#[cfg(target_os = "linux")]
pub fn is_wayland() -> bool {
    std::env::var_os("WAYLAND_DISPLAY").is_some()
        || std::env::var("XDG_SESSION_TYPE").is_ok_and(|t| t.eq_ignore_ascii_case("wayland"))
}

/// The GTK bundled in the AppImage (3.24.33) cannot load a cursor theme of
/// size 0 and then aborts on Wayland as soon as a window needs a cursor.
/// Profiles can carry `cursor-size` 0 (e.g. written by an Xfce session) and
/// newer GTK copes with it, so fall back to GNOME's default of 24.
/// Call after GTK init.
#[cfg(target_os = "linux")]
pub fn ensure_cursor_size() {
    use gtk::glib::prelude::ObjectExt;
    const KEY: &str = "gtk-cursor-theme-size";
    let Some(settings) = gtk::Settings::default() else {
        return;
    };
    if settings.property::<i32>(KEY) <= 0 {
        tracing::info!("cursor size not set; using 24");
        settings.set_property(KEY, 24i32);
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    #[test]
    fn detects_wayland_from_either_variable() {
        // Environment is process-global; keep both checks in one test.
        let (wd, st) = (
            std::env::var_os("WAYLAND_DISPLAY"),
            std::env::var_os("XDG_SESSION_TYPE"),
        );
        std::env::remove_var("WAYLAND_DISPLAY");
        std::env::set_var("XDG_SESSION_TYPE", "x11");
        assert!(!is_wayland());
        std::env::set_var("XDG_SESSION_TYPE", "Wayland");
        assert!(is_wayland());
        std::env::remove_var("XDG_SESSION_TYPE");
        std::env::set_var("WAYLAND_DISPLAY", "wayland-0");
        assert!(is_wayland());
        for (k, v) in [("WAYLAND_DISPLAY", wd), ("XDG_SESSION_TYPE", st)] {
            match v {
                Some(v) => std::env::set_var(k, v),
                None => std::env::remove_var(k),
            }
        }
    }
}
