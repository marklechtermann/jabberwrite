---
status: accepted
date: 2026-10-30
---

# 0007: Paste on Wayland via the XDG RemoteDesktop portal

## Context

ADR 0001 left Wayland out of v1. Ubuntu 26.04 no longer ships an X11 session, and users report that on Wayland the text reaches the clipboard but is never pasted. The paste keystroke is synthesized with enigo over XTest, which only reaches XWayland windows; native Wayland apps never see it. Wayland deliberately offers no API for one client to send keys to another.

Options:

- XDG RemoteDesktop portal: supported by GNOME and KDE, no extra software or privileges. The desktop asks the user once; a restore token skips the dialog later.
- `/dev/uinput` virtual keyboard: works on every compositor, but needs the `input` group or a udev rule, which users must set up by hand.
- `wtype`/`ydotool`: external tools; `wtype` does not work on GNOME, `ydotool` needs a daemon and uinput access.
- No auto-paste on Wayland: copy only and ask the user to press Ctrl+V.

## Decision

On Linux, when `WAYLAND_DISPLAY` is set, the paste shortcut is sent through the RemoteDesktop portal (keyboard only) using the `ashpd` crate. One portal session is opened lazily and kept for the app's lifetime; the restore token is saved in the data directory with persist mode "until revoked", so the permission dialog appears only once. If the session fails (e.g. the user stopped it), it is reopened on the next paste. X11 sessions keep using enigo.

## Consequences

- Paste works on GNOME and KDE Wayland without setup beyond one permission dialog.
- While the session is open, the desktop may show a remote-control indicator.
- Compositors without the RemoteDesktop portal (e.g. most wlroots ones) still do not paste; the text stays in the clipboard and the error is shown.
- New dependency `ashpd` (MIT), on Linux only.
- The global hotkey and the overlay on Wayland are not covered by this decision.
