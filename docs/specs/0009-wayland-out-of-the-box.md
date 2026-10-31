---
status: in-progress  # draft | accepted | in-progress | done
adrs: [0007, 0008]
---

# 0009: Wayland out of the box on Ubuntu

## Goal

The release AppImage starts and works on Ubuntu GNOME Wayland without the user changing system settings or installing packages.

## Scope

- Cursor crash: the AppImage bundles GTK 3.24.33, which cannot load a cursor theme of size 0 and aborts on Wayland as soon as a window needs a cursor (`assertion failed: (display_wayland->cursor_theme_name)`). Profiles can carry `org.gnome.desktop.interface cursor-size=0`, e.g. written by an Xfce session; newer GTK copes with it. After GTK init, Jabberwrite sets `gtk-cursor-theme-size` to 24 when it is 0 or less. The user's settings are not changed.
- README: Wayland notes on the group `input` (log in again, check with `id`, log line `no keyboard access`).

## Out of scope

- Replacing evdev with the GlobalShortcuts portal (would remove the `input` group requirement; ADR 0009). Spec 0010.
- Recording badge on GNOME (no layer-shell, ADR 0008).

## Tasks

- [x] Reproduce the crash with the 0.1.20 AppImage on Ubuntu 26.04 Wayland (VM without GPU)
- [x] Isolate the cause: bundled GTK 3.24.33 plus `cursor-size=0`; system GTK 3.24.52 is not affected
- [x] `session::ensure_cursor_size()` called from `setup` on Linux
- [x] README Wayland notes
- [ ] CI green (fmt, clippy, tests, licenses)
- [ ] Release AppImage verified on the VM: settings window opens, dictation pastes into a native Wayland window

## Done when

The release AppImage opens its settings window and pastes a dictation on the Ubuntu VM whose profile has `cursor-size=0`, with no environment variables or system changes.
