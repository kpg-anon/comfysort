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

  onMount(async () => {
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

  // Truncate long release bodies for the toast.
  const shortNotes = $derived(notes.length > 220 ? notes.slice(0, 217) + "…" : notes);
</script>

{#if phase !== "idle" && !dismissed}
  <div class="upd" role="status">
    {#if phase === "available"}
      <div class="head">
        <span class="dot"></span>
        <span class="title">Update available — <b>v{version}</b></span>
      </div>
      {#if shortNotes}<p class="notes">{shortNotes}</p>{/if}
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
