---
status: accepted
date: 2026-10-29
---

# 0001: App stack: Rust + Tauri v2 + React

## Context

Jabberwrite is a background dictation tool for Windows, Linux and macOS. It needs a tray icon, a small always-on-top overlay that never takes focus, and a settings window that will grow over time. The core (audio, speech-to-text, hotkey, paste) must be native and fast.

## Decision

- Rust for all application logic.
- Tauri v2 for tray, windows and bundling.
- Frontend: React + TypeScript, built with Vite, managed with npm. No component library, router or state library; plain CSS. Two entry points: `settings.html` and `overlay.html`.
- Standard Tauri layout: frontend in `src/`, Rust in `src-tauri/` with plain modules (`audio`, `stt`, `hotkey`, `paste`, `config`, `overlay`, `tray`), no multi-crate workspace.
- Tray-only app: no Dock icon on macOS (accessory activation policy), no taskbar entry. Closing settings hides the window; quitting is only possible from the tray menu.
- The overlay is a transparent, undecorated, always-on-top, non-focusable, click-through Tauri window. Per-OS fixes (`WS_EX_NOACTIVATE`, non-activating `NSPanel`) are added only where testing shows the window still steals focus.
- Platforms for v1: Windows x64, macOS arm64, Linux x64 on X11. Wayland is out of scope for v1.

## Consequences

- The settings UI can grow easily with web tech.
- The build needs Node/npm in addition to Rust. On Linux the app depends on WebKitGTK.
- A webview overlay can steal focus in some cases; we may need small OS-specific patches.
- Wayland users are not supported until a later decision.
