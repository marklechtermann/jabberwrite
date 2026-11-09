---
status: done  # draft | accepted | in-progress | done
adrs: []
---

# 0014: Clipboard setting

## Goal

Dictation currently always leaves the recognized text in the clipboard. Make that optional, off by default, so the user's clipboard is not overwritten.

## Scope

- Config option `copy_to_clipboard`, default `false`, with a checkbox "Keep recognized text in the clipboard" in Settings → General.
- Pasting still goes through the clipboard (copy, then paste shortcut). With the option off, the previous clipboard text is read first and restored shortly after the paste. With it on, the text stays as before.
- Existing configs without the field get the new default (`false`).

## Out of scope

Restoring non-text clipboard content (images, files). Typing text without the clipboard. The history Copy button always copies.

## Tasks

- [x] Config field and default
- [x] Restore previous clipboard text after paste when disabled
- [x] Settings checkbox and backend command
- [x] README

## Done when

- By default, the clipboard holds its previous text after dictation.
- With the option on, the clipboard holds the recognized text.
- Verified on Linux; Windows and macOS untested.
