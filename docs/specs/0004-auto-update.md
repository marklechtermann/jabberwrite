---
status: in-progress
adrs: [0004, 0005]
---

# 0004: Auto-update

## Goal

Users learn about new GitHub releases and install them from inside the app, without downloading an installer by hand. Windows is the priority; Linux and macOS use the same mechanism.

## Scope

**Checking**

- On request only, never in the background (ADR 0005).
- Tray menu item "Check for Updates…": checks, then shows a native dialog: "up to date", "Jabberwrite X is available. Install now?" (Install / Later), or the error.
- Settings window, section "Updates": "Check for updates" button, result text, and an "Install and restart" button when a newer version is known. Tray and Settings share one status, so both reflect the same result.
- Dev builds refuse to check.

**Installing**

- Refused while recording or processing; the hotkey is ignored while an update runs.
- Download progress is shown in the status line (tray tooltip and Settings).
- Windows: the updater launches the NSIS installer in passive mode and exits the app; the installer restarts it. Linux and macOS: replace in place, then restart.
- Signature verification failures abort the update and show the error.

**Release pipeline**

- Minisign key pair; public key in `tauri.conf.json`, private key in GitHub secrets.
- Release builds produce signed updater artifacts and `latest.json`; the release is only published when `latest.json` covers all platforms.
- Windows installer: NSIS only, per user. MSI is dropped.

## Out of scope

Background or periodic checks, a config option for them, release notes in the dialog, update channels, rollback, delta updates, code signing / notarization.

## Tasks

- [x] ADR 0005
- [x] `update` module: check, install, shared status, phase guard, `on_before_exit` hook
- [x] Tray item "Check for Updates…" with native dialogs
- [x] Settings "Updates" section
- [x] `tauri.conf.json`: updater endpoint and public key, NSIS per user, drop MSI
- [x] Release workflow: signed artifacts, `latest.json`, completeness check before publishing
- [x] README: updating and release setup
- [ ] Maintainer: generate the key pair, set the public key in `tauri.conf.json`, add the GitHub secrets
- [ ] Verify the first real update on Windows (0.1.N → 0.1.N+1), including tray icon and restart

## Done when

- With the key set up, a merge to `main` publishes a release containing `latest.json` and signed installers.
- An installed build on Windows offers the newer release from the tray menu and from Settings, installs it, exits, and comes back running the new version (shown in Settings).
- Starting an update while recording does nothing harmful; a tampered download is rejected.
