---
status: accepted
date: 2026-10-29
---

# 0004: Continuous versioning, CI and packaging

## Context

Every build must carry a unique version, shown in the settings dialog, and CI must guarantee the app builds on all three platforms.

## Decision

- Version scheme `MAJOR.MINOR.PATCH`: `MAJOR.MINOR` set by hand in `src-tauri/Cargo.toml` (single source of truth; `tauri.conf.json` reads from it). PATCH is the GitHub Actions run number, injected by CI before building. `package.json` stays at `0.0.0` and is ignored.
- Local builds report `MAJOR.MINOR.0-dev+<shortsha>`.
- GitHub Actions matrix: Windows x64, Linux x64, macOS arm64.
  - Pull requests: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`, frontend type-check, build.
  - Push to `main`: build installers with `tauri-action`, tag `vX.Y.N`, publish a GitHub Release with the artifacts.
- Installers from day one, unsigned: `.msi`/NSIS (Windows), `.dmg` (macOS), `.AppImage` + `.deb` (Linux).
- No code signing, notarization or auto-updater in v1.

## Consequences

- Every merge to `main` produces a versioned, downloadable release automatically.
- Unsigned builds trigger SmartScreen and Gatekeeper warnings; macOS permission prompts are less smooth.
- The run number only increases, so versions are monotonic but not contiguous per MINOR.
