<script lang="ts">
  import { session } from "$lib/session.svelte";
</script>

<section class="pane">
  <div class="title">「 Sort Targets 」</div>
  <div class="list">
    {#each session.sortedTargets as dest (dest.path)}
      <button
        class="target"
        class:trash={dest.isTrash}
        onclick={() => session.moveToDest(dest)}
        disabled={!session.current}
        title={dest.path}
      >
        <span class="key" class:trashkey={dest.isTrash}>{dest.hotkey}</span>
        <span class="label">{dest.label}</span>
        <span class="count">({dest.mediaCount})</span>
      </button>
    {/each}
    {#if session.sortedTargets.length === 0}
      <div class="empty">No destination folders found under the output root.</div>
    {/if}
  </div>
</section>

<style>
  .pane {
    display: flex;
    flex-direction: column;
    min-height: 0;
    background: var(--bg-panel);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
  }
  .title { padding: 8px 12px 4px; color: var(--text-primary); font-weight: 600; }
  .list { overflow-y: auto; padding: 4px 6px; display: flex; flex-direction: column; gap: 2px; }
  .target {
    display: grid;
    grid-template-columns: 22px 1fr auto;
    gap: 8px;
    align-items: center;
    border: none;
    background: transparent;
    color: var(--text-secondary);
    text-align: left;
    padding: 5px 6px;
    border-radius: var(--radius-sm);
    cursor: pointer;
    font-size: 12.5px;
  }
  .target:hover:not(:disabled) { background: var(--bg-panel-alt); color: var(--text-primary); }
  .target:disabled { opacity: 0.45; cursor: default; }
  .key {
    display: inline-grid;
    place-items: center;
    width: 20px;
    height: 20px;
    border-radius: var(--radius-sm);
    background: var(--bg-chip);
    border: 1px solid var(--border);
    color: var(--green);
    font-family: var(--mono);
    font-weight: 700;
    font-size: 11px;
  }
  .trashkey { color: var(--red); }
  .label { overflow: hidden; white-space: nowrap; text-overflow: ellipsis; }
  .trash .label { color: var(--text-muted); }
  .count { color: var(--text-muted); font-variant-numeric: tabular-nums; }
</style>
