---
status: accepted
date: 2026-10-29
---

# 0003: Global hotkey via our own low-level keyboard hook

## Context

The default shortcut is Ctrl+Super (Ctrl+Cmd on macOS), a modifier-only chord. Standard global-hotkey APIs (`RegisterHotKey`, Tauri's global-shortcut plugin, Carbon, XDG portal) require a non-modifier key, so they cannot express it. Ctrl+Super is also the prefix of real OS shortcuts (e.g. Ctrl+Win+→).

## Decision

- Implement a low-level keyboard hook in Rust for every shortcut, modifier-only or not, so there is one code path:
  - Windows: `SetWindowsHookEx(WH_KEYBOARD_LL)`
  - macOS: `CGEventTap` (requires Input Monitoring / Accessibility permission)
  - Linux X11: XInput2 / XRecord
- A modifier-only chord fires on release, and only if no other key was pressed while it was held.
- Recording is toggle-only (press to start, press again to stop).

## Consequences

- The default shortcut works and does not break other Ctrl+Win shortcuts.
- We maintain three small platform-specific hook implementations.
- macOS users must grant an extra permission.
- This approach does not work on Wayland; Wayland needs a separate decision.
