//! Clipboard write plus a synthesized paste keystroke.

use enigo::{Direction, Enigo, Keyboard, Settings};

use crate::keys::{Key, Shortcut};

/// Keeps the clipboard handle alive: on X11 the text is only available
/// while its owner exists.
pub struct Clipboard(arboard::Clipboard);

impl Clipboard {
    pub fn new() -> Result<Clipboard, String> {
        arboard::Clipboard::new()
            .map(Clipboard)
            .map_err(|e| format!("clipboard: {e}"))
    }

    pub fn set(&mut self, text: &str) -> Result<(), String> {
        self.0.set_text(text).map_err(|e| format!("clipboard: {e}"))
    }
}

/// Send the paste shortcut to the focused window. Run on the main thread.
pub fn send(shortcut: &Shortcut) -> Result<(), String> {
    let err = |e: &dyn std::fmt::Display| format!("paste: {e}");
    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| err(&e))?;
    let mods: Vec<enigo::Key> = shortcut
        .modifiers()
        .into_iter()
        .map(to_enigo)
        .collect::<Result<_, _>>()?;
    let key = shortcut.key().map(to_enigo).transpose()?;
    for m in &mods {
        enigo.key(*m, Direction::Press).map_err(|e| err(&e))?;
    }
    let result = key.map(|k| enigo.key(k, Direction::Click)).transpose();
    for m in mods.iter().rev() {
        let _ = enigo.key(*m, Direction::Release);
    }
    result.map(|_| ()).map_err(|e| err(&e))
}

fn to_enigo(key: Key) -> Result<enigo::Key, String> {
    use enigo::Key as E;
    Ok(match key {
        Key::Ctrl => E::Control,
        Key::Shift => E::Shift,
        Key::Alt => E::Alt,
        Key::Super => E::Meta,
        Key::Char(c) => E::Unicode(c.to_ascii_lowercase()),
        Key::Space => E::Space,
        Key::Enter => E::Return,
        Key::Tab => E::Tab,
        Key::Escape => E::Escape,
        Key::Backspace => E::Backspace,
        other => return Err(format!("unsupported paste key {other}")),
    })
}
