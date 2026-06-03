# Changelog

All notable changes to **comfysort** are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.2] — 2026-06-02

### Added
- **In-app updater** — comfysort checks GitHub Releases on launch and shows a notification when a newer version is available, with one-click **Update now** (downloads, installs, and relaunches). Updates are verified with a self-generated signing key — no paid code-signing certificate required. (Activates once the repo is public.)

## [0.3.1] — 2026-06-02

### Added
- New application icon, regenerated across every platform size and embedded in the standalone exe and installers.
- MIT `LICENSE` file at the repo root.
- Release automation in `scripts/`: `bump-version.ps1` (version across all manifests), `clean.ps1` (reclaim build artifacts), and `release.ps1` (bump → build → commit → tag → push → GitHub Release → prune).
- GitHub Releases publishing with a **portable** zip alongside the NSIS/MSI installers, plus a Download section in the README.

### Changed
- README overhaul: mascot hero moved to the top, "The loop" is now a rendered Mermaid diagram, and the interface mockup was replaced with prose + a screenshots placeholder.

## [0.3.0] — 2026-05-30

### Added
- "Moving…" indicator and the app version shown in the header brand.
- Default input/output folders that auto-load straight into a session on launch.
- Open `config.toml` directly from the Settings overlay.

### Changed
- Renamed the binary to `comfysort`; the window now centers on launch.

### Fixed
- UI freeze on large moves — all mutating/picker commands are now async.
- Horizontal overscroll; navigator selection is sticky after a move.

## [0.2.0] — 2026-05 (keyboard-first milestone)

### Added
- TUI-parity keyboard model: hotkey binding (persisted per output root), recursive fuzzy folder search, contiguous multiselect, and cross-volume safe moves behind a confirm modal.
- Settings overlay backed by `config.toml`; four theme presets (Comfy Dark, Nord, Gruvbox, Catppuccin).
- Virtualized inbox for 25k+ files; right-click context menu; disk-space readout.
- Folder Navigator with recursive media counts.

## [0.1.0] — 2026-05 (milestone 1)

### Added
- Initial Tauri v2 + SvelteKit (Svelte 5) GUI over the pure-Rust engine: three-pane workstation, native webview image/video previews, journaled move/copy/trash with multi-step session undo, and a folder navigator.

[Unreleased]: https://github.com/kpg-anon/comfysort/compare/v0.3.2...HEAD
[0.3.2]: https://github.com/kpg-anon/comfysort/releases/tag/v0.3.2
[0.3.1]: https://github.com/kpg-anon/comfysort/releases/tag/v0.3.1
