//! macOS: listen-only CGEventTap. Needs the Input Monitoring permission;
//! retries until it is granted.

use std::ffi::c_void;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::time::Duration;

use super::dispatch;
use crate::keys::Key;

type CFMachPortRef = *mut c_void;
type CFRunLoopSourceRef = *mut c_void;
type CFRunLoopRef = *mut c_void;
type CGEventRef = *mut c_void;
type TapCallback = extern "C" fn(*mut c_void, u32, CGEventRef, *mut c_void) -> CGEventRef;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn CGEventTapCreate(
        tap: u32,
        place: u32,
        options: u32,
        events_of_interest: u64,
        callback: TapCallback,
        user_info: *mut c_void,
    ) -> CFMachPortRef;
    fn CGEventTapEnable(tap: CFMachPortRef, enable: bool);
    fn CGEventGetIntegerValueField(event: CGEventRef, field: u32) -> i64;
    fn CGEventGetFlags(event: CGEventRef) -> u64;
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    static kCFRunLoopCommonModes: *const c_void;
    fn CFMachPortCreateRunLoopSource(
        alloc: *const c_void,
        port: CFMachPortRef,
        order: isize,
    ) -> CFRunLoopSourceRef;
    fn CFRunLoopGetCurrent() -> CFRunLoopRef;
    fn CFRunLoopAddSource(rl: CFRunLoopRef, source: CFRunLoopSourceRef, mode: *const c_void);
    fn CFRunLoopRun();
}

const SESSION_TAP: u32 = 1;
const HEAD_INSERT: u32 = 0;
const LISTEN_ONLY: u32 = 1;
const KEY_DOWN: u32 = 10;
const KEY_UP: u32 = 11;
const FLAGS_CHANGED: u32 = 12;
const TAP_DISABLED_BY_TIMEOUT: u32 = 0xFFFF_FFFE;
const KEYCODE_FIELD: u32 = 9;
const SOURCE_PID_FIELD: u32 = 41;

static TAP: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());

pub fn run() -> Result<(), String> {
    std::thread::Builder::new()
        .name("hotkey-macos".into())
        .spawn(|| unsafe {
            let mask = (1u64 << KEY_DOWN) | (1u64 << KEY_UP) | (1u64 << FLAGS_CHANGED);
            let mut warned = false;
            let tap = loop {
                let tap = CGEventTapCreate(
                    SESSION_TAP,
                    HEAD_INSERT,
                    LISTEN_ONLY,
                    mask,
                    callback,
                    std::ptr::null_mut(),
                );
                if !tap.is_null() {
                    break tap;
                }
                if !warned {
                    tracing::warn!(
                        "event tap unavailable; waiting for Input Monitoring permission"
                    );
                    warned = true;
                }
                std::thread::sleep(Duration::from_secs(2));
            };
            TAP.store(tap, Ordering::Relaxed);
            let source = CFMachPortCreateRunLoopSource(std::ptr::null(), tap, 0);
            CFRunLoopAddSource(CFRunLoopGetCurrent(), source, kCFRunLoopCommonModes);
            CGEventTapEnable(tap, true);
            tracing::info!("event tap installed");
            CFRunLoopRun();
        })
        .map(|_| ())
        .map_err(|e| e.to_string())
}

extern "C" fn callback(_: *mut c_void, kind: u32, event: CGEventRef, _: *mut c_void) -> CGEventRef {
    unsafe {
        if kind == TAP_DISABLED_BY_TIMEOUT {
            CGEventTapEnable(TAP.load(Ordering::Relaxed), true);
            return event;
        }
        // Ignore our own synthesized paste keystrokes.
        if CGEventGetIntegerValueField(event, SOURCE_PID_FIELD) == std::process::id() as i64 {
            return event;
        }
        let code = CGEventGetIntegerValueField(event, KEYCODE_FIELD) as u32;
        let key = map(code);
        match kind {
            KEY_DOWN => dispatch(key, true),
            KEY_UP => dispatch(key, false),
            FLAGS_CHANGED => {
                let flag = match key {
                    Key::Shift => 0x2_0000,
                    Key::Ctrl => 0x4_0000,
                    Key::Alt => 0x8_0000,
                    Key::Super => 0x10_0000,
                    _ => return event,
                };
                dispatch(key, CGEventGetFlags(event) & flag != 0);
            }
            _ => {}
        }
    }
    event
}

fn map(code: u32) -> Key {
    const LETTERS: [(u32, char); 36] = [
        (0x00, 'A'),
        (0x0B, 'B'),
        (0x08, 'C'),
        (0x02, 'D'),
        (0x0E, 'E'),
        (0x03, 'F'),
        (0x05, 'G'),
        (0x04, 'H'),
        (0x22, 'I'),
        (0x26, 'J'),
        (0x28, 'K'),
        (0x25, 'L'),
        (0x2E, 'M'),
        (0x2D, 'N'),
        (0x1F, 'O'),
        (0x23, 'P'),
        (0x0C, 'Q'),
        (0x0F, 'R'),
        (0x01, 'S'),
        (0x11, 'T'),
        (0x20, 'U'),
        (0x09, 'V'),
        (0x0D, 'W'),
        (0x07, 'X'),
        (0x10, 'Y'),
        (0x06, 'Z'),
        (0x1D, '0'),
        (0x12, '1'),
        (0x13, '2'),
        (0x14, '3'),
        (0x15, '4'),
        (0x17, '5'),
        (0x16, '6'),
        (0x1A, '7'),
        (0x1C, '8'),
        (0x19, '9'),
    ];
    const FKEYS: [u32; 12] = [
        0x7A, 0x78, 0x63, 0x76, 0x60, 0x61, 0x62, 0x64, 0x65, 0x6D, 0x67, 0x6F,
    ];
    match code {
        0x3B | 0x3E => Key::Ctrl,
        0x38 | 0x3C => Key::Shift,
        0x3A | 0x3D => Key::Alt,
        0x37 | 0x36 => Key::Super,
        0x31 => Key::Space,
        0x24 => Key::Enter,
        0x30 => Key::Tab,
        0x35 => Key::Escape,
        0x33 => Key::Backspace,
        _ => {
            if let Some((_, c)) = LETTERS.iter().find(|(k, _)| *k == code) {
                Key::Char(*c)
            } else if let Some(i) = FKEYS.iter().position(|k| *k == code) {
                Key::F(i as u8 + 1)
            } else {
                Key::Other(code)
            }
        }
    }
}
