---
status: in-progress  # draft | accepted | in-progress | done
adrs: [0007, 0009]
---

# 0010: Wayland hotkey via GlobalShortcuts portal

## Goal

On GNOME and KDE Wayland the global hotkey works without adding the user to the group `input` and logging in again.

## Scope

- `src-tauri/src/hotkey/portal.rs`: write the hidden `.desktop` file, register the app ID, create a GlobalShortcuts session, bind one shortcut and toggle on `Activated`, ignoring key repeat.
- Preferred trigger from the configured shortcut; `CTRL+LOGO+space` when it is modifier-only.
- `hotkey::start` on Wayland: evdev, then portal, then X11 via XWayland.
- README Wayland notes; ADR 0008 set to superseded by 0009.

## Out of scope

- Changing the portal trigger from Jabberwrite's settings (the desktop owns it; interface version 1 cannot reopen the dialog).
- Showing the bound trigger in the settings window.
- Compositors without the portal (sway and other wlroots): they keep evdev or XWayland.

## Tasks

- [x] Verify the portal on Ubuntu 26.04 GNOME 50 (app ID requirement, no modifier-only triggers, dialog once, key repeat)
- [x] ADR 0009
- [x] `hotkey/portal.rs` with unit tests for trigger conversion, key repeat and `.desktop` quoting
- [x] Fallback order in `hotkey::start`
- [x] README
- [x] CI green (fmt, clippy, tests, licenses)
- [ ] Manual test on Ubuntu 26.04 Wayland without the group `input`: confirm dialog on first start, no dialog on second start, toggling works in a native Wayland window, holding the shortcut toggles once
- [ ] Manual test on KDE Plasma Wayland

## Done when

On Ubuntu GNOME Wayland, with the user not in the group `input`, the AppImage starts, asks once to confirm `Ctrl+Super+Space`, and toggles dictation with it in native Wayland windows.
