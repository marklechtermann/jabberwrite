---
status: accepted
date: 2026-11-13
---

# 0010: Arch package on GitHub Releases, updated in-app through pacman

## Context

On Arch Linux neither Linux release fits well: the AppImage lives outside pacman, and the `.deb` cannot be installed or updated because there is no `dpkg`. Arch users want what `.deb` users get: download one package, install it, then update from within the app (ADR 0005). The package should not go to a public registry such as the AUR.

The Tauri updater installs AppImage, `.deb` and `.rpm`, but not pacman packages. It does allow a custom `latest.json` target and can download and verify an update without installing it.

## Decision

**Package.** The release workflow builds `Jabberwrite_<version>_x86_64.pkg.tar.zst` from the release `.deb` with `packaging/arch/PKGBUILD` (no rebuild), signs it with the updater key and attaches it to the GitHub release. Users install it with `sudo pacman -U <file>`.

**Marker.** The package ships `/usr/share/jabberwrite/package-format` containing `pacman`. The app looks for it relative to its executable (`<prefix>/share/…` for `<prefix>/bin/jabberwrite`), and ignores it inside an AppImage.

**In-app update.** With the marker, the updater checks the `latest.json` target `linux-x86_64-pacman`, which the workflow adds after all builds finished. The app downloads the package through the updater, which verifies the signature, writes it to a fresh `0700` temp directory and runs `pkexec pacman -U --noconfirm`, the same way the updater installs a `.deb`. Then it restarts. `publish-release` requires the new target in `latest.json`, like the others.

## Consequences

- Arch users update like everyone else: tray → Check for Updates…, and confirm the polkit prompt. This needs a polkit agent (KDE, GNOME and most desktops have one); the package depends on `polkit`.
- Versions are installed by pacman, so `pacman -Q jabberwrite` and file ownership stay correct.
- The package is unsigned for pacman (`pacman -U` of a local file accepts that by default); integrity on update comes from the updater signature.
- Releases before this change have no Arch package, so the first one comes from the first release with this change.
- Another package format can reuse the marker with its own value and target.
- Amends ADR 0005 for the Arch package; the rest stands.
