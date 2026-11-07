---
status: in-progress  # draft | accepted | in-progress | done
adrs: [0008, 0009]
---

# 0013: Overlay while the model downloads and loads

## Goal

After installation the model is downloaded first, and on every start it takes a while until it is loaded, longer on weak hardware. Today this is only visible in the tray tooltip and in Settings, so the shortcut seems to do nothing. Show the overlay badge during initialization so the user sees what is happening and when Jabberwrite is ready.

## Scope

- Two new overlay states, same 56 px round badge as the recording states, no text:
  - `downloading`: a white download arrow in the center and a calm blue ring that fills clockwise from 0 to 100 % with the real download progress.
  - `loading`: a calm blue ring segment spinning like the recording ring, no microphone, nothing in the center.
- `App::load_model()` drives the overlay:
  - while the model downloads: `downloading` with the current progress (updates in 1 % steps, as the tray status already does)
  - while the model loads (also with a custom `model_path`, and when the model is already on disk): `loading`
  - on success: the overlay hides
  - on failure: the existing `error` badge, hidden after 2 s as today; the error text stays in the tray status
- The badge is shown on every start, also when loading is fast. It stays until loading ends; no auto-hide.
- The shortcut stays ignored while loading (`Phase::Loading`, unchanged). The recording overlays work as before once the model is ready.
- Both renderers get the new states:
  - webview (X11, Windows, macOS): `src/overlay/main.tsx` and `overlay.css`
  - layer-shell (Wayland, ADR 0008/0009): `src-tauri/src/overlay/layer.rs`, including frame-paced redraws for the spinning `loading` ring and a redraw on each progress update
- Startup race in the webview: the overlay state is emitted before the overlay webview has registered its listener, and the webview defaults to `recording`. Rust keeps the current overlay state (state and progress), and the webview reads it on mount through a command before listening for events. The webview no longer defaults to `recording`; it renders nothing until it knows the state.
- `overlay::show` gets a way to pass the progress (for example an optional value or a separate `overlay::progress(p)`); the exact shape is up to the implementation.

## Out of scope

- Text in the overlay (would need a font rasterizer on Wayland).
- A badge on GNOME Wayland, which has no layer-shell (unchanged, ADR 0008).
- Feedback when the shortcut is pressed while loading.
- Changing the download itself (resume, mirrors, retries).

## Tasks

- [x] Store the current overlay state in Rust and add a command the webview reads on mount
- [x] Add `downloading` (with progress) and `loading` to `overlay::show` / `hide`
- [x] Call the overlay from `App::load_model()` for download, load, success and failure
- [x] Webview: render both states (arrow + progress ring, blue spinner); no default `recording` state
- [x] Layer-shell: render both states, animate the `loading` ring, redraw on progress
- [x] CI green
- [ ] Manual test on Linux X11 and Wayland (KDE or sway): delete the model dir, start: arrow with filling ring, then spinner, then the badge disappears and the shortcut records
- [ ] Manual test with the model on disk: spinner is shown, then disappears
- [ ] Manual test of a failed download (for example offline): error badge, error text in the tray
- [ ] Manual test on Windows and macOS if available

## Done when

On a fresh install the badge shows the download progress, then a blue spinner while the model loads, and disappears when Jabberwrite is ready; on every later start the blue spinner is shown until the model is loaded. Recording, processing and error badges work as before.
