---
status: in-progress  # draft | accepted | in-progress | done
adrs: []
---

# 0015: Copy last text

## Goal

Sometimes the text is pasted into the wrong window. Let the user copy the most recent recognized text to the clipboard again with one click on the tray icon.

## Scope

- Tray menu item "Copy Last Text" (above "Settings…"). It copies the newest history entry to the clipboard.
- If there is no entry (nothing recognized yet, or history cleared), the error badge is shown.

## Out of scope

Choosing older entries (Settings → History has Copy buttons). A success notification.

## Tasks

- [x] `History::last`
- [x] `App::copy_last_text`
- [x] Tray menu item
- [x] README

## Done when

- After a dictation, tray → "Copy Last Text" puts exactly that text in the clipboard.
