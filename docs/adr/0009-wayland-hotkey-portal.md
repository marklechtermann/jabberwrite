---
status: accepted
date: 2026-10-31
---

# 0009: Wayland hotkey via GlobalShortcuts portal as fallback to evdev

Supersedes ADR 0008. Its overlay decision is carried over unchanged below; only the hotkey part changes.

## Context

ADR 0008 reads the hotkey from `/dev/input` with evdev. That needs membership in the group `input`, which users must set up by hand and which only applies after logging in again. Without it, the shortcut only fires while an XWayland window has focus.

ADR 0008 left the XDG GlobalShortcuts portal open until it was verified. Tested on Ubuntu 26.04 (GNOME 50.1, xdg-desktop-portal 1.21.1, xdg-desktop-portal-gnome 50.0):

- The portal is available (interface version 1).
- Host apps need an app ID: without one the portal answers `NotAllowed: An app id is required`. `org.freedesktop.host.portal.Registry.Register` provides it, but only for an ID with an installed `.desktop` file (`App info not found` otherwise).
- On the first bind the desktop shows a dialog where the user confirms or changes the trigger. Later binds return the stored trigger without a dialog.
- Modifier-only triggers such as `Ctrl+Super` cannot be set. A trigger needs a non-modifier key.
- `Activated` arrives on press. Holding the keys repeats `Activated` (after the key repeat delay, then every 30 ms), and `Deactivated` is sometimes missing after a hold.

KDE Plasma implements the portal since 5.27 (not tested here). wlroots compositors such as sway do not.

## Decision

On Wayland the hotkey uses the first backend that works:

1. evdev, as in ADR 0008. Users in the group `input` keep modifier-only shortcuts such as `Ctrl+Super`.
2. The GlobalShortcuts portal. Jabberwrite writes `~/.local/share/applications/io.github.marklechtermann.jabberwrite.desktop` (hidden from menus, `Exec` pointing to the AppImage or binary), registers that app ID and binds one shortcut. The preferred trigger is the configured shortcut; if it is modifier-only, `CTRL+LOGO+space` is offered instead. The shortcut toggles on `Activated`; an `Activated` within 600 ms of the previous one without a `Deactivated` in between counts as key repeat.
3. The X11 hook via XWayland, if `DISPLAY` is set.

Otherwise the tray shows the error.

Overlay (unchanged from ADR 0008): the recording badge is a `zwlr_layer_shell_v1` surface drawn with `smithay-client-toolkit`. Without layer-shell there is no badge.

## Consequences

- On GNOME and KDE the hotkey works without the group `input` and without logging in again, after confirming the shortcut once.
- In portal mode the trigger is owned by the desktop. Changing the shortcut in Jabberwrite's settings has no effect there; it is changed in the desktop's settings. Version 1 of the interface has no way to reopen the dialog.
- In portal mode the RemoteDesktop paste portal (ADR 0007) also runs under the registered app ID, so a permission granted earlier without an app ID may be asked once more.
- A key repeat delay above 600 ms makes a long hold count as a second press.
- New dependencies: the `global_shortcuts` feature of `ashpd`, and `futures-util` (already in the dependency tree).
