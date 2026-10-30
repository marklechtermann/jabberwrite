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

/// Paste on Wayland through the XDG RemoteDesktop portal (ADR 0007).
/// Synthetic X11 input only reaches XWayland windows there.
#[cfg(target_os = "linux")]
pub mod wayland {
    use std::path::Path;
    use std::sync::Mutex;

    use ashpd::desktop::remote_desktop::{
        DeviceType, KeyState, RemoteDesktop, SelectDevicesOptions,
    };
    use ashpd::desktop::{PersistMode, Session};

    use crate::keys::{Key, Shortcut};

    struct Portal {
        proxy: RemoteDesktop,
        session: Session<RemoteDesktop>,
    }

    static PORTAL: Mutex<Option<Portal>> = Mutex::new(None);

    pub fn active() -> bool {
        std::env::var_os("WAYLAND_DISPLAY").is_some_and(|v| !v.is_empty())
    }

    /// Send the paste shortcut. Blocks; do not call on the main thread, the
    /// first call may wait for the user to answer the permission dialog.
    pub fn send(shortcut: &Shortcut, token_file: &Path) -> Result<(), String> {
        let mut mods = Vec::new();
        for m in shortcut.modifiers() {
            mods.push(keysym(m)?);
        }
        let key = shortcut.key().map(keysym).transpose()?;
        let mut portal = PORTAL.lock().unwrap();
        tauri::async_runtime::block_on(async {
            if let Some(p) = portal.take() {
                match press(&p, &mods, key).await {
                    Ok(()) => {
                        *portal = Some(p);
                        return Ok(());
                    }
                    Err(e) => {
                        tracing::warn!("portal session failed, reopening: {e}");
                        let _ = p.session.close().await;
                    }
                }
            }
            let p = open(token_file).await?;
            let result = press(&p, &mods, key).await;
            *portal = Some(p);
            result
        })
        .map_err(|e| format!("paste (Wayland portal): {e}"))
    }

    async fn open(token_file: &Path) -> Result<Portal, String> {
        let err = |e: ashpd::Error| e.to_string();
        let token = std::fs::read_to_string(token_file).ok();
        let proxy = RemoteDesktop::new().await.map_err(err)?;
        let session = proxy
            .create_session(Default::default())
            .await
            .map_err(err)?;
        proxy
            .select_devices(
                &session,
                SelectDevicesOptions::default()
                    .set_devices(Some(DeviceType::Keyboard.into()))
                    .set_persist_mode(PersistMode::ExplicitlyRevoked)
                    .set_restore_token(token.as_deref().map(str::trim)),
            )
            .await
            .and_then(|r| r.response())
            .map_err(err)?;
        let started = proxy
            .start(&session, None, Default::default())
            .await
            .and_then(|r| r.response())
            .map_err(err)?;
        if !started.devices().contains(DeviceType::Keyboard) {
            let _ = session.close().await;
            return Err("keyboard access was not granted".into());
        }
        if let Some(token) = started.restore_token() {
            if let Err(e) = std::fs::write(token_file, token) {
                tracing::warn!("cannot save portal token: {e}");
            }
        }
        tracing::info!("RemoteDesktop portal session started");
        Ok(Portal { proxy, session })
    }

    async fn press(p: &Portal, mods: &[i32], key: Option<i32>) -> Result<(), String> {
        let notify = |sym: i32, state: KeyState| {
            p.proxy
                .notify_keyboard_keysym(&p.session, sym, state, Default::default())
        };
        let mut result = Ok(());
        for &m in mods {
            result = notify(m, KeyState::Pressed).await;
            if result.is_err() {
                break;
            }
        }
        if let (Ok(()), Some(k)) = (&result, key) {
            result = match notify(k, KeyState::Pressed).await {
                Ok(()) => notify(k, KeyState::Released).await,
                e => e,
            };
        }
        for &m in mods.iter().rev() {
            let _ = notify(m, KeyState::Released).await;
        }
        result.map_err(|e| e.to_string())
    }

    /// X11 keysyms; Latin-1 keysyms equal their code point.
    fn keysym(key: Key) -> Result<i32, String> {
        Ok(match key {
            Key::Ctrl => 0xffe3,
            Key::Shift => 0xffe1,
            Key::Alt => 0xffe9,
            Key::Super => 0xffeb,
            Key::Char(c) if c.is_ascii() => c.to_ascii_lowercase() as i32,
            Key::Space => 0x20,
            Key::Enter => 0xff0d,
            Key::Tab => 0xff09,
            Key::Escape => 0xff1b,
            Key::Backspace => 0xff08,
            other => return Err(format!("unsupported paste key {other}")),
        })
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn paste_shortcut_keysyms() {
            let s = Shortcut::parse("Ctrl+Shift+V").unwrap();
            let mods: Vec<i32> = s
                .modifiers()
                .into_iter()
                .map(|k| keysym(k).unwrap())
                .collect();
            assert_eq!(mods, [0xffe3, 0xffe1]);
            assert_eq!(s.key().map(keysym), Some(Ok(0x76)));
        }
    }
}
