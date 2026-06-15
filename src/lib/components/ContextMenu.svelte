<script lang="ts">
  import { session } from "$lib/session.svelte";
  import { I } from "$lib/icons";

  const ctx = $derived(session.ctx);

  // Place the menu at the cursor, clamped inside the viewport against the menu's
  // *measured* size so it never gets cut off near a screen edge.
  function positionMenu(node: HTMLElement, pos: { x: number; y: number }) {
    const place = (p: { x: number; y: number }) => {
      const margin = 6;
      const { width, height } = node.getBoundingClientRect();
      const left = Math.max(margin, Math.min(p.x, window.innerWidth - width - margin));
      const top = Math.max(margin, Math.min(p.y, window.innerHeight - height - margin));
      node.style.left = `${left}px`;
      node.style.top = `${top}px`;
    };
    place(pos);
    return { update: place };
  }
</script>

{#if ctx}
  <!-- full-screen catcher closes the menu on any outside click / right-click -->
  <div
    class="cm-catch"
    role="presentation"
    onclick={() => session.closeContext()}
    oncontextmenu={(e) => { e.preventDefault(); session.closeContext(); }}
  ></div>
  <div class="cm" use:positionMenu={{ x: ctx.x, y: ctx.y }}>
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
