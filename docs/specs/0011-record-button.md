---
status: in-progress  # draft | accepted | in-progress | done
adrs: []
---

# 0011: Record button in settings

## Goal

Test recording and recognition without the global shortcut, e.g. while the hotkey does not work on a new platform.

## Scope

- Settings → General: a "Hold to record" button. Pressing it starts recording, releasing it (or leaving the window with the button held) stops.
- The recognized text is shown below the button and added to the history like any dictation. It is not pasted and not copied: focus is on the settings window, and on Wayland pasting would trigger the portal dialog.
- Backend commands `record_start` and `record_stop`; `record_stop` returns the text once recognition is done. The hotkey toggle uses the same start/stop logic and still pastes.

## Out of scope

- Push-to-talk for the global shortcut (separate idea).

## Tasks

- [x] `App::start` / `App::stop` with a paste flag; `toggle` built on them
- [x] Commands `record_start`, `record_stop`
- [x] Button and result in the settings window
- [ ] CI green
- [ ] Manual test: hold, speak, release shows the text; short tap shows nothing recognized

## Done when

Holding the button, speaking and releasing shows the recognized text in the settings window and adds it to the history, without pasting.
