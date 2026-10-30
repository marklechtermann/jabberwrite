//! Wayland: keyboard events read straight from `/dev/input` (ADR 0008).
//!
//! Wayland offers no global key hook, so we read every keyboard device the
//! user can access. Events go through the same `Matcher` as the X11 and
//! low-level hooks. Keys are mapped by Linux key code, which is the physical
//! US position, so letters follow the physical key, not the active layout.

use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use evdev::{Device, EventType, KeyCode};

use super::dispatch;
use crate::keys::Key;

const RESCAN: Duration = Duration::from_secs(2);

/// Device paths already examined or being read, so each is opened once.
type Known = Arc<Mutex<HashSet<PathBuf>>>;

pub fn run() -> Result<(), String> {
    let known: Known = Arc::default();
    let scan = attach_new(&known);
    if scan.keyboards == 0 {
        return Err(if scan.denied > 0 {
            "no keyboard access: add your user to the group 'input' and log in again".into()
        } else {
            "no keyboard found in /dev/input".into()
        });
    }
    std::thread::Builder::new()
        .name("hotkey-evdev-scan".into())
        .spawn(move || loop {
            std::thread::sleep(RESCAN);
            attach_new(&known);
        })
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[derive(Default)]
struct Scan {
    keyboards: usize,
    denied: usize,
}

/// Open every event device not yet known and start a reader for each keyboard.
fn attach_new(known: &Known) -> Scan {
    let mut scan = Scan::default();
    let Ok(entries) = fs::read_dir("/dev/input") else {
        return scan;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path
            .file_name()
            .is_some_and(|n| n.to_string_lossy().starts_with("event"))
        {
            continue;
        }
        if known.lock().unwrap().contains(&path) {
            continue;
        }
        let dev = match Device::open(&path) {
            Ok(dev) => dev,
            Err(e) if e.kind() == io::ErrorKind::PermissionDenied => {
                // Not recorded as known, so it is retried after group changes.
                scan.denied += 1;
                continue;
            }
            Err(_) => continue,
        };
        known.lock().unwrap().insert(path.clone());
        if !is_keyboard(&dev) {
            continue;
        }
        let known = known.clone();
        let spawned = std::thread::Builder::new()
            .name("hotkey-evdev".into())
            .spawn(move || read_device(dev, path, known));
        if spawned.is_ok() {
            scan.keyboards += 1;
        }
    }
    scan
}

fn is_keyboard(dev: &Device) -> bool {
    dev.supported_keys()
        .is_some_and(|keys| keys.contains(KeyCode::KEY_A) && keys.contains(KeyCode::KEY_ENTER))
}

fn read_device(mut dev: Device, path: PathBuf, known: Known) {
    loop {
        let events = match dev.fetch_events() {
            Ok(events) => events,
            Err(e) => {
                tracing::info!("keyboard {} stopped: {e}", path.display());
                known.lock().unwrap().remove(&path);
                return;
            }
        };
        for ev in events {
            if ev.event_type() != EventType::KEY {
                continue;
            }
            // value: 0 = release, 1 = press, 2 = autorepeat (ignored, the matcher tracks held keys).
            let pressed = match ev.value() {
                0 => false,
                1 => true,
                _ => continue,
            };
            dispatch(map(ev.code()), pressed);
        }
    }
}

/// Linux key code (linux/input-event-codes.h, US layout positions) to `Key`.
/// Unknown codes map to `Key::Other`, so any other key still breaks a
/// modifier-only chord.
pub(crate) fn map(code: u16) -> Key {
    match code {
        29 | 97 => Key::Ctrl,
        42 | 54 => Key::Shift,
        56 | 100 => Key::Alt,
        125 | 126 => Key::Super,
        57 => Key::Space,
        28 | 96 => Key::Enter,
        15 => Key::Tab,
        1 => Key::Escape,
        14 => Key::Backspace,
        2..=10 => Key::Char(char::from(b'1' + (code - 2) as u8)),
        11 => Key::Char('0'),
        59..=68 => Key::F((code - 58) as u8),
        87 => Key::F(11),
        88 => Key::F(12),
        _ => match LETTERS.iter().find(|(c, _)| *c == code) {
            Some((_, letter)) => Key::Char(*letter),
            None => Key::Other(u32::from(code)),
        },
    }
}

const LETTERS: [(u16, char); 26] = [
    (16, 'Q'), (17, 'W'), (18, 'E'), (19, 'R'), (20, 'T'), (21, 'Y'), (22, 'U'), (23, 'I'), (24, 'O'), (25, 'P'),
    (30, 'A'), (31, 'S'), (32, 'D'), (33, 'F'), (34, 'G'), (35, 'H'), (36, 'J'), (37, 'K'), (38, 'L'),
    (44, 'Z'), (45, 'X'), (46, 'C'), (47, 'V'), (48, 'B'), (49, 'N'), (50, 'M'),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::{Matcher, Shortcut};

    #[test]
    fn maps_physical_keys() {
        assert_eq!(map(29), Key::Ctrl);
        assert_eq!(map(97), Key::Ctrl);
        assert_eq!(map(126), Key::Super);
        assert_eq!(map(47), Key::Char('V'));
        assert_eq!(map(2), Key::Char('1'));
        assert_eq!(map(11), Key::Char('0'));
        assert_eq!(map(59), Key::F(1));
        assert_eq!(map(88), Key::F(12));
        assert_eq!(map(1), Key::Escape);
        assert_eq!(map(999), Key::Other(999));
    }

    #[test]
    fn ctrl_super_fires_on_release_from_device_events() {
        // Press Ctrl, press Super, release Super: the modifier-only chord fires.
        let mut m = Matcher::new(Shortcut::parse("Ctrl+Super").unwrap());
        let fired: Vec<bool> = [(29, 1), (125, 1), (125, 0)]
            .iter()
            .map(|(c, v)| m.on_key(map(*c), *v == 1))
            .collect();
        assert_eq!(fired, [false, false, true]);
    }

    #[test]
    fn other_key_spoils_modifier_chord() {
        let mut m = Matcher::new(Shortcut::parse("Ctrl+Super").unwrap());
        let fired = [(29, 1), (125, 1), (30, 1), (30, 0), (125, 0), (29, 0)]
            .iter()
            .any(|(c, v)| m.on_key(map(*c), *v == 1));
        assert!(!fired);
    }
}
