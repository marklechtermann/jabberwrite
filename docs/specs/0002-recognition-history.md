---
status: in-progress
adrs: []
---

# 0002: Recognition history

## Goal

Keep a continuous log of everything that was recognized, so text can be copied again when pasting failed.

## Scope

- Every non-empty transcription is appended to `history.jsonl` in the per-user data directory (next to `logs/` and `models/`), before it is pasted. Format: JSON Lines, one `{"time": "<RFC 3339, UTC>", "text": "..."}` object per line.
- Settings window, section **History**:
  - Recognized texts listed newest first (last 200), each with date/time and a **Copy** button that puts the text in the clipboard. New entries appear live.
  - **Clear history** button (truncates the file).
  - Checkbox **Clear history when Jabberwrite quits**, stored as `clear_history_on_exit` in `config.json` (default `false`).
  - Path of the history file with an "Open folder" button.

## Out of scope

Size limit or rotation of the file, search, per-entry deletion, export. Clearing on forced termination (SIGKILL, crash, power loss).

## Tasks

- [x] `history`: append, list, clear (JSON Lines)
- [x] Log in the transcribe step, emit a `history` event
- [x] `clear_history_on_exit` config option, cleared on app exit
- [x] Settings UI: list with Copy, Clear, checkbox, file path

## Done when

- Dictating text adds an entry to `history.jsonl` and to the list in the settings window, even if the paste fails.
- Copy puts the entry's text in the clipboard.
- Clear history empties the list and the file; with the checkbox set, the file is empty after the next quit.
