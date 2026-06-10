# Changelog

All notable changes to **comfysort** are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.7] — 2026-06-10

### Fixed
- **The setup.exe installer now ships `WebView2Loader.dll`.** The NSIS bundle was missing the loader DLL the exe needs at runtime, so a fresh setup.exe install — and every in-app update, which installs via the NSIS package — produced an app that failed to launch ("WebView2Loader.dll was not found" / error 0xc00004bc on relaunch). The MSI already shipped it; now both do. In-app updates land on a working install again from this release forward.

## [0.4.6] — 2026-06-10

### Added
- **Delete from the Navigator context menu** — right-click a folder → "Delete to trash…", the same reversible flow as Ctrl+D.

### Changed
- The cross-drive and folder-delete confirmations share a redesigned themed modal: accent-tinted header with an icon tile, blurred backdrop, and keyboard-hinted buttons. Folder deletes no longer use the bare browser confirm box, and the prompt now says what the folder holds ("3 media files · 2 subfolders").

### Fixed
- The refresh status reported the inbox total ("Refreshed — 10 items"); it now reports what the rescan actually found: "Refreshed — 0 new items".

## [0.4.5] — 2026-06-10

### Fixed
- **Portable builds no longer run the installer on update.** The update notice now detects a portable copy (the `config.toml` beside the exe) and offers the new portable zip as a download instead of silently running the NSIS installer — which installed a second copy elsewhere and could fail with a missing `WebView2Loader.dll`. Installed (setup.exe / MSI) builds keep the one-click in-app update.

> Note for portable users on 0.4.3/0.4.4: this fix ships *in* 0.4.5, so the update prompt in your current version still shows the old behavior — press **Later** and update once by replacing your folder with the [0.4.5 portable zip](https://github.com/kpg-anon/comfysort/releases/tag/v0.4.5). From 0.4.5 on, the prompt does the right thing.

## [0.4.4] — 2026-06-10

### Added
- **Recursive inbox scan** — a new Behavior toggle in Settings walks every subfolder of the inbox folder(s) and merges nested media into the queue. Off by default; flipping it rescans the open inbox immediately, and the choice persists to `config.toml` like everything else.

### Changed
- The destination-root chip and the history/settings buttons use the same themed tooltip as the rest of the header (no more OS tooltip box).
- Inbox filenames show the full name in the shared themed tooltip instead of the OS one.

### Fixed
- The inbox "Size" column header is centered over its column like "Type" (was right-aligned).

## [0.4.3] — 2026-06-03

### Changed
- All hover tooltips (header, sort targets, and Navigator folders) are themed in-app popovers instead of the OS tooltip box; a multi-folder inbox lists its folders there.
- The refresh and add-folder buttons are rounded squares, matching the history/settings buttons.
- A long inbox path shows as the folder name (full path in its tooltip), so the header buttons are no longer pushed off-screen.

### Fixed
- The "add inbox folder" button now renders its icon (was a missing glyph codepoint).
- The folder picker is parented to the main window so it reliably opens in front.

## [0.4.2] — 2026-06-03

### Added
- **Multiple inbox folders** — once a session is open, the ＋ button beside the inbox folder adds more source folders; their media merges into one inbox.

### Changed
- The header refresh button animates on click for clearer feedback.
- Settings: the "opens straight into them" note moved below the default folders as smaller subtext.

## [0.4.1] — 2026-06-03

### Added
- A refresh button beside the inbox folder in the header to rescan for new files (previously only via the inbox right-click menu).

### Changed
- The `=` / `−` sort slots are greyed to set them apart from the numbered targets, under a cleaner bar separator.
- The sort-target editor opens in its own panel; "check for updates on launch" moved into Startup settings.
- Portable build now ships `config.toml` beside the executable (alongside the WebView2 loader), so settings travel with the app and the file always exists to edit.

### Fixed
- `Shift+D` copies into the target folder — whether you've drilled into it or have a fuzzy-search match selected.
- Fuzzy search no longer accepts only one character at a time after a copy.

## [0.4.0] — 2026-06-03

### Added
- **Action history** — a popup (icon next to the settings cog) lists this session's moves/copies/trashes and can revert any single file individually.
- **Sort-target editor** — a dedicated panel (opened from Settings) to bind hotkey slots to any folder, including folders outside the destination root.
- **`=` archive slot** — defaults to a managed `.comfysort/archive` folder; the `−` slot is available for a second custom destination.
- **Navigator right-click menu** — open a folder in Explorer, or rename it in place.
- **Type-to-search in the Navigator** — just start typing to fuzzy-find folders.
- **"Check for updates on launch" toggle** in Settings.
- **Portable build** keeps `config.toml` beside the executable and bundles the WebView2 loader, so the app travels in one folder.

### Changed
- Undo is now **Ctrl+U**; arrow keys handle all list navigation (the `j`/`k`/`h`/`l` shortcuts and the Navigator's `/` were removed).
- The preview's media-type icon matches the inbox color (green image / blue video).
- Brighter row hover in the inbox and Navigator; right-clicking a Navigator folder now selects it.

### Fixed
- `Shift+D` now copies into the highlighted Navigator folder from any pane.
- Folder media counts refresh immediately after a copy while searching.
- `Tab` during a Navigator search returns focus to the inbox cleanly.
- After a multi-select move, the cursor lands on the next item (not two down).

## [0.3.2] — 2026-06-02

### Added
- **In-app updater** — comfysort checks for new releases on launch and shows a notification when one is available, with one-click **Update now** (downloads, installs, and relaunches).

## [0.3.1] — 2026-06-02

### Added
- New application icon across the app and installers.
- MIT `LICENSE`.
- Downloadable builds on the Releases page — installers plus a portable zip — and a Download section in the README.

### Changed
- README overhaul: new hero, a rendered "loop" diagram, and a cleaner structure.

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

[Unreleased]: https://github.com/kpg-anon/comfysort/compare/v0.4.7...HEAD
[0.4.7]: https://github.com/kpg-anon/comfysort/releases/tag/v0.4.7
[0.4.6]: https://github.com/kpg-anon/comfysort/releases/tag/v0.4.6
[0.4.5]: https://github.com/kpg-anon/comfysort/releases/tag/v0.4.5
[0.4.4]: https://github.com/kpg-anon/comfysort/releases/tag/v0.4.4
[0.4.3]: https://github.com/kpg-anon/comfysort/releases/tag/v0.4.3
[0.4.2]: https://github.com/kpg-anon/comfysort/releases/tag/v0.4.2
[0.4.1]: https://github.com/kpg-anon/comfysort/releases/tag/v0.4.1
[0.4.0]: https://github.com/kpg-anon/comfysort/releases/tag/v0.4.0
[0.3.2]: https://github.com/kpg-anon/comfysort/releases/tag/v0.3.2
[0.3.1]: https://github.com/kpg-anon/comfysort/releases/tag/v0.3.1
