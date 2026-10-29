---
status: draft  # draft | accepted | in-progress | done
adrs: []       # e.g. [0001]
---

# 0003: Launch at login

## Goal

Jabberwrite lives in the tray and is only useful while running. Let the user choose that it starts automatically when they log in, so dictation is available without launching the app by hand.

## Scope

- A "Start jabberwrite at login" checkbox in the settings window. Default: off.
- Implemented with the official Tauri autostart plugin (`tauri-plugin-autostart`), which registers the app with the OS:
  - Windows: `Run` registry key under `HKCU`
  - macOS: LaunchAgent
  - Linux: `~/.config/autostart/*.desktop`
- The OS entry is the single source of truth. The setting is not stored in `config.json`; the checkbox reads the current state from the plugin and writes it back on toggle.
- When started by autostart, the app runs as usual (tray only, no settings window opened). Opening settings on a missing macOS permission (spec 0001) still applies.
- Errors from enabling/disabling (e.g. read-only autostart directory) are shown next to the checkbox and logged; the checkbox reverts to the actual state.

## Out of scope

Delayed start, start minimised/visible variants, per-machine (all users) autostart, a CLI flag to toggle it. The `--autostart` argument is not needed unless testing shows the app must distinguish autostart from manual launch.

## Tasks

- [ ] Add `tauri-plugin-autostart` and its capability permissions
- [ ] Backend commands to read and set the autostart state
- [ ] Settings window: checkbox with error display
- [ ] Verify the generated entry points to the installed binary on each OS (Linux AppImage path needs checking)
- [ ] Update README / settings docs if they list features

## Done when

- Enabling the checkbox creates the OS autostart entry; after logging out and in, jabberwrite is running in the tray without a visible window.
- Disabling it removes the entry and the app no longer starts at login.
- The checkbox reflects the real OS state after an app restart, also if the entry was removed manually.
- Verified on Windows x64, macOS arm64 and Linux X11.
