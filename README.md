<div align="center">

# comfysort

**A calm, preview-first desktop workstation for sorting large piles of media — fast.**

*Preview the file. Press a key. It moves. Press `u` to undo.*

Tauri v2 · SvelteKit · TypeScript · Rust

</div>

---

A GUI sibling to the [`comfysort` TUI](../comfysort), built from the ground up to
take advantage of a real webview (native image/video preview, fluid layout) while
keeping the TUI's hard-won safety guarantees.

## The loop (milestone 1 — working)

Pick an **inbox** and a **destination root** → preview the current file →
press a hotkey (or click a target) → it **moves**, **copies**, or goes to **trash**
→ every mutation is journaled → press **`u`** to walk it back.

- **Native previews** — images and videos render directly in the webview, full quality.
- **Auto-bound hotkeys** — `1`–`9` map to the destination folders under the output
  root; `0` is trash. `Shift`+digit copies instead of moving.
- **Journaled & reversible** — append-only `journal.jsonl`, multi-step session undo,
  Windows-Explorer-style `name (2).ext` collision rename, cross-volume safe moves.
- **Soft delete** — "trash" moves into `<output>/.comfysort/.trash/`, never `rm`.

## Keyboard

| Key | Action |
|---|---|
| `↑`/`↓` or `j`/`k` | Change selection |
| `1`–`9` | Move current file to that destination |
| `0` | Move to trash |
| `Shift`+`1`–`9` | Copy to that destination (original stays) |
| `u` | Undo last operation |
| click a Sort Target | Move current file there |

## Develop

Prereqs: Rust, Node, and the Tauri v2 prerequisites for your OS.

```bash
npm install
npm run tauri dev      # launch the app with hot reload
npm run tauri build    # release bundle (desktop)
```

Test the engine (pure, no Tauri deps):

```bash
cargo test -p comfysort-engine
```

> **Windows toolchain note.** This repo currently builds with the
> `x86_64-pc-windows-gnu` toolchain. The Tauri shell's `[lib] crate-type` is set to
> `["rlib"]` (desktop-only): the mobile `cdylib`/`staticlib` types overflow mingw's
> PE export table. Re-add them — ideally on the MSVC toolchain with VS Build Tools —
> when targeting Android/iOS.

## Layout

```
comfysort-tauri/
├── Cargo.toml             # workspace root (members: src-tauri, crates/engine)
├── ARCHITECTURE.md        # the canonical design + IPC-contract doc
├── src/                   # SvelteKit frontend (SPA)
│   ├── routes/+page.svelte
│   └── lib/{api.ts, session.svelte.ts, theme.css, components/}
├── src-tauri/             # Tauri shell: commands.rs is the only IPC bridge
└── crates/engine/         # pure Rust sorting engine (journaled ops + undo)
```

See **[`ARCHITECTURE.md`](ARCHITECTURE.md)** for the IPC command contract, DTOs,
safety invariants, and design language. The `.claude/skills/` directory holds 39
Tauri reference skills (auto-discovered by Claude Code).

## License

MIT
