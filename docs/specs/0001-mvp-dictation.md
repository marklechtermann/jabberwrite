---
status: in-progress
adrs: [0001, 0002, 0003, 0004]
---

# 0001: MVP dictation

## Goal

Press a global shortcut, speak, press it again, and the transcribed text is placed in the clipboard and pasted into the focused window. Built for dictating prompts into coding agents and editors on Windows, macOS and Linux (X11).

## Scope

**Flow**

- States: `idle → recording → processing → idle`.
- Shortcut toggles recording. Default: Ctrl+Super (Windows, Linux), Ctrl+Cmd (macOS); modifier-only, fires on release if no other key was pressed meanwhile (ADR 0003).
- Audio from the OS default input device via `cpal`, converted to mono 16 kHz f32.
- On stop: transcribe with Orukeet via sherpa-onnx (ADR 0002).
- Trim leading/trailing whitespace; no other post-processing.
- Put text in the clipboard (it stays there), then simulate the paste shortcut (default Ctrl+V, Cmd+V on macOS; configurable via `paste_shortcut` in `config.json`).
- Hotkey ignored while `processing` and while the model is loading or downloading.
- Recordings shorter than ~0.3 s are discarded. Empty transcription: clipboard unchanged, no paste.

**Model**

- Downloaded on first launch from Hugging Face (`oruk/orukeet`, pinned revision, SHA-256 verified) into the per-user data directory. Progress shown in the tray tooltip.
- `model_path` in `config.json` overrides the location.
- Loaded once at startup and kept in memory.

**Overlay**

- Small transparent, always-on-top, non-focusable, click-through window, bottom-centre of the primary screen.
- Shows a recording state (red dot) and a processing state (spinner); hidden after paste.
- On error, shows an error state for ~2 s.

**Tray**

- Tray icon on all platforms; menu: Settings…, Quit. Tooltip shows status and last error.
- No Dock icon (macOS), no taskbar entry.

**Settings window** (React)

- Shortcut field: click "Record", press the keys.
- Config file path (read-only) with an "Open folder" button.
- Log file path.
- Version number.
- Repository link: https://github.com/marklechtermann/jabberwrite
- Credits: Orukeet and NVIDIA Parakeet, with links and the CC BY-SA 4.0 notice.
- macOS: if Microphone, Input Monitoring or Accessibility permission is missing, open settings on startup with a hint and a button to the matching System Settings pane.
- Closing hides the window.

**Config**

- JSON, per-user config directory via the `directories` crate:
  - Linux `~/.config/jabberwrite/config.json`
  - Windows `%APPDATA%\jabberwrite\config.json`
  - macOS `~/Library/Application Support/jabberwrite/config.json`
- Created with defaults on first launch; read at startup; written when settings are saved. No file watcher.
- Invalid JSON: start with defaults, show the error, never overwrite the file.

**Other**

- Single-instance guard.
- Logging with `tracing` to a rotating file in the data directory.
- Licence Apache-2.0 (ADR 0006, originally dual MIT / Apache-2.0).
- CI, versioning and installers per ADR 0004.

## Out of scope

Wayland, push-to-talk, GPU acceleration, microphone picker, language setting, launch at login, sound cues, recording-length limit, text post-processing, code signing / notarization, auto-updater, macOS onboarding wizard, macOS x64 and Windows arm64 builds. See `docs/ideas/`.

## Tasks

- [x] Scaffold Tauri v2 + React/TS/Vite (standard layout, two entry points), licence files, README
- [x] `config`: load/save JSON, defaults, invalid-file handling
- [x] Logging with `tracing` to rotating file
- [x] Single-instance guard
- [x] Tray icon and menu, tray-only activation policy
- [x] `hotkey`: low-level hook for Windows, macOS, X11; modifier-only release logic
- [x] `audio`: capture default input, resample to mono 16 kHz f32
- [x] `stt`: model download with SHA-256 check and progress; sherpa-onnx load and transcribe
- [x] `paste`: clipboard write and simulated paste shortcut
- [x] State machine wiring incl. short-recording and empty-result handling
- [x] Overlay window: states, non-focusable, click-through; per-OS fixes if needed
- [x] Settings window: shortcut recorder, paths, version, repo link, credits
- [x] macOS permission check and System Settings deep links
- [x] Version injection (`Cargo.toml` → Tauri → frontend), dev version for local builds
- [x] GitHub Actions: PR checks matrix; release on `main` with `tauri-action`

## Done when

- On Windows x64, macOS arm64 and Linux X11, pressing Ctrl+Super / Ctrl+Cmd, speaking and pressing again pastes the text into the focused app (editor and terminal tested), and the text remains in the clipboard.
- The overlay shows recording and processing without taking focus, and hides afterwards.
- Changing the shortcut in settings persists to `config.json` and takes effect immediately.
- A merge to `main` produces a GitHub Release `vX.Y.N` with installers for all three platforms, and the settings dialog shows that version.
