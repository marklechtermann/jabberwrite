---
status: in-progress  # draft | accepted | in-progress | done
adrs: [0001, 0007]
---

# 0007: Paste on Wayland

## Goal

On Linux Wayland sessions (Ubuntu 26.04 has no X11 session anymore), the transcribed text is pasted into the focused window, not only copied to the clipboard.

## Scope

- When `WAYLAND_DISPLAY` is set, send the configured paste shortcut via the XDG RemoteDesktop portal (ADR 0007) instead of enigo.
- Open the portal session lazily on the first paste; keep it open; reopen once if a paste fails.
- Save the restore token in the data directory (`portal-token`) so the permission dialog appears only once.
- README: describe Wayland behavior.

## Out of scope

Global hotkey and overlay on Wayland (see `docs/ideas.md`), compositors without the RemoteDesktop portal, a uinput fallback.

## Tasks

- [x] ADR 0007
- [x] Portal paste in `src-tauri/src/paste.rs`, used from `app.rs` on Wayland
- [x] `ashpd` dependency, `npm run licenses`
- [x] README platform notes
- [ ] Test on Ubuntu 26.04 GNOME Wayland: first paste asks for permission, later pastes and app restarts do not

## Done when

On GNOME Wayland, dictating into a native Wayland app (e.g. GNOME Text Editor, Ptyxis) pastes the text; after allowing once, restarting the app pastes without a dialog. X11, Windows and macOS behave as before.
