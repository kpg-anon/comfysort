<script lang="ts">
  import { session } from "$lib/session.svelte";
  import { I } from "$lib/icons";

  const ctx = $derived(session.ctx);
  // Clamp the menu inside the viewport.
  const left = $derived(ctx ? Math.min(ctx.x, window.innerWidth - 230) : 0);
  const top = $derived(ctx ? Math.min(ctx.y, window.innerHeight - 200) : 0);
</script>

{#if ctx}
  <!-- full-screen catcher closes the menu on any outside click / right-click -->
  <div
    class="cm-catch"
    role="presentation"
    onclick={() => session.closeContext()}
    oncontextmenu={(e) => { e.preventDefault(); session.closeContext(); }}
  ></div>
  <div class="cm" style="left:{left}px; top:{top}px">
    <div class="cm-name" title={ctx.item.path}>{ctx.item.fileName}</div>
    <div class="cm-sep"></div>
    <button class="cmi" onclick={() => session.openInDefault(ctx.item.path)}>
      <span class="nf">{I.eye}</span> Open in default viewer
    </button>
    <button class="cmi" onclick={() => session.revealInExplorer(ctx.item.path)}>
      <span class="nf">{I.folderOpen}</span> Reveal in file explorer
    </button>
    <div class="cm-sep"></div>
    <button class="cmi danger" onclick={() => session.trashPath(ctx.item.path)}>
      <span class="nf">{I.trash}</span> Move to trash
    </button>
    <div class="cm-sep"></div>
    <button class="cmi" onclick={() => session.refreshInbox()}>
      <span class="nf">{I.refresh}</span> Refresh inbox
    </button>
  </div>
{/if}

<style>
  .cm-catch { position: fixed; inset: 0; z-index: 70; }
  .cm {
    position: fixed;
    z-index: 71;
    min-width: 214px;
    background: var(--bg-panel);
    border: 1px solid var(--border);
    border-radius: 9px;
    padding: 5px;
    box-shadow: 0 14px 40px rgba(0, 0, 0, 0.5);
    animation: cmpop 0.1s ease-out;
  }
  @keyframes cmpop { from { opacity: 0; transform: translateY(-3px); } }
  .cm-name {
    padding: 5px 9px 6px;
    font-size: 11.5px;
    color: var(--text-muted);
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    max-width: 260px;
  }
  .cm-sep { height: 1px; background: var(--border-muted); margin: 4px 4px; }
  .cmi {
    display: flex;
    align-items: center;
    gap: 9px;
    width: 100%;
    border: none;
    background: transparent;
    color: var(--text-secondary);
    text-align: left;
    padding: 7px 9px;
    border-radius: var(--radius-sm);
    cursor: pointer;
    font-size: 12.5px;
  }
  .cmi:hover { background: var(--bg-panel-alt); color: var(--text-primary); }
  .cmi .nf { width: 15px; text-align: center; color: var(--text-muted); font-size: 12px; }
  .cmi:hover .nf { color: var(--cyan); }
  .cmi.danger:hover { color: var(--red); }
  .cmi.danger:hover .nf { color: var(--red); }
</style>
