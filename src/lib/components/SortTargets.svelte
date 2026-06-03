<script lang="ts">
  import { session } from "$lib/session.svelte";
  import { I } from "$lib/icons";

  // Only render slots that are actually bound (trash is always present). The
  // pane grows downward as binds are added, leaving the Navigator more room.
  const targets = $derived(session.sortedTargets);
  const hasUserBinds = $derived(targets.some((d) => !d.isTrash));
</script>

<section class="pane">
  <div class="title">「 Sort Targets 」</div>
  <div class="list">
    {#each targets as dest, i (dest.path)}
      {#if dest.isTrash && i > 0}<div class="sep" aria-hidden="true"></div>{/if}
      <div class="slot" class:trash={dest.isTrash}>
        <span class="key" class:trashkey={dest.isTrash}>{dest.hotkey}</span>
        <span class="nf icon">{dest.isTrash ? I.trash : I.folder}</span>
        <button class="label" title={dest.path} disabled={!session.current}
          onclick={() => session.moveToDest(dest)}>{dest.label}</button>
        <span class="count">({dest.mediaCount})</span>
        {#if !dest.isTrash}
          <button class="unbind nf" title="Unbind {dest.hotkey}" onclick={() => session.unbind(dest.hotkey!)}>{I.close}</button>
        {:else}
          <span class="spacer"></span>
        {/if}
      </div>
    {/each}
  </div>
  {#if !hasUserBinds}
    <div class="hint">
      <span class="nf">{I.tag}</span>
      bind a folder: focus the Navigator, highlight it, press ⇧1-9
    </div>
  {/if}
</section>

<style>
  .pane {
    display: flex; flex-direction: column; min-height: 0;
    background: var(--bg-panel); border: 1px solid var(--border);
    border-radius: var(--radius); overflow: hidden;
  }
  .title { padding: 8px 12px 4px; color: var(--purple); font-weight: 600; font-family: var(--sans); }
  .list { overflow-y: auto; padding: 4px 6px 6px; display: flex; flex-direction: column; gap: 1px; }
  .slot {
    display: grid;
    grid-template-columns: 22px 16px 1fr auto 18px;
    gap: 8px;
    align-items: center;
    padding: 3px 6px;
    border-radius: var(--radius-sm);
    font-size: 12.5px;
  }
  .slot:hover { background: var(--bg-panel-alt); }
  .sep { border-top: 1px dashed var(--border); margin: 3px 8px; }
  .key {
    display: inline-grid; place-items: center;
    width: 20px; height: 20px; border-radius: var(--radius-sm);
    background: var(--bg-chip); border: 1px solid var(--border);
    color: var(--green); font-family: var(--mono); font-weight: 700; font-size: 11px;
  }
  .trashkey { color: var(--red); }
  .icon { color: var(--yellow); font-size: 12px; text-align: center; }
  .trash .icon { color: var(--red); }
  .label {
    justify-self: start;
    border: none; background: transparent; color: var(--text-secondary);
    text-align: left; cursor: pointer; padding: 0; font-size: 12.5px;
    overflow: hidden; white-space: nowrap; text-overflow: ellipsis; max-width: 100%;
  }
  .slot:hover .label { color: var(--text-primary); }
  .label:disabled { cursor: default; }
  .trash .label { color: var(--text-muted); }
  .count { color: var(--text-muted); font-variant-numeric: tabular-nums; }
  .unbind {
    border: none; background: transparent; color: var(--text-muted);
    cursor: pointer; font-size: 12px; line-height: 1; opacity: 0.5; padding: 0;
    transition: color 0.1s, opacity 0.1s;
  }
  .slot:hover .unbind { opacity: 0.85; }
  .unbind:hover { color: var(--red); opacity: 1; }
  .spacer { width: 18px; }
  .hint {
    display: flex; align-items: center; gap: 6px;
    padding: 7px 12px; border-top: 1px solid var(--border-muted);
    color: var(--text-muted); font-size: 10.5px;
  }
  .hint .nf { color: var(--purple); }
</style>
