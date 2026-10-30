---
status: in-progress  # draft | accepted | in-progress | done
adrs: [0007, 0008]
---

# 0008: Wayland hotkey and overlay

## Goal

On Linux Wayland sessions the global hotkey fires while a native Wayland window has focus, and the recording badge is shown where the compositor supports it.

## Scope

- Session detection: Wayland backends are used when `XDG_SESSION_TYPE=wayland` or `WAYLAND_DISPLAY` is set; X11 sessions keep the current code.
- `src-tauri/src/hotkey/evdev.rs`: reads all keyboard devices in `/dev/input`, maps Linux key codes to `keys::Key` (modifiers, Space, Enter, Tab, Escape, Backspace, digits, letters, F1-F12) and calls `dispatch`. Devices are rescanned while running so hot-plugged keyboards work.
- Fallback and errors: no readable keyboard device falls back to the X11 hook if XWayland exists; otherwise the tray status says to add the user to the group `input`.
- `src-tauri/src/overlay.rs` plus `overlay/layer.rs`: badge on a `zwlr_layer_shell_v1` surface (recording, processing, error states as today); no overlay without layer-shell.
- README: Wayland notes for hotkey and overlay.
- `ACKNOWLEDGEMENTS.md` regenerated for the new crates.

## Out of scope

- Paste on Wayland: decided in ADR 0007 (RemoteDesktop portal), implemented on `main`.
- Clipboard backend on Wayland (`arboard` without `wayland-data-control`), see `docs/ideas.md`.
- Compositors without layer-shell (GNOME): no badge, by design.

## Tasks

- [x] ADR 0008
- [x] Wayland session detection and backend selection in `hotkey/mod.rs`
- [x] evdev hotkey backend with key-code mapping and hot-plug
- [x] Layer-shell overlay with software-drawn badge; no overlay without layer-shell (code done; not yet seen on a live compositor)
- [x] Error and fallback status in the tray (existing `set_status` path in `lib.rs` shows the hotkey error)
- [x] README and `ACKNOWLEDGEMENTS.md` updated
- [ ] Smoke test on KDE/Wayland: hotkey fires in a native Wayland window, badge visible, paste still works (spec 0007)

## Done when

On KDE Wayland, the configured shortcut starts and stops dictation while a native Wayland app (e.g. Kate, Konsole) has focus, the badge is visible, and the transcript is pasted. X11, Windows and macOS behave as before.
