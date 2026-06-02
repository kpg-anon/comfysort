<!-- ───────────────────────────── HERO ───────────────────────────── -->
<div align="center">

<h1><samp>comfysort</samp></h1>

<p>
  <b>A calm, preview-first desktop workstation for sorting large piles of media — fast.</b><br>
  <i>Preview the file. Press a key. It moves. Press <kbd>u</kbd> to undo. That's the whole loop.</i>
</p>

<p>
  <img src="https://img.shields.io/badge/version-0.3.0-c287ff?style=for-the-badge" alt="Version 0.3.0">
  <img src="https://img.shields.io/badge/Tauri-v2-24C8DB?style=for-the-badge&logo=tauri&logoColor=white" alt="Tauri v2">
  <img src="https://img.shields.io/badge/Svelte-5-FF3E00?style=for-the-badge&logo=svelte&logoColor=white" alt="Svelte 5">
  <img src="https://img.shields.io/badge/Rust-engine-CE412B?style=for-the-badge&logo=rust&logoColor=white" alt="Rust engine">
  <img src="https://img.shields.io/badge/license-MIT-6ea8ff?style=for-the-badge" alt="License: MIT">
</p>

<p>
  <img src="https://img.shields.io/badge/Windows-supported-1F2937?style=flat-square&logo=windows&logoColor=white" alt="Windows">
  <img src="https://img.shields.io/badge/status-active-82d65a?style=flat-square" alt="Status: active">
</p>

<!-- Add your own hero shot here. The screenshots dir may not exist yet — create docs/screenshots/ and drop a PNG in. -->
<img src="docs/screenshots/hero.png" alt="comfysort in action" width="860">

<sub><i>Native webview image/video previews, hotkey destinations, and a focusable folder navigator — all from the keyboard.</i></sub>

</div>

---

