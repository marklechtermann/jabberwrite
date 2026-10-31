//! Global hotkey via our own low-level keyboard hook (ADR 0003).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

use crate::keys::{Key, Matcher, Shortcut};

#[cfg(target_os = "linux")]
pub(crate) mod evdev;
#[cfg(target_os = "linux")]
mod portal;
#[cfg(target_os = "linux")]
mod x11;
#[cfg(target_os = "linux")]
use x11 as platform;

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
use windows as platform;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
use macos as platform;

struct Hook {
    matcher: Mutex<Matcher>,
    paused: AtomicBool,
    on_fire: Box<dyn Fn() + Send + Sync>,
}

static HOOK: OnceLock<Hook> = OnceLock::new();

/// Install the keyboard hook on a background thread. Call once.
pub fn start(shortcut: Shortcut, on_fire: impl Fn() + Send + Sync + 'static) -> Result<(), String> {
    let hook = Hook {
        matcher: Mutex::new(Matcher::new(shortcut)),
        paused: AtomicBool::new(false),
        on_fire: Box::new(on_fire),
    };
    HOOK.set(hook)
        .map_err(|_| "hotkey already started".to_string())?;
    // Wayland order (ADR 0009): evdev, GlobalShortcuts portal, X11 via XWayland.
    #[cfg(target_os = "linux")]
    if crate::session::is_wayland() {
        let Err(evdev_err) = evdev::run() else {
            return Ok(());
        };
        tracing::warn!("{evdev_err}; trying the global shortcut portal");
        let Err(portal_err) = portal::run(shortcut) else {
            return Ok(());
        };
        if std::env::var_os("DISPLAY").is_none() {
            return Err(format!("{evdev_err}; {portal_err}"));
        }
        tracing::warn!("{portal_err}; using X11 via XWayland instead");
    }
    platform::run()
}

pub fn set_shortcut(shortcut: Shortcut) {
    if let Some(hook) = HOOK.get() {
        hook.matcher.lock().unwrap().set_shortcut(shortcut);
    }
}

/// Suppress firing, e.g. while the settings window records a new shortcut.
pub fn set_paused(paused: bool) {
    if let Some(hook) = HOOK.get() {
        hook.paused.store(paused, Ordering::Relaxed);
    }
}

/// Forget held keys when the platform hook may have missed key-ups
/// (e.g. while the Windows lock screen was active).
#[cfg(target_os = "windows")]
fn reset() {
    if let Some(hook) = HOOK.get() {
        hook.matcher.lock().unwrap().reset();
    }
}

/// Called by the platform hook for every key event. Must return quickly.
fn dispatch(key: Key, pressed: bool) {
    let Some(hook) = HOOK.get() else { return };
    if hook.matcher.lock().unwrap().on_key(key, pressed) {
        fire();
    }
}

/// Run the callback unless paused. The portal calls this directly: the
/// desktop does the matching there.
fn fire() {
    let Some(hook) = HOOK.get() else { return };
    if !hook.paused.load(Ordering::Relaxed) {
        (hook.on_fire)();
    }
}
