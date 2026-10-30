# Ideas

Inbox of raw ideas that are not specs yet. One `##` section per idea, a few sentences each, no ceremony. When an idea becomes a spec, remove its section.

## Code signing and notarization

Sign Windows installers and sign/notarize macOS builds to avoid SmartScreen/Gatekeeper warnings and smooth macOS permission prompts.

## GPU acceleration

Faster transcription via GPU (Metal, CUDA, Vulkan), e.g. through NeMo-Speech.cpp and the GGUF model, or an ONNX Runtime GPU provider. Relevant if CPU latency on Windows/Linux is too slow.

## Language setting

Optional language hint for transcription, only if accuracy on mixed or non-English speech turns out to need it.

## macOS onboarding

A first-run wizard that walks the user through granting Microphone, Input Monitoring and Accessibility permissions.

## Microphone picker

Let the user choose the input device in settings instead of always using the OS default.

## More build targets

Add macOS x64 and Windows arm64 (note: NeMo-Speech.cpp has no Windows arm64 SDK; check sherpa-onnx/ONNX Runtime support).

## Paste shortcut in settings UI

Expose the `paste_shortcut` config option in the settings dialog, e.g. for terminals that need Ctrl+Shift+V.

## Push-to-talk mode

Record while the shortcut is held, in addition to toggle mode. Configurable per user.

## Recording length limit

Stop recording automatically after a configurable maximum duration to avoid runaway recordings.

## Sound cues

Short sounds on recording start and stop.

## Text post-processing

Optional processing of the transcript: trailing space, filler-word removal, custom replacements, or LLM cleanup.

## Wayland support

Full support for Linux Wayland sessions. Paste is covered by spec 0007; still needed: an alternative for the global hotkey (evdev with `input` group or XDG GlobalShortcuts portal) and the overlay (layer-shell). Out of v1 per ADR 0001.