> [!NOTE]
> **comfysort** is the desktop-GUI sibling of the [`comfysort` terminal app](https://github.com/kpg-anon/comfysort). It is a ground-up reimplementation built on a real webview — so previews render at full quality and the layout is fluid — while keeping the TUI's hard-won safety guarantees. The two share principles, not code.

<!-- ───────────────────────────── TOC ───────────────────────────── -->
<details>
<summary><b>Table of contents</b></summary>

- [Why comfysort](#-why-comfysort)
- [The loop](#-the-loop)
- [Features](#-features)
- [The interface](#-the-interface)
- [Install & build](#-install--build)
- [Keyboard](#-keyboard)
- [Configuration](#-configuration)
- [How it stays safe](#-how-it-stays-safe)
- [Tech stack & architecture](#-tech-stack--architecture)
- [Roadmap](#-roadmap)
- [License](#-license)

</details>

## ✨ Why comfysort

If you triage thousands of images and videos at a time — photo dumps, screenshot graveyards, downloaded archives, render output — most file managers slow you down. They want you to *navigate*. **comfysort wants you to decide.**

| | Principle | What it means |
|:--:|:--|:--|
| 🖼️ | **Preview-first** | The image or video is the hero — rendered natively in the webview, full quality, front and center. |
| ⌨️ | **Keyboard-first** | Destinations bind to <kbd>1</kbd>–<kbd>9</kbd>, trash to <kbd>0</kbd>. A focus model routes navigation; hotkeys are global. |
| 🌙 | **Calm** | Dark theme, restrained palette, no decorative chrome. Built for long sessions. |
| ↩️ | **Explicit & reversible** | Nothing moves without an action. Every mutation is journaled. <kbd>u</kbd> walks it all back. |

It is **not** a general file manager and **not** an auto-sorter. Nothing on disk changes without an explicit user action.

## 🔁 The loop

Pick an **inbox** and a **destination root** → preview the current file → press a hotkey (or click a target) → it **moves**, **copies**, or goes to **trash** → every mutation is journaled → press <kbd>u</kbd> to walk it back.

```
   ┌──────────┐    ┌──────────┐    ┌──────────────┐    ┌──────────┐
   │  Scan    │ ─> │ Preview  │ ─> │ 1–9 move     │ ─> │ Journal  │
   │  inbox   │    │  file    │    │ 0   trash    │    │ + counts │
   └──────────┘    └────▲─────┘    │ ⇧+digit copy │    └────┬─────┘
                        │          └──────────────┘         │
                        └────────────── u (undo) ───────────┘
```

## 🎛️ Features

- [x] **Native webview previews** — images and videos render directly in the webview at full quality (via the Tauri asset protocol); dimensions and duration read straight from the media element.
- [x] **Keyboard-first three-pane workstation** — **Inbox** (left), **Preview** (center, the hero), and a right column of **File Info**, **Sort Targets**, and **Navigator**.
- [x] **Focus model** — <kbd>Tab</kbd> toggles keyboard focus between Inbox and Navigator; the focused pane gets a purple border and `*` marker. Hotkeys and undo stay global; navigation routes by focus.
- [x] **Journaled move / copy / trash** — every mutation is appended to `<output>/.comfysort/journal.jsonl` (intent before, result after).
- [x] **Multi-step session undo** — <kbd>u</kbd> walks the whole session back: moves restore the file *and* re-insert the inbox row, copies unlink the duplicate, trashes restore from `.trash`.
- [x] **Collision-safe rename** — name conflicts get Windows-Explorer-style `name (2).ext` suffixes; the default policy never overwrites.
- [x] **Cross-volume safe moves** — `rename` first; across a drive/share boundary it falls back to copy → verify size → delete source, with a confirm modal (<kbd>y</kbd> once / <kbd>a</kbd> always this session / <kbd>n</kbd> cancel) before any bytes move.
- [x] **Folder Navigator** — drill in/out of the destination tree, **fuzzy search** anywhere under the root (<kbd>/</kbd>), create and delete folders, with **recursive media counts** (a folder of subfolders still reflects its descendants).
- [x] **Persistent hotkey binding** — bind any folder under the output root to a slot (<kbd>Shift</kbd>+digit on a highlighted folder); binds survive restart via `<output>/.comfysort/bindings.json`. Only trash auto-binds, to <kbd>0</kbd>.
- [x] **Inbox sort / filter + multiselect** — cycle sort field (name/size/modified) and filter (all/images/videos); <kbd>Shift</kbd>+<kbd>↑</kbd>/<kbd>↓</kbd> extends a contiguous selection that move/copy then act on in a batch (each op journaled individually).
- [x] **Virtualized inbox** — only visible rows render, so an inbox of **25k+ files** stays smooth; mutating commands return tiny deltas instead of re-sending the list.
- [x] **Right-click context menu** — open in the default viewer, reveal in the file explorer, move to trash, or refresh the inbox.
- [x] **Native settings overlay** — a cog opens an in-app overlay that reads and writes `config.toml`.
- [x] **4 theme presets** — Comfy Dark, Nord, Gruvbox, Catppuccin.
- [x] **Disk-space readout** — free / total space for the destination drive.

## 🖥️ The interface

A header with input/output chips, a wide preview, and a right column of context panes — driven entirely from the keyboard.

```
┌─ ~/media/inbox ▾ ───────── ▸ Moved to keep ───────── comfysort v0.3.0 ⚙ ┐
├──────────────┬─────────────────────────────────────────┬───────────────────┤
│ 「 Inbox * 」 │             「 Preview 」                 │  「 File Info 」   │
│              │                                          │  Path · Size      │
│  Name  Size  │              ███████████████             │  Dimensions       │
│  ▸ clip.webm │              ███ native ████             │  Duration · Type  │
│    pic.jpg   │              ███ preview ███             ├───────────────────┤
│    meme.png  │              ███████████████             │ 「 Sort Targets 」 │
│              │                                          │  [1] keep   (463) │
│              │                                          │  [2] group  (128) │
│              │                                          │  [0] trash    (4) │
│ 2 selected   │                                          ├───────────────────┤
│ 29 items 77M │                                          │ 「 Navigator 」    │
├──────────────┴─────────────────────────────────────────┴───────────────────┤
│ tab focus · ↑↓ select · 1-9 move · 0 trash · ⇧+digit copy · u undo · / find │
└────────────────────────────────────────────────────────────────────────────┘
```

## 📦 Install & build

> [!IMPORTANT]
> Building from source is the path today. You need the **Tauri v2 prerequisites** for your OS, plus a Rust toolchain and Node.

**Prerequisites**

- **Rust** (stable) and Cargo
- **Node.js** + npm
- **Tauri v2 system prerequisites** — see the [Tauri prerequisites guide](https://v2.tauri.app/start/prerequisites/) (on Windows: the Microsoft C++ Build Tools and a WebView2 runtime)

**Run with hot reload**

```bash
npm install
npm run tauri dev
```

**Build a release bundle**

```bash
npm run tauri build
```

This produces a standalone **`comfysort.exe`** in `target/release/`, plus **MSI** and **NSIS** installers under `target/release/bundle/`.

**Test the engine** (pure Rust, no Tauri/webview stack — compiles fast):

```bash
cargo test -p comfysort-engine
```

> [!NOTE]
> The Tauri shell's `[lib] crate-type` is currently `["rlib"]` (desktop-only). The mobile `cdylib`/`staticlib` types overflow mingw's PE export table on the Windows-GNU toolchain — re-add them (ideally on MSVC with VS Build Tools) when targeting Android/iOS.

## ⌨️ Keyboard

There's a **focus model**: either the Inbox or the Navigator holds keyboard focus. <kbd>Tab</kbd> toggles it; the focused pane gets a purple border and a `*`. Hotkeys and undo are **global**; navigation routes by focus.

**Global — any focus**

| Key | Action |
|:--|:--|
| <kbd>Tab</kbd> | Toggle focus between Inbox and Navigator |
| <kbd>1</kbd>–<kbd>9</kbd> | Move current file (or selection) to that destination |
| <kbd>0</kbd> | Move to trash |
| <kbd>Shift</kbd>+<kbd>1</kbd>–<kbd>9</kbd> | **Copy** to that destination — original stays *(in Navigator focus, **binds** the highlighted folder to that slot)* |
| <kbd>u</kbd> | Undo the last operation (multi-step) |
| <kbd>/</kbd> | Open fuzzy folder search |
| <kbd>Ctrl</kbd>+<kbd>N</kbd> | New folder (inline prompt) |
| <kbd>Ctrl</kbd>+<kbd>R</kbd> | Toggle inbox sort order |
| <kbd>F5</kbd> | Refresh / rescan the inbox |
| <kbd>Esc</kbd> | Close overlay / cancel search |

**Inbox focus**

| Key | Action |
|:--|:--|
| <kbd>↑</kbd>/<kbd>↓</kbd> · <kbd>j</kbd>/<kbd>k</kbd> | Change selection |
| <kbd>Alt</kbd>+<kbd>↑</kbd> / <kbd>Alt</kbd>+<kbd>↓</kbd> | Jump to top / bottom |
| <kbd>Shift</kbd>+<kbd>↑</kbd>/<kbd>↓</kbd> | Extend a contiguous multiselection |
| <kbd>s</kbd> | Cycle sort field — name / size / modified |
| <kbd>f</kbd> | Cycle filter — all / images / videos |

**Navigator focus**

| Key | Action |
|:--|:--|
| <kbd>↑</kbd>/<kbd>↓</kbd> · <kbd>j</kbd>/<kbd>k</kbd> | Move cursor |
| <kbd>→</kbd>/<kbd>l</kbd> · <kbd>←</kbd>/<kbd>h</kbd> | Drill into folder / ascend |
| <kbd>Enter</kbd> | **Move** current file (or selection) into the highlighted folder |
| <kbd>Shift</kbd>+<kbd>D</kbd> | **Copy** into the highlighted folder (source stays) |
| <kbd>Ctrl</kbd>+<kbd>D</kbd> | Delete folder to trash (confirm prompt) |
| <kbd>Esc</kbd> | Return focus to the Inbox |

**Fuzzy search** (<kbd>/</kbd>): type to match folders anywhere under the root · <kbd>↑</kbd>/<kbd>↓</kbd> pick · <kbd>Enter</kbd> move into the match · <kbd>Esc</kbd> exit.

You can also **click** any Sort Target or Navigator row to act on it, and **right-click** an inbox item for the context menu.

## ⚙️ Configuration

Open **Settings** with the cog in the header. Changes persist to **`config.toml`** in the app config directory (atomically; missing or older files fall back to per-field defaults, never an error). One struct is the single source of truth for both backend behavior and the frontend.

| Setting | Values | Default |
|:--|:--|:--|
| Default input / output folders | paths | *(empty — auto-load when set)* |
| Collision policy | `rename` · `skip` · `overwrite` | `rename` |
| Confirm folder delete | on / off | on |
| Confirm cross-drive move | on / off | on |
| Default sort field | name · size · modified | modified |
| Default sort order | ascending · descending | descending |
| Default filter | all · images · videos | all |
| Video autoplay / loop / muted | on / off | on |
| Theme | Comfy Dark · Nord · Gruvbox · Catppuccin | Comfy Dark |

Set **default folders** and comfysort auto-loads that session on launch. Only the collision policy drives backend behavior (threaded into move/copy — trash and folder-delete always rename); the rest are read by the frontend. A button in Settings opens `config.toml` in your editor.

## 🛟 How it stays safe

> [!CAUTION]
> comfysort mutates files on disk. These invariants are why it's safe to drive at speed:

- **No autonomous moves.** Every mutation requires an explicit user action — there is no auto-sort.
- **Journal intent, then result.** Append-only JSONL at `<output>/.comfysort/journal.jsonl`.
- **Soft delete only.** "Trash" renames into `<output>/.comfysort/.trash/` — reversible — never an `rm`.
- **Collisions never clobber.** Conflicts get Explorer-style `name (2).ext` suffixes; the default policy is `rename`, never `overwrite`.
- **Cross-volume moves are verified.** Across a drive/share boundary it copies → verifies size → deletes the source (source kept until verified), behind a confirm modal.
- **Undo is real.** A session stack walks every op back: move → restore, copy → unlink duplicate, trash → restore.

## 🧱 Tech stack & architecture

Built with **Tauri v2** + **SvelteKit (Svelte 5, SPA)** + **TypeScript** over a **pure-Rust engine**.

It's a Cargo workspace with two members. The engine has **no Tauri imports** and is fully testable on its own; the Tauri shell is the only bridge to it. Filesystem mutation lives entirely in the engine's operations layer — never in command glue or the frontend.

```
comfysort-tauri/
├── Cargo.toml             # workspace root (members: src-tauri, crates/engine)
├── src/                   # SvelteKit frontend (SPA, ssr=false)
│   ├── routes/+page.svelte    # the app shell + global key dispatch
│   └── lib/                   # components, stores, api wrapper, themes, tokens
├── src-tauri/             # Tauri shell — commands.rs is the only IPC bridge
└── crates/engine/         # pure-Rust engine: journaled ops, undo, scan, search
    ├── domain.rs              # core types + serde DTOs sent to the frontend
    ├── media.rs               # media-kind detection + inbox scan
    ├── destinations.rs        # destination scan + recursive media counts
    ├── operations.rs          # journaled move/copy/trash + collision + relocate
    ├── persistence.rs         # per-root saved hotkey bindings (bindings.json)
    ├── search.rs              # recursive fuzzy folder search
    ├── settings.rs            # config.toml schema + load/save
    └── session.rs             # in-memory session state (roots, items, op stack)
```

The frontend mirrors the engine DTOs in a single typed IPC wrapper (`src/lib/api.ts`). The inbox is sent in full only when a session opens; mutating commands return small deltas so the frontend updates its local list without re-serializing thousands of items.

## 🗺️ Roadmap

- [ ] Restart-safe undo via journal replay (per-session multi-step undo already works)
- [ ] File operations on a worker queue for very large batches
- [ ] Recursive inbox folders with breadcrumbs
- [ ] Richer media metadata (EXIF, codec, color depth)
- [ ] Backend thumbnail pipeline for huge inboxes
- [ ] macOS / Linux bundles and a release pipeline

## 📜 License

**MIT.**

> [!NOTE]
> `package.json` and the manifest declare MIT, but a `LICENSE` file may still need to be added to the repo root.

---

<div align="center">
<sub>A calm place to sort your media. Preview the file. Press a key. <kbd>u</kbd> to undo.</sub>
</div>
