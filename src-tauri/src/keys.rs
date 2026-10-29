//! Platform-independent shortcut model and matching logic.
//!
//! Platform hooks translate native key events into [`Key`] and feed them to a
//! [`Matcher`], which decides when the configured shortcut fires (ADR 0003).

use std::collections::HashSet;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    Ctrl,
    Shift,
    Alt,
    Super,
    Char(char),
    F(u8),
    Space,
    Enter,
    Tab,
    Escape,
    Backspace,
    Other(u32),
}

impl Key {
    fn modifier(self) -> Option<u8> {
        match self {
            Key::Ctrl => Some(CTRL),
            Key::Shift => Some(SHIFT),
            Key::Alt => Some(ALT),
            Key::Super => Some(SUPER),
            _ => None,
        }
    }

    fn parse(s: &str) -> Option<Key> {
        let upper = s.to_ascii_uppercase();
        Some(match upper.as_str() {
            "CTRL" | "CONTROL" => Key::Ctrl,
            "SHIFT" => Key::Shift,
            "ALT" | "OPTION" => Key::Alt,
            "SUPER" | "WIN" | "CMD" | "COMMAND" | "META" => Key::Super,
            "SPACE" => Key::Space,
            "ENTER" | "RETURN" => Key::Enter,
            "TAB" => Key::Tab,
            "ESC" | "ESCAPE" => Key::Escape,
            "BACKSPACE" => Key::Backspace,
            _ => {
                let mut chars = upper.chars();
                match (chars.next(), chars.next()) {
                    (Some(c), None) if c.is_ascii_alphanumeric() => Key::Char(c),
                    (Some('F'), Some(_)) => {
                        let n: u8 = upper[1..].parse().ok()?;
                        if (1..=24).contains(&n) {
                            Key::F(n)
                        } else {
                            return None;
                        }
                    }
                    _ => return None,
                }
            }
        })
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Key::Ctrl => f.write_str("Ctrl"),
            Key::Shift => f.write_str("Shift"),
            Key::Alt => f.write_str("Alt"),
            Key::Super if cfg!(target_os = "macos") => f.write_str("Cmd"),
            Key::Super => f.write_str("Super"),
            Key::Char(c) => write!(f, "{c}"),
            Key::F(n) => write!(f, "F{n}"),
            Key::Space => f.write_str("Space"),
            Key::Enter => f.write_str("Enter"),
            Key::Tab => f.write_str("Tab"),
            Key::Escape => f.write_str("Escape"),
            Key::Backspace => f.write_str("Backspace"),
            Key::Other(code) => write!(f, "Key{code}"),
        }
    }
}

const CTRL: u8 = 1;
const SHIFT: u8 = 2;
const ALT: u8 = 4;
const SUPER: u8 = 8;
const MOD_ORDER: [Key; 4] = [Key::Ctrl, Key::Shift, Key::Alt, Key::Super];

/// A set of modifiers plus at most one regular key, e.g. `Ctrl+Super` or `Ctrl+Shift+V`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Shortcut {
    mods: u8,
    key: Option<Key>,
}

impl Shortcut {
    pub fn parse(s: &str) -> Result<Shortcut, String> {
        let mut mods = 0;
        let mut key = None;
        for part in s.split('+').map(str::trim) {
            let k = Key::parse(part).ok_or_else(|| format!("unknown key '{part}' in '{s}'"))?;
            match k.modifier() {
                Some(m) => mods |= m,
                None if key.is_none() => key = Some(k),
                None => return Err(format!("'{s}' has more than one non-modifier key")),
            }
        }
        let shortcut = Shortcut { mods, key };
        if mods == 0 && !matches!(key, Some(Key::F(_))) {
            return Err(format!("'{s}' needs at least one modifier (or an F key)"));
        }
        Ok(shortcut)
    }

    /// Modifiers in canonical order, for synthesizing key presses.
    pub fn modifiers(&self) -> Vec<Key> {
        MOD_ORDER
            .into_iter()
            .filter(|k| self.mods & k.modifier().unwrap() != 0)
            .collect()
    }

    pub fn key(&self) -> Option<Key> {
        self.key
    }
}

impl fmt::Display for Shortcut {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let parts: Vec<String> = self
            .modifiers()
            .into_iter()
            .chain(self.key)
            .map(|k| k.to_string())
            .collect();
        f.write_str(&parts.join("+"))
    }
}

/// Tracks key state and reports when the shortcut fires.
///
/// Modifier-only shortcuts fire on release, and only if no other key was
/// pressed while the modifiers were held. Shortcuts with a regular key fire
/// when that key goes down with exactly the configured modifiers held.
pub struct Matcher {
    shortcut: Shortcut,
    held: HashSet<Key>,
    armed: bool,
    spoiled: bool,
}

impl Matcher {
    pub fn new(shortcut: Shortcut) -> Self {
        Matcher {
            shortcut,
            held: HashSet::new(),
            armed: false,
            spoiled: false,
        }
    }

