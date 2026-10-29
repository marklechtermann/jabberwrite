---
status: accepted
date: 2026-10-29
---

# 0005: In-app updates via the Tauri updater and GitHub Releases

## Context

Users install Jabberwrite from GitHub Releases (ADR 0004) and have no way to learn about or install newer versions. Windows is the main target. The hard part of self-updating is that a running app cannot replace its own files.

Alternatives considered:

- **Notify only** (call the GitHub API, open the release page): no signing needed, but the user still downloads and runs the installer by hand.
- **Own updater** (GitHub API, download the installer, verify a SHA-256, run it): more code to write and secure than the first-party plugin, and a checksum fetched from the same release protects nothing if the release is compromised.
- **`tauri-plugin-updater`** with a static `latest.json` on GitHub Releases: first-party, handles download, signature check, install and relaunch per platform.

## Decision

Use `tauri-plugin-updater`, fed by `https://github.com/marklechtermann/jabberwrite/releases/latest/download/latest.json`. The repository is public, so no token is needed and GitHub API rate limits do not apply.

**Signing.** The updater refuses unsigned updates; this cannot be disabled. A minisign key pair is generated once with `npm run tauri signer generate`. The public key lives in `tauri.conf.json`, the private key (and optional password) in the GitHub secrets `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`. If the private key is lost, installed apps can never update again and users must reinstall by hand. This is separate from OS code signing (still an idea); a compromised GitHub account alone cannot push a malicious update without the key.

**Self-update safety.**

- Windows: the updater downloads the installer, verifies the signature, launches it and calls `process::exit(0)` itself, so the app is gone before any file is replaced. The NSIS installer then restarts the app. An `on_before_exit` hook hides the tray icon, because the exit skips Tauri's cleanup.
- Linux and macOS: the updater replaces the AppImage or `.app` in place; the app then calls `AppHandle::restart()`. The single-instance lock is released by Tauri's exit cleanup before the new process starts.
- An update is refused while recording or processing. While one runs, the hotkey is ignored. A model download in progress may be interrupted; it restarts on the next launch.

**Windows installer.** Only NSIS, installed per user (`installMode: currentUser`), so updates never need admin rights. The MSI target is dropped (it is per machine and would raise a UAC prompt on every update). The update runs in `passive` mode: a small progress window, no interaction.

**Checks happen on request only**, never in the background: the app stays offline unless the user clicks "Check for Updates…" in the tray menu (native dialogs) or "Check for updates" in the Settings window. The Settings window shows the result and an "Install and restart" button. Dev builds (`-dev` version) refuse to check, because they would see every release as newer.

**CI.**

- `createUpdaterArtifacts` is enabled only for releases, through `src-tauri/tauri.release.conf.json`, so local `tauri build` works without the private key.
- `tauri-action` writes `latest.json`. All matrix jobs read-modify-write the same asset and can overwrite each other, so `publish-release` verifies that `latest.json` lists every platform before it undrafts the release.
- The release job fails early when the public key is still the placeholder or the secret is missing.

## Consequences

- One-time setup by the maintainer: generate the key pair, put the public key into `tauri.conf.json`, add the secrets. Until then releases fail by design.
- Versions without the updater (up to 0.1.x of this first release) must be updated by hand once.
- The old MSI users must switch to the NSIS installer once.
- macOS: builds are unsigned (ad-hoc signed), so macOS ties the Microphone, Input Monitoring and Accessibility grants to the exact binary. After an update the user may have to grant them again; the Settings window opens and says so (existing behaviour). Code signing (see `docs/ideas/code-signing.md`) fixes this.
- Linux: `.deb` installs update through `pkexec` where the updater supports it; AppImage updates in place. A `.deb` user without a polkit agent gets an error and can install by hand.
- Antivirus software may object to an unsigned installer launched from a temp directory. Windows SmartScreen does not apply: the app downloads the file itself, so it carries no Mark-of-the-Web.
- Adds `tauri-plugin-updater` and `tauri-plugin-dialog` (and their dependency trees) to the app.
- Amends ADR 0004: its "no auto-updater" and `.msi` statements no longer hold; the rest stands.
