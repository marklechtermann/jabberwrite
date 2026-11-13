---
status: accepted
date: 2026-11-13
---

# 0010: Package-manager installs turn off the in-app updater; Arch gets an AUR package

## Context

The in-app updater (ADR 0005) replaces an AppImage in place or installs a `.deb` through `pkexec dpkg -i`. On Arch Linux neither fits: an AppImage lives outside pacman, and there is no `dpkg`. Arch users expect `pacman`/AUR helpers to install and update their software.

If a package manager installs Jabberwrite, its files belong to that package manager. The in-app updater must not touch them: it would fail (no `dpkg`), or overwrite files pacman owns and leave its database wrong.

Alternatives considered:

- **Detect the distribution** (no `APPIMAGE`, no `dpkg`): guesses, and breaks for other package managers or a `.deb` repacked elsewhere.
- **Compile-time feature for packaged builds**: needs a separate build per package format; the AUR package could no longer reuse the signed release `.deb`.
- **Marker file shipped by the package**: explicit, works for any package manager, no extra build.

## Decision

**Marker file.** A package that owns the install ships `<prefix>/share/jabberwrite/package-manager` next to `<prefix>/bin/jabberwrite`, e.g. `/usr/share/jabberwrite/package-manager`. Its text tells the user how to update; an empty file gets a generic hint. At startup the app looks for the marker relative to its own executable, ignoring it inside an AppImage. If it exists, the update status is `managed`: the app never checks or installs updates, the tray item "Check for Updates…" shows the hint, and Settings → Updates shows only the hint.

**AUR package `jabberwrite-bin`.** `packaging/aur/PKGBUILD` repackages the release `.deb` (no rebuild) and adds the marker. The release workflow sets `pkgver`, updates the checksum and pushes the package to the AUR with `KSXGitHub/github-actions-deploy-aur`. The job runs only if the secret `AUR_SSH_PRIVATE_KEY` is set, so a missing AUR setup never blocks a release.

## Consequences

- Arch users install with `yay -S jabberwrite-bin` and update with `yay -Syu`.
- The PKGBUILD in this repository is the source of truth; edits made directly in the AUR are overwritten by the next release.
- One-time setup: an AUR account that owns `jabberwrite-bin`, its SSH key in the secret `AUR_SSH_PRIVATE_KEY`, optionally the variables `AUR_USERNAME` and `AUR_EMAIL` for the AUR commit author.
- The `.deb` from GitHub Releases has no marker and keeps updating itself (ADR 0005). Other distribution packages can opt in by shipping the marker.
- Amends ADR 0005 for package-manager installs; the rest stands.
