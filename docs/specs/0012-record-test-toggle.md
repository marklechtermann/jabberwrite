---
status: in-progress  # draft | accepted | in-progress | done
adrs: []
---

# 0012: Test recording as click-to-toggle

## Goal

The test recording in Settings → General must be usable. Today it is a hold button (spec 0011), and on at least Linux the recording ends right after the press, so nothing can be tested. Holding a button while speaking is also awkward. Make it a test environment that works like the global shortcut: click to start, click to stop, text shown below.

## Scope

- Settings → General → "Test recording": one button that toggles.
  - Idle: label "Start recording". Click starts recording.
  - Recording: label "Stop recording", visibly active. Click stops recording.
  - Processing: button disabled, "Recognizing…" next to it.
- Nothing ends the recording except a second click on the button. Pointer release, pointer capture, button blur and key release no longer stop it.
- Keyboard: Space and Enter on the focused button click it once (native button behavior, no custom key handlers).
- Result below the button, as in spec 0011:
  - text recognized: shown as text, also added to the history, not pasted and not copied
  - too short or empty: "Nothing recognized."
  - error (for example silent microphone): shown as error
- A new click on the button clears the previous result and starts the next recording.
- Leaving the General topic (or closing the window) while recording stops the recording and discards the result, so the microphone is never left open.
- Section text is updated: "Click to start recording, click again to stop. The text appears here and in the history; it is not pasted."
- Backend commands `record_start` and `record_stop` stay as they are.

## Out of scope

- Push-to-talk for the global shortcut (separate idea).
- Showing the result of a recording that was stopped via the global shortcut while the test recording was running. If the shortcut stops it, the button reports "Nothing recognized.".
- A recording length limit (separate idea).

## Tasks

- [x] Cause of the immediate stop: not reproduced (no GUI in the implementing session). Suspects were `onLostPointerCapture`, `onBlur` and pointer events in the Linux webview; all of them are gone with the toggle
- [x] Replace the hold handlers in `RecordButton` with a single `onClick` toggle and the new labels
- [x] Stop a running test recording when the component unmounts
- [x] Update the section text
- [ ] CI green
- [ ] Manual test on Linux (X11 and Wayland): click, speak, click shows the text; click twice quickly shows nothing recognized; switching to History while recording stops the microphone
- [ ] Manual test on Windows and macOS if available

## Done when

Clicking "Start recording", speaking and clicking "Stop recording" shows the recognized text below the button and adds it to the history, without pasting, and the recording does not end by itself in between.
