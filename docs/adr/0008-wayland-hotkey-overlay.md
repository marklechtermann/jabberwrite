---
status: superseded by 0009
date: 2026-10-30
---

# 0008: Wayland hotkey and overlay via evdev and layer-shell

## Context

ADR 0001 left Wayland out of v1. ADR 0007 covers paste on Wayland through the RemoteDesktop portal. It does not cover the global hotkey or the recording overlay. The X11 keyboard hook only sees keys of XWayland windows, so on Wayland the shortcut does not fire while a native window has focus. A Tauri webview window cannot be placed as a non-focusable, always-on-top badge on wlroots compositors; the layer-shell protocol exists for that.

Options for the hotkey:

- XDG GlobalShortcuts portal: no extra privileges. Compositor support varies and was not verified for the targets here, so it cannot be relied on yet.
- evdev: reads keyboard devices in `/dev/input` directly. Works on every compositor, but needs membership in the group `input`, which users set up by hand.

Options for the overlay:

- layer-shell (`zwlr_layer_shell_v1`): supported by KDE, sway and other wlroots compositors. GNOME does not implement it.
- No overlay on Wayland: the state is only visible in the tray.

## Decision

When `session::is_wayland()` holds (`WAYLAND_DISPLAY` set or `XDG_SESSION_TYPE=wayland`):

- The hotkey is read from all keyboard devices in `/dev/input` with the `evdev` crate, rescanned every 2 s so hot-plugged keyboards work. If no device is readable and `DISPLAY` is set, the X11 hook (via XWayland) is used instead. Otherwise the tray status names the group `input`.
- The recording badge is a `zwlr_layer_shell_v1` surface drawn with `smithay-client-toolkit`. Without layer-shell there is no badge.

X11 sessions keep the existing code paths.

## Consequences

- Any member of `input` can read all keyboards. The app matches the shortcut only and does not store or send keystrokes; the README says so.
- The hotkey works on GNOME Wayland, but GNOME shows no badge.
- Shortcut letters follow US key positions, so other layouts match the physical key.
- If the GlobalShortcuts portal is verified on the target compositors, a later ADR may replace evdev and remove the `input` group requirement.
- New dependencies, Linux only: `evdev` and `smithay-client-toolkit` (license checked via `npm run licenses`).
- Paste on Wayland is decided in ADR 0007 and not changed here.
