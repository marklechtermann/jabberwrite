//! Wayland: global shortcut through the XDG GlobalShortcuts portal (ADR 0009).
//!
//! Used when evdev has no keyboard access. The portal needs an app ID, which a
//! host app only gets by registering one that has a `.desktop` file, so we
//! write that file first. The desktop asks the user once to confirm the
//! shortcut and remembers it. GNOME rejects modifier-only triggers, so a
//! modifier-only config shortcut is offered as `CTRL+LOGO+space` instead.

use std::path::PathBuf;
use std::time::Duration;

use ashpd::desktop::global_shortcuts::{BindShortcutsOptions, GlobalShortcuts, NewShortcut};
use futures_util::StreamExt;

type Session = ashpd::desktop::Session<GlobalShortcuts>;

use crate::keys::{Key, Shortcut};

const APP_ID: &str = "io.github.marklechtermann.jabberwrite";
const SHORTCUT_ID: &str = "dictate";
const FALLBACK_TRIGGER: &str = "CTRL+LOGO+space";
/// Holding the shortcut repeats `Activated` (after the key repeat delay,
/// 500 ms on GNOME) and may never send `Deactivated`.
const REPEAT_WINDOW: Duration = Duration::from_millis(600);

/// Set up the portal session. Returns an error if the portal is unusable, so
/// the caller can fall back. Binding happens in the background because the
/// first bind waits for the user to confirm the shortcut.
pub fn run(shortcut: Shortcut) -> Result<(), String> {
    write_desktop_file()?;
    let (proxy, session) = tauri::async_runtime::block_on(open())?;
    tauri::async_runtime::spawn(async move {
        if let Err(e) = listen(&proxy, &session, &trigger(shortcut)).await {
            tracing::error!("global shortcut portal: {e}");
        }
    });
    Ok(())
}

async fn open() -> Result<(GlobalShortcuts, Session), String> {
    let err = |e: ashpd::Error| format!("global shortcut portal: {e}");
    let app_id = ashpd::AppID::try_from(APP_ID).map_err(err)?;
    // The portal may not have picked up a .desktop file we just wrote.
    let mut tries = 0;
    while let Err(e) = ashpd::register_host_app(app_id.clone()).await {
        tries += 1;
        if tries == 5 {
            return Err(err(e));
        }
        std::thread::sleep(Duration::from_millis(300));
    }
    let proxy = GlobalShortcuts::new().await.map_err(err)?;
    let session = proxy
        .create_session(Default::default())
        .await
        .map_err(err)?;
    Ok((proxy, session))
}

async fn listen(proxy: &GlobalShortcuts, session: &Session, trigger: &str) -> Result<(), String> {
    let err = |e: ashpd::Error| e.to_string();
    // Subscribe before binding so no activation is missed.
    let activated = proxy.receive_activated().await.map_err(err)?;
    let deactivated = proxy.receive_deactivated().await.map_err(err)?;
    let new = NewShortcut::new(SHORTCUT_ID, "Start or stop dictation").preferred_trigger(trigger);
    let bound = proxy
        .bind_shortcuts(session, &[new], None, BindShortcutsOptions::default())
        .await
        .and_then(|r| r.response())
        .map_err(|e| format!("shortcut not confirmed: {e}"))?;
    match bound.shortcuts().iter().find(|s| s.id() == SHORTCUT_ID) {
        Some(s) => tracing::info!("global shortcut bound: {}", s.trigger_description()),
        None => return Err("shortcut not confirmed".into()),
    }

    let events = futures_util::stream::select(
        activated
            .filter(|a| std::future::ready(a.shortcut_id() == SHORTCUT_ID))
            .map(|a| (true, a.timestamp())),
        deactivated
            .filter(|d| std::future::ready(d.shortcut_id() == SHORTCUT_ID))
            .map(|d| (false, d.timestamp())),
    );
    let mut events = std::pin::pin!(events);
    let mut repeats = Repeats::default();
    while let Some((active, at)) = events.next().await {
        if active {
            if repeats.press(at) {
                super::fire();
            }
        } else {
            repeats.release();
        }
    }
    Err("portal signal stream ended".into())
}

/// Tells a real press from key repeat.
#[derive(Default)]
struct Repeats {
    last: Option<Duration>,
    released: bool,
}

