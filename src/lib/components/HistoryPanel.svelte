<script lang="ts">
  import { session } from "$lib/session.svelte";
  import { I } from "$lib/icons";

  function timeStr(ms: number): string {
    return new Date(ms).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", second: "2-digit" });
  }
  const kindMeta: Record<string, { label: string; cls: string }> = {
    move: { label: "moved", cls: "move" },
    copy: { label: "copied", cls: "copy" },
    trash: { label: "trashed", cls: "trash" },
    undo: { label: "undo", cls: "undo" },
  };
  /** Parent folder name of a resolved path (where the file landed). */
  function landedIn(p: string): string {
    const segs = p.replace(/\\/g, "/").replace(/\/+$/, "").split("/");
    return segs[segs.length - 2] ?? "";
  }
</script>

{#if session.showHistory}
  <div class="scrim" role="presentation" onclick={() => session.toggleHistory()}></div>
  <div class="panel">
    <header>
      <span class="nf hicon">{I.history}</span>
      <h2>Action history</h2>
      <button class="x nf" title="Close (Esc)" onclick={() => session.toggleHistory()}>{I.close}</button>
    </header>
    <div class="body">
      {#if session.history.length === 0}
        <div class="empty">No actions yet this session.</div>
      {:else}
        {#each session.history as h (h.id)}
          <div class="entry" class:reverted={h.reverted}>
            <span class="badge {kindMeta[h.kind]?.cls ?? ''}">{kindMeta[h.kind]?.label ?? h.kind}</span>
            <div class="info">
              <div class="fn" title={h.resolvedPath}>{h.fileName}</div>
              <div class="sub">→ {landedIn(h.resolvedPath) || "—"} · {timeStr(h.time)}</div>
            </div>
            {#if h.reverted}
              <span class="done">reverted</span>
            {:else}
              <button class="revert" title="Undo just this file" onclick={() => session.revertEntry(h)}>
                <span class="nf">{I.undo}</span> revert
              </button>
            {/if}
          </div>
        {/each}
      {/if}
    </div>
    <footer><span>Per-file revert — independent of the <kbd>Ctrl+U</kbd> undo stack.</span></footer>
  </div>
{/if}

<style>
  .scrim {
    position: fixed; inset: 0; z-index: 55;
    background: rgba(8, 10, 13, 0.45);
    animation: fade 0.12s ease-out;
  }
  .panel {
    position: fixed; z-index: 56; top: 52px; right: 14px;
    width: 360px; max-height: 76vh; display: flex; flex-direction: column;
    background: var(--bg-panel); border: 1px solid var(--border);
    border-top: 2px solid var(--purple);
    border-radius: 12px; box-shadow: 0 24px 70px rgba(0, 0, 0, 0.5);
    overflow: hidden; animation: pop 0.13s ease-out;
  }
  @keyframes fade { from { opacity: 0; } }
  @keyframes pop { from { opacity: 0; transform: translateY(-6px); } }
  header {
    display: flex; align-items: center; gap: 10px;
    padding: 12px 14px; border-bottom: 1px solid var(--border-muted);
  }
  header .hicon { color: var(--purple); font-size: 14px; }
  header h2 { margin: 0; font-size: 14px; color: var(--text-primary); flex: 1; }
  .x {
    border: 1px solid var(--border); background: var(--bg-chip); color: var(--text-muted);
    width: 24px; height: 24px; border-radius: var(--radius-sm); cursor: pointer;
    display: grid; place-items: center; font-size: 11px;
  }
  .x:hover { color: var(--red); border-color: var(--red); }
  .body { overflow-y: auto; overscroll-behavior: contain; padding: 6px; }
  .empty { padding: 28px 12px; color: var(--text-muted); text-align: center; font-size: 12.5px; }
  .entry {
    display: grid; grid-template-columns: auto 1fr auto; gap: 9px; align-items: center;
    padding: 7px 8px; border-radius: var(--radius-sm);
  }
  .entry:hover { background: var(--bg-panel-alt); }
  .entry.reverted { opacity: 0.5; }
  .badge {
    flex: none; font-size: 10px; font-weight: 700; text-transform: uppercase;
    padding: 2px 6px; border-radius: 4px; letter-spacing: 0.03em;
    background: var(--bg-chip); color: var(--text-muted);
  }
  .badge.move { color: var(--green); }
  .badge.copy { color: var(--cyan); }
  .badge.trash { color: var(--red); }
  .info { min-width: 0; }
  .fn { color: var(--text-primary); font-size: 12.5px; overflow: hidden; white-space: nowrap; text-overflow: ellipsis; }
  .sub { color: var(--text-muted); font-size: 10.5px; font-family: var(--mono); overflow: hidden; white-space: nowrap; text-overflow: ellipsis; }
  .revert {
    flex: none; display: inline-flex; align-items: center; gap: 5px;
    border: 1px solid var(--border); background: var(--bg-chip); color: var(--text-secondary);
    border-radius: var(--radius-sm); padding: 4px 8px; cursor: pointer; font-size: 11.5px;
  }
  .revert:hover { border-color: var(--orange); color: var(--orange); }
  .revert .nf { font-size: 11px; }
  .done { flex: none; color: var(--text-muted); font-size: 11px; font-style: italic; }
  footer {
    padding: 8px 14px; border-top: 1px solid var(--border-muted);
    color: var(--text-muted); font-size: 10.5px;
  }
  footer kbd { font-family: var(--mono); background: var(--bg-app); border: 1px solid var(--border); border-radius: 3px; padding: 0 4px; }
</style>