    pub fn set_shortcut(&mut self, shortcut: Shortcut) {
        self.shortcut = shortcut;
        self.armed = false;
        self.spoiled = false;
    }

    fn held_mods(&self) -> u8 {
        self.held
            .iter()
            .filter_map(|k| k.modifier())
            .fold(0, |a, m| a | m)
    }

    /// Feed one key event; returns true when the shortcut fires.
    pub fn on_key(&mut self, key: Key, pressed: bool) -> bool {
        let repeat = pressed && self.held.contains(&key);
        if pressed {
            self.held.insert(key);
        } else {
            self.held.remove(&key);
        }
        if repeat {
            return false;
        }

        let fired = match self.shortcut.key {
            Some(target) => pressed && key == target && self.held_mods() == self.shortcut.mods,
            None => self.on_modifier_only(key, pressed),
        };

        if self.held.is_empty() {
            self.armed = false;
            self.spoiled = false;
        }
        fired
    }

    fn on_modifier_only(&mut self, key: Key, pressed: bool) -> bool {
        if pressed {
            if key.modifier().is_none() || self.held_mods() & !self.shortcut.mods != 0 {
                self.spoiled = true;
            } else if self.held_mods() == self.shortcut.mods {
                self.armed = true;
            }
            false
        } else {
            let fire = self.armed && !self.spoiled;
            if fire {
                // Releasing the remaining modifiers must not fire again.
                self.spoiled = true;
            }
            fire
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Feeds every key (no short-circuiting) and reports whether any fired.
    fn feed(m: &mut Matcher, keys: &[Key], down: bool) -> bool {
        let mut fired = false;
        for k in keys {
            fired |= m.on_key(*k, down);
        }
        fired
    }

    fn press(m: &mut Matcher, keys: &[Key]) -> bool {
        feed(m, keys, true)
    }

    fn release(m: &mut Matcher, keys: &[Key]) -> bool {
        feed(m, keys, false)
    }

    #[test]
    fn parse_and_display() {
        let s = Shortcut::parse("ctrl+win").unwrap();
        assert_eq!(s, Shortcut::parse("Ctrl+Super").unwrap());
        assert_eq!(s, Shortcut::parse("Cmd + Control").unwrap());
        assert_eq!(
            Shortcut::parse("Shift+Ctrl+v").unwrap().to_string(),
            "Ctrl+Shift+V"
        );
        assert!(Shortcut::parse("F9").is_ok());
        assert!(Shortcut::parse("A").is_err());
        assert!(Shortcut::parse("Ctrl+A+B").is_err());
        assert!(Shortcut::parse("Ctrl+Nope").is_err());
        assert!(Shortcut::parse("F25").is_err());
    }

    #[test]
    fn modifier_only_fires_on_release() {
        let mut m = Matcher::new(Shortcut::parse("Ctrl+Super").unwrap());
        assert!(!press(&mut m, &[Key::Ctrl, Key::Super]));
        assert!(m.on_key(Key::Super, false));
        assert!(!m.on_key(Key::Ctrl, false));
        // works again afterwards
        press(&mut m, &[Key::Super, Key::Ctrl]);
        assert!(release(&mut m, &[Key::Ctrl, Key::Super]));
    }

    #[test]
    fn modifier_only_spoiled_by_other_key() {
        let mut m = Matcher::new(Shortcut::parse("Ctrl+Super").unwrap());
        press(&mut m, &[Key::Ctrl, Key::Super, Key::Other(1)]);
        assert!(!release(&mut m, &[Key::Other(1), Key::Super, Key::Ctrl]));
        let mut m = Matcher::new(Shortcut::parse("Ctrl+Super").unwrap());
        press(&mut m, &[Key::Ctrl, Key::Shift, Key::Super]);
        assert!(!release(&mut m, &[Key::Super, Key::Shift, Key::Ctrl]));
    }

    #[test]
    fn modifier_only_needs_all_mods() {
        let mut m = Matcher::new(Shortcut::parse("Ctrl+Super").unwrap());
        press(&mut m, &[Key::Ctrl]);
        assert!(!release(&mut m, &[Key::Ctrl]));
    }

    #[test]
    fn key_chord_fires_on_press_without_repeat() {
        let mut m = Matcher::new(Shortcut::parse("Ctrl+Alt+Space").unwrap());
        press(&mut m, &[Key::Ctrl, Key::Alt]);
        assert!(m.on_key(Key::Space, true));
        assert!(!m.on_key(Key::Space, true)); // auto-repeat
        release(&mut m, &[Key::Space]);
        assert!(m.on_key(Key::Space, true));
        // extra modifier does not match
        let mut m = Matcher::new(Shortcut::parse("Ctrl+Space").unwrap());
        press(&mut m, &[Key::Ctrl, Key::Shift]);
        assert!(!m.on_key(Key::Space, true));
    }
}
