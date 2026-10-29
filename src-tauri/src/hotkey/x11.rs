//! X11: XInput2 raw key events on the root window.

use std::collections::HashMap;
use std::sync::mpsc;

use x11rb::connection::Connection;
use x11rb::protocol::xinput::{self, ConnectionExt as _};
use x11rb::protocol::xproto::ConnectionExt as _;
use x11rb::protocol::Event;

use super::dispatch;
use crate::keys::Key;

pub fn run() -> Result<(), String> {
    let (tx, rx) = mpsc::channel();
    std::thread::Builder::new()
        .name("hotkey-x11".into())
        .spawn(move || {
            let conn = match setup() {
                Ok(c) => {
                    let _ = tx.send(Ok(()));
                    c
                }
                Err(e) => {
                    let _ = tx.send(Err(e));
                    return;
                }
            };
            let (conn, keymap) = conn;
            loop {
                match conn.wait_for_event() {
                    Ok(Event::XinputRawKeyPress(ev)) => dispatch(map(&keymap, ev.detail), true),
                    Ok(Event::XinputRawKeyRelease(ev)) => dispatch(map(&keymap, ev.detail), false),
                    Ok(_) => {}
                    Err(e) => {
                        tracing::error!("X11 connection lost: {e}");
                        return;
                    }
                }
            }
        })
        .map_err(|e| e.to_string())?;
    rx.recv().map_err(|e| e.to_string())?
}

type Keymap = HashMap<u32, Key>;

fn setup() -> Result<(x11rb::rust_connection::RustConnection, Keymap), String> {
    let err = |e: &dyn std::fmt::Display| format!("X11 hotkey setup failed: {e}");
    let (conn, screen) = x11rb::connect(None).map_err(|e| err(&e))?;
    let root = conn.setup().roots[screen].root;

    conn.xinput_xi_query_version(2, 0)
        .map_err(|e| err(&e))?
        .reply()
        .map_err(|e| err(&e))?;
    let mask = xinput::XIEventMask::RAW_KEY_PRESS | xinput::XIEventMask::RAW_KEY_RELEASE;
    conn.xinput_xi_select_events(
        root,
        &[xinput::EventMask {
            deviceid: 1, // XIAllMasterDevices
            mask: vec![mask],
        }],
    )
    .map_err(|e| err(&e))?
    .check()
    .map_err(|e| err(&e))?;

    let keymap = keymap(&conn).map_err(|e| err(&e))?;
    Ok((conn, keymap))
}

/// Map keycodes to keys via the first keysym of each keycode.
fn keymap(conn: &impl Connection) -> Result<Keymap, Box<dyn std::error::Error>> {
    let setup = conn.setup();
    let (min, max) = (setup.min_keycode, setup.max_keycode);
    let reply = conn.get_keyboard_mapping(min, max - min + 1)?.reply()?;
    let per = reply.keysyms_per_keycode as usize;
    let mut map = HashMap::new();
    for (i, syms) in reply.keysyms.chunks(per.max(1)).enumerate() {
        if let Some(key) = syms.first().and_then(|s| keysym_to_key(*s)) {
            map.insert(min as u32 + i as u32, key);
        }
    }
    Ok(map)
}

fn map(keymap: &Keymap, code: u32) -> Key {
    keymap.get(&code).copied().unwrap_or(Key::Other(code))
}

fn keysym_to_key(sym: u32) -> Option<Key> {
    Some(match sym {
        0xffe3 | 0xffe4 => Key::Ctrl,
        0xffe1 | 0xffe2 => Key::Shift,
        0xffe9 | 0xffea => Key::Alt,
        0xffeb | 0xffec => Key::Super,
        0x20 => Key::Space,
        0xff0d => Key::Enter,
        0xff09 => Key::Tab,
        0xff1b => Key::Escape,
        0xff08 => Key::Backspace,
        0x30..=0x39 => Key::Char(char::from_u32(sym)?),
        0x61..=0x7a => Key::Char(char::from_u32(sym)?.to_ascii_uppercase()),
        0xffbe..=0xffd5 => Key::F((sym - 0xffbe + 1) as u8),
        _ => return None,
    })
}
