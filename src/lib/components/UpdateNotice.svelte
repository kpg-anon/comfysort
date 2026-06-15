<script lang="ts">
  import { onMount } from "svelte";
  import { settings } from "$lib/settings.svelte";
  import { api } from "$lib/api";

  type Phase = "idle" | "available" | "downloading" | "ready" | "error";
  let phase: Phase = $state("idle");
  let version = $state("");
  let notes = $state("");
  let progress = $state(0); // 0..100
  let errorMsg = $state("");
  let dismissed = $state(false);
  // Portable copies (config.toml beside the exe) must not run the NSIS
  // installer — it installs elsewhere instead of updating the folder. They get
  // a "download the new portable zip" action instead.
  let portable = $state(false);
  // The Update handle from the updater plugin (kept untyped to avoid pulling the
  // plugin's types into the SSR/prerender pass; this app is SPA-only anyway).
  let update: any = null;

  // Sample release body (mirrors the CHANGELOG format release.ps1 ships) for the
  // dev-only preview below — exercises headings, bullets, bold, `code`, and the
  // "--- Full changelog: <url>" footer so the rendering can be eyeballed in dev.
  const SAMPLE_NOTES = `### Added
- **Restore last session.** The start screen offers a one-click **Restore last session** whenever your last inbox/destination differ from your configured defaults; the roots are remembered in \`config.toml\`.
- **Recursive-scan prompt** when adding or changing an inbox folder — choose *Top level only* or *Include subfolders* each time.
- **Empty trash** from Settings — a new **Trash** section clears \`.comfysort/.trash\` (with an inline confirm).

### Changed
- **Redesigned confirmation dialogs** into a flat split choice-cards layout — no more glow, gradient, or blur.
- **Renaming a bound folder updates its sort-target label immediately.**

### Fixed
- **No more startup flash** — the window opens already painted in your theme.
- **Context menus no longer get cut off near a screen edge.**

---
Full changelog: https://github.com/kpg-anon/comfysort/blob/master/CHANGELOG.md`;

  onMount(async () => {
    // Dev-only: lets you preview the update toast without a real release. Open the
    // devtools console and call `__previewUpdate()` (or `__previewUpdate({ portable: true })`).
    // Gated on import.meta.env.DEV, so it's stripped from production builds.
    if (import.meta.env.DEV) {
      (window as unknown as Record<string, unknown>).__previewUpdate = (opts?: { portable?: boolean }) => {
        version = "9.9.9";
        notes = SAMPLE_NOTES;
        portable = opts?.portable ?? false;
        progress = 0;
        errorMsg = "";
        dismissed = false;
        phase = "available";
      };
      console.info("[updater] dev: call __previewUpdate() in the console to preview the update toast");
    }

    // Respect the "check for updates on launch" setting (load it first if the
    // app just started and config.toml hasn't been read yet).
    if (!settings.loaded) {
      try { await settings.load(); } catch { /* keep defaults */ }
    }
    if (!settings.autoUpdateCheck) return;
    try {
      portable = await api.isPortable();
    } catch {
      /* treat as installed */
    }
    // The updater only resolves in the packaged app against the public release
    // endpoint. In dev, offline, or when no newer release is published, this
    // throws or returns null — fail silently rather than nag the user.
    try {
      const { check } = await import("@tauri-apps/plugin-updater");
      const found = await check();
      if (found) {
        update = found;
        version = found.version;
        notes = (found.body ?? "").trim();
        phase = "available";
      }
    } catch (e) {
      console.debug("[updater] check skipped:", e);
    }
  });

  /** Portable update path: open the release's portable zip in the browser. */
  async function downloadPortable() {
    try {
      const { openUrl } = await import("@tauri-apps/plugin-opener");
      await openUrl(
        `https://github.com/kpg-anon/comfysort/releases/download/v${version}/comfysort_${version}_x64-portable.zip`,
      );
      dismissed = true;
    } catch (e) {
      errorMsg = String(e);
      phase = "error";
    }
  }

  async function install() {
    if (!update) return;
    phase = "downloading";
    let total = 0;
    let got = 0;
    try {
      await update.downloadAndInstall((ev: any) => {
        switch (ev.event) {
          case "Started":
            total = ev.data?.contentLength ?? 0;
            break;
          case "Progress":
            got += ev.data?.chunkLength ?? 0;
            progress = total ? Math.min(100, Math.round((got / total) * 100)) : 0;
            break;
          case "Finished":
            progress = 100;
            break;
        }
      });
      phase = "ready";
      const { relaunch } = await import("@tauri-apps/plugin-process");
      await relaunch();
    } catch (e) {
      errorMsg = String(e);
      phase = "error";
    }
  }

  // ---- Release-notes rendering ---------------------------------------------
  // The release body is the CHANGELOG section (markdown). Parse a small subset —
  // ### headings, - bullets, **bold**, `code`, links — into structured blocks so
  // the toast renders them cleanly (purple bullets, scrollable) instead of raw
  // markdown text.
  type Seg = { t: "text" | "bold" | "code" | "link"; v: string; href?: string };
  type Block =
    | { kind: "heading"; text: string }
    | { kind: "bullet"; segs: Seg[] }
    | { kind: "para"; segs: Seg[] }
    | { kind: "rule" };

  function parseInline(s: string): Seg[] {
    const segs: Seg[] = [];
    const re = /\*\*([^*]+)\*\*|`([^`]+)`|\[([^\]]+)\]\(([^)]+)\)|(https?:\/\/[^\s)]+)/g;
    let last = 0;
    let m: RegExpExecArray | null;
    while ((m = re.exec(s))) {
      if (m.index > last) segs.push({ t: "text", v: s.slice(last, m.index) });
      if (m[1] !== undefined) segs.push({ t: "bold", v: m[1] });
      else if (m[2] !== undefined) segs.push({ t: "code", v: m[2] });
      else if (m[3] !== undefined) segs.push({ t: "link", v: m[3], href: m[4] });
      else if (m[5] !== undefined) segs.push({ t: "link", v: m[5], href: m[5] });
      last = re.lastIndex;
    }
    if (last < s.length) segs.push({ t: "text", v: s.slice(last) });
    return segs;
  }

  function parseNotes(body: string): Block[] {
    const out: Block[] = [];
    for (const raw of body.split(/\r?\n/)) {
      const line = raw.trim();
      if (!line) continue;
      if (/^[-*_]{3,}$/.test(line)) { out.push({ kind: "rule" }); continue; }
      const h = line.match(/^#{1,6}\s+(.*)$/);
      if (h) { out.push({ kind: "heading", text: h[1] }); continue; }
      const b = line.match(/^[-*]\s+(.*)$/);
      if (b) { out.push({ kind: "bullet", segs: parseInline(b[1]) }); continue; }
      out.push({ kind: "para", segs: parseInline(line) });
    }
    return out;
  }

  async function openExternal(url: string) {
    if (!url) return;
    try {
      const { openUrl } = await import("@tauri-apps/plugin-opener");
      await openUrl(url);
    } catch (e) {
      console.debug("[updater] openUrl failed:", e);
    }
  }

  // Split the trailing "--- Full changelog: <url>" footer off the body, parse the
  // rest into blocks, and surface the footer URL as a "Full changelog" link.
  const noteParts = $derived(notes.split(/\n-{3,}\s*\n/));
  const blocks = $derived(parseNotes(noteParts[0] ?? ""));
  const changelogUrl = $derived(
    (noteParts.slice(1).join("\n").match(/https?:\/\/\S+/) ?? [""])[0],
  );
</script>

{#snippet inline(segs: Seg[])}{#each segs as s}{#if s.t === "bold"}<b>{s.v}</b>{:else if s.t === "code"}<code>{s.v}</code>{:else if s.t === "link"}<a class="cl-link" href={s.href} onclick={(e) => { e.preventDefault(); openExternal(s.href ?? ""); }}>{s.v}</a>{:else}{s.v}{/if}{/each}{/snippet}

{#if phase !== "idle" && !dismissed}
  <div class="upd" role="status">
    {#if phase === "available"}
      <div class="head">
        <span class="dot"></span>
        <span class="title">Update available — <b>v{version}</b></span>
      </div>
      {#if blocks.length}
        <div class="changelog">
          {#each blocks as blk}
            {#if blk.kind === "heading"}
              <div class="cl-h">{blk.text}</div>
            {:else if blk.kind === "rule"}
              <div class="cl-rule"></div>
            {:else if blk.kind === "bullet"}
              <div class="cl-bullet"><span class="cl-dot">•</span><span class="cl-text">{@render inline(blk.segs)}</span></div>
            {:else}
              <p class="cl-para">{@render inline(blk.segs)}</p>
            {/if}
          {/each}
        </div>
        {#if changelogUrl}
          <button class="cl-foot" onclick={() => openExternal(changelogUrl)}>Full changelog ↗</button>
        {/if}
      {/if}
      {#if portable}
        <p class="notes hint">Portable build — grab the new zip and replace this app folder. Your settings travel in config.toml beside the exe.</p>
        <div class="row">
          <button class="btn go" onclick={downloadPortable}>Download zip</button>
          <button class="btn ghost" onclick={() => (dismissed = true)}>Later</button>
        </div>
      {:else}
        <div class="row">
          <button class="btn go" onclick={install}>Update now</button>
          <button class="btn ghost" onclick={() => (dismissed = true)}>Later</button>
        </div>
      {/if}
    {:else if phase === "downloading"}
      <div class="head"><span class="title">Downloading v{version}…</span></div>
      <div class="bar"><div class="fill" style="width:{progress}%"></div></div>
      <p class="notes">{progress}% — the app will restart when it's done.</p>
    {:else if phase === "ready"}
      <div class="head"><span class="title">Restarting to finish update…</span></div>
    {:else if phase === "error"}
      <div class="head"><span class="title err">Update failed</span></div>
      <p class="notes">{errorMsg}</p>
      <div class="row"><button class="btn ghost" onclick={() => (dismissed = true)}>Dismiss</button></div>
    {/if}
  </div>
{/if}

<style>
  .upd {
    position: fixed;
    right: 16px;
    bottom: 16px;
    z-index: 45;
    width: 340px;
    background: var(--bg-panel);
    border: 1px solid var(--border);
    border-top: 2px solid var(--purple);
    border-radius: 12px;
    padding: 13px 15px 14px;
    box-shadow: 0 18px 50px rgba(0, 0, 0, 0.5);
    animation: upop 0.16s ease-out;
  }
  @keyframes upop { from { opacity: 0; transform: translateY(8px); } }
  .head { display: flex; align-items: center; gap: 8px; }
  .dot {
    flex: none; width: 8px; height: 8px; border-radius: 50%;
    background: var(--purple);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--purple) 22%, transparent);
  }
  .title { color: var(--text-primary); font-size: 13px; font-weight: 600; }
  .title b { color: var(--purple); }
  .title.err { color: var(--red); }
  .notes {
    color: var(--text-muted); font-size: 11.5px; line-height: 1.45;
    margin: 8px 0 0; white-space: pre-wrap;
  }
  .notes.hint { color: var(--yellow); }

  /* Structured, scrollable changelog. */
  .changelog {
    margin-top: 10px;
    max-height: 190px;
    overflow-y: auto;
    padding-right: 7px;
    scrollbar-width: thin;
    scrollbar-color: var(--border) transparent;
  }
  .changelog::-webkit-scrollbar { width: 7px; }
  .changelog::-webkit-scrollbar-thumb { background: var(--border); border-radius: 99px; }
  .changelog::-webkit-scrollbar-thumb:hover { background: var(--text-muted); }
  .cl-h {
    font-size: 9.5px; text-transform: uppercase; letter-spacing: 0.08em;
    color: var(--purple); font-weight: 700; margin: 11px 0 5px;
  }
  .cl-h:first-child { margin-top: 0; }
  .cl-bullet { display: flex; gap: 7px; align-items: baseline; margin: 4px 0; }
  .cl-dot { flex: none; color: var(--purple); font-size: 13px; line-height: 1.2; }
  .cl-text { color: var(--text-muted); font-size: 11.5px; line-height: 1.5; }
  .cl-para { color: var(--text-muted); font-size: 11.5px; line-height: 1.5; margin: 6px 0 0; }
  .cl-rule { height: 1px; background: var(--border-muted); margin: 9px 0; }
  .changelog :global(b) { color: var(--text-secondary); font-weight: 600; }
  .changelog :global(code) {
    font-family: var(--mono); font-size: 10.5px;
    background: var(--bg-chip); border: 1px solid var(--border-muted);
    border-radius: 4px; padding: 0 4px; color: var(--text-secondary);
  }
  .changelog :global(.cl-link) { color: var(--purple); text-decoration: none; cursor: pointer; }
  .changelog :global(.cl-link):hover { text-decoration: underline; }
  .cl-foot {
    display: inline-flex; align-items: center; gap: 4px;
    margin-top: 9px; padding: 0; background: none; border: none;
    color: var(--purple); font-size: 11px; cursor: pointer;
  }
  .cl-foot:hover { text-decoration: underline; }

  .row { display: flex; gap: 8px; margin-top: 12px; }
  .btn {
    flex: 1; padding: 8px; border-radius: var(--radius);
    border: 1px solid var(--border); background: var(--bg-chip);
    color: var(--text-secondary); cursor: pointer; font-size: 12px;
  }
  .btn:hover { border-color: var(--text-muted); color: var(--text-primary); }
  .btn.go {
    background: var(--purple); color: var(--text-inverse);
    border-color: var(--purple); font-weight: 600;
  }
  .btn.go:hover { filter: brightness(1.07); }
  .btn.ghost { flex: 0 0 auto; padding: 8px 12px; }
  .bar {
    margin-top: 10px; height: 6px; border-radius: 99px;
    background: var(--bg-chip); overflow: hidden;
  }
  .fill { height: 100%; background: var(--purple); transition: width 0.15s ease-out; }
</style>
