//! macOS privacy permissions needed for recording, the hotkey and pasting.

use serde::Serialize;

#[derive(Clone, Serialize)]
pub struct Permissions {
    pub microphone: bool,
    pub input_monitoring: bool,
    pub accessibility: bool,
}

impl Permissions {
    pub fn all_granted(&self) -> bool {
        self.microphone && self.input_monitoring && self.accessibility
    }
}

/// System Settings deep link for a permission kind.
pub fn settings_url(kind: &str) -> Option<&'static str> {
    Some(match kind {
        "microphone" => {
            "x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone"
        }
        "input_monitoring" => {
            "x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent"
        }
        "accessibility" => {
            "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility"
        }
        _ => return None,
    })
}

#[cfg(target_os = "macos")]
pub fn check() -> Option<Permissions> {
    use objc2::runtime::AnyObject;
    use objc2::{class, msg_send};

    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn CGPreflightListenEventAccess() -> bool;
        fn AXIsProcessTrusted() -> bool;
    }
    #[link(name = "AVFoundation", kind = "framework")]
    extern "C" {
        static AVMediaTypeAudio: &'static AnyObject;
    }

    // 0 = not determined (macOS asks on first recording), 3 = authorized.
    let mic: isize = unsafe {
        msg_send![class!(AVCaptureDevice), authorizationStatusForMediaType: AVMediaTypeAudio]
    };
    unsafe {
        Some(Permissions {
            microphone: mic == 0 || mic == 3,
            input_monitoring: CGPreflightListenEventAccess(),
            accessibility: AXIsProcessTrusted(),
        })
    }
}

#[cfg(not(target_os = "macos"))]
pub fn check() -> Option<Permissions> {
    None
}
