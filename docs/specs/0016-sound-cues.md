---
status: done  # draft | accepted | in-progress | done
adrs: []
---

# 0016: Sound cues

## Goal

Starting and stopping a recording is only visible through the overlay. Add an optional audible cue so the user notices both without looking, off by default.

## Scope

- Config option `sound_cues`, default `false`, with a checkbox "Play a sound when recording starts and stops" in Settings → General.
- A rising two-note chime when a recording starts, a falling one when it stops. Applies to the hotkey and to the test recording in Settings.
- The tones are synthesized and played through `cpal` on the default output device; no audio files, no new dependency.
- The stop cue plays after the microphone is closed, so it is not part of the recording. The start cue plays right after the microphone opens, so no speech is cut off; with speakers it may be picked up faintly.
- Playback failures are logged and never block dictation.
- Existing configs without the field get the new default (`false`).

## Out of scope

Custom sound files, volume control, cues for errors or for the pasted text.

## Tasks

- [x] Config field and default
- [x] Synthesized start/stop cues
- [x] Settings checkbox and backend command
- [x] README

## Done when

- With the option on, starting and stopping a recording each play a short sound.
- With it off (default), nothing is played.
- Verified on Linux; Windows and macOS untested.
