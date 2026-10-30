//! Linux session type detection (ADR 0008).

/// True when the current Linux session runs on Wayland.
#[cfg(target_os = "linux")]
pub fn is_wayland() -> bool {
    std::env::var_os("WAYLAND_DISPLAY").is_some()
        || std::env::var("XDG_SESSION_TYPE").is_ok_and(|t| t.eq_ignore_ascii_case("wayland"))
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
