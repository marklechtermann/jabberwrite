//! Global hotkey via our own low-level keyboard hook (ADR 0003).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

use crate::keys::{Key, Matcher, Shortcut};

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

/// Called by the platform hook for every key event. Must return quickly.
fn dispatch(key: Key, pressed: bool) {
    let Some(hook) = HOOK.get() else { return };
    let fired = hook.matcher.lock().unwrap().on_key(key, pressed);
    if fired && !hook.paused.load(Ordering::Relaxed) {
        (hook.on_fire)();
    }
}