impl Repeats {
    fn press(&mut self, at: Duration) -> bool {
        let repeat = self
            .last
            .is_some_and(|l| at.saturating_sub(l) <= REPEAT_WINDOW);
        let new = self.released || !repeat;
        self.last = Some(at);
        self.released = false;
        new
    }

    fn release(&mut self) {
        self.released = true;
    }
}

/// The config shortcut as an XDG "shortcuts" trigger, or the fallback if it
/// has no key or the key cannot be expressed.
fn trigger(shortcut: Shortcut) -> String {
    let Some(key) = shortcut.key().and_then(key_name) else {
        return FALLBACK_TRIGGER.into();
    };
    let mut parts: Vec<&str> = shortcut
        .modifiers()
        .into_iter()
        .filter_map(|m| match m {
            Key::Ctrl => Some("CTRL"),
            Key::Shift => Some("SHIFT"),
            Key::Alt => Some("ALT"),
            Key::Super => Some("LOGO"),
            _ => None,
        })
        .collect();
    parts.push(&key);
    parts.join("+")
}

/// XKB keysym name of a non-modifier key.
fn key_name(key: Key) -> Option<String> {
    Some(match key {
        Key::Char(c) => c.to_ascii_lowercase().to_string(),
        Key::F(n) => format!("F{n}"),
        Key::Space => "space".into(),
        Key::Enter => "Return".into(),
        Key::Tab => "Tab".into(),
        Key::Escape => "Escape".into(),
        Key::Backspace => "BackSpace".into(),
        _ => return None,
    })
}

/// `~/.local/share/applications/<app id>.desktop`, pointing to the running
/// AppImage or binary. Hidden from menus; the .deb ships its own entry.
fn write_desktop_file() -> Result<(), String> {
    let base = directories::BaseDirs::new().ok_or("no home directory")?;
    let dir = base.data_dir().join("applications");
    let exe = std::env::var_os("APPIMAGE")
        .map(PathBuf::from)
        .or_else(|| std::env::current_exe().ok())
        .ok_or("cannot find own executable")?;
    let content = desktop_entry(&exe.to_string_lossy());
    let path = dir.join(format!("{APP_ID}.desktop"));
    if std::fs::read_to_string(&path).is_ok_and(|c| c == content) {
        return Ok(());
    }
    std::fs::create_dir_all(&dir)
        .and_then(|()| std::fs::write(&path, content))
        .map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    tracing::info!("wrote {}", path.display());
    Ok(())
}

fn desktop_entry(exe: &str) -> String {
    // Desktop entry spec: quote the path, escape `"`, `` ` ``, `$` and `\`,
    // then escape `\` again for the string value.
    let mut quoted = String::new();
    for c in exe.chars() {
        match c {
            '"' | '`' | '$' => quoted.push_str(&format!("\\\\{c}")),
            '\\' => quoted.push_str("\\\\\\\\"),
            c => quoted.push(c),
        }
    }
    format!(
        "[Desktop Entry]\nType=Application\nName=Jabberwrite\n\
         Comment=Voice-to-text dictation for agentic coding\n\
         Exec=\"{quoted}\"\nTerminal=false\nNoDisplay=true\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    #[test]
    fn triggers() {
        let t = |s: &str| trigger(Shortcut::parse(s).unwrap());
        assert_eq!(t("Ctrl+Super"), "CTRL+LOGO+space");
        assert_eq!(t("Ctrl+Shift+Space"), "CTRL+SHIFT+space");
        assert_eq!(t("Alt+F9"), "ALT+F9");
        assert_eq!(t("Ctrl+Super+D"), "CTRL+LOGO+d");
    }

    #[test]
    fn key_repeat_is_not_a_press() {
        let mut r = Repeats::default();
        // Tap, release, tap.
        assert!(r.press(ms(0)));
        r.release();
        assert!(r.press(ms(300)));
        r.release();
        // Hold: repeat after 500 ms, then every 30 ms, no release.
        assert!(r.press(ms(2000)));
        assert!(!r.press(ms(2500)));
        assert!(!r.press(ms(2530)));
        assert!(!r.press(ms(2560)));
        // Next press after the hold, still without a release.
        assert!(r.press(ms(3400)));
    }

    #[test]
    fn desktop_entry_quotes_exec() {
        let e = desktop_entry("/home/a b/Jabber$write.AppImage");
        assert!(e.contains("Exec=\"/home/a b/Jabber\\\\$write.AppImage\"\n"));
    }
}
