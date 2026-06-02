<script lang="ts">
  import { session } from "$lib/session.svelte";

  const mainSlots = ["1", "2", "3", "4", "5", "6", "7", "8", "9"];
  const extraSlots = ["0", "-", "="];
</script>

<section class="pane">
  <div class="title">「 Sort Targets 」</div>
  <div class="list">
    {#each mainSlots as key (key)}
      {@const dest = session.destForHotkey(key)}
      <div class="slot" class:bound={!!dest}>
        <span class="key">{key}</span>
        {#if dest}
          <button class="label" title={dest.path} disabled={!session.current}
            onclick={() => session.moveToDest(dest)}>{dest.label}</button>
          <span class="count">({dest.mediaCount})</span>
          <button class="unbind" title="Unbind {key}" onclick={() => session.unbind(key)}>×</button>
        {:else}
          <span class="undef">(undefined)</span>
        {/if}
      </div>
    {/each}

    <div class="divider"></div>

    {#each extraSlots as key (key)}
      {@const dest = session.destForHotkey(key)}
      <div class="slot" class:bound={!!dest} class:trash={dest?.isTrash}>
        <span class="key" class:trashkey={dest?.isTrash}>{key}</span>
        {#if dest}
          <button class="label" title={dest.path} disabled={!session.current}
            onclick={() => session.moveToDest(dest)}>{dest.label}</button>
          <span class="count">({dest.mediaCount})</span>
          {#if !dest.isTrash}
            <button class="unbind" title="Unbind {key}" onclick={() => session.unbind(key)}>×</button>
          {/if}
        {:else}
          <span class="undef">(undefined)</span>
        {/if}
      </div>
    {/each}
  </div>
  <div class="hint">bind: focus Navigator, highlight a folder, press ⇧1-9</div>
</section>

<style>
  .pane {
    display: flex; flex-direction: column; min-height: 0;
    background: var(--bg-panel); border: 1px solid var(--border);
    border-radius: var(--radius); overflow: hidden;
  }
  .title { padding: 8px 12px 4px; color: var(--text-primary); font-weight: 600; }
  .list { overflow-y: auto; padding: 4px 6px; display: flex; flex-direction: column; gap: 1px; }
  .slot {
    display: grid;
    grid-template-columns: 22px 1fr auto auto;
    gap: 8px;
    align-items: center;
    padding: 3px 6px;
    border-radius: var(--radius-sm);
    font-size: 12.5px;
  }
  .slot:hover { background: var(--bg-panel-alt); }
  .key {
    display: inline-grid; place-items: center;
    width: 20px; height: 20px; border-radius: var(--radius-sm);
    background: var(--bg-chip); border: 1px solid var(--border);
    color: var(--text-muted); font-family: var(--mono); font-weight: 700; font-size: 11px;
  }
  .bound .key { color: var(--green); }
  .trashkey { color: var(--red) !important; }
  .label {
    justify-self: start;
    border: none; background: transparent; color: var(--text-secondary);
    text-align: left; cursor: pointer; padding: 0; font-size: 12.5px;
    overflow: hidden; white-space: nowrap; text-overflow: ellipsis; max-width: 100%;
  }
  .slot:hover .label { color: var(--text-primary); }
  .label:disabled { cursor: default; }
  .trash .label { color: var(--text-muted); }
  .undef { justify-self: start; color: var(--text-muted); opacity: 0.6; }
  .count { color: var(--text-muted); font-variant-numeric: tabular-nums; }
  .unbind {
    border: none; background: transparent; color: var(--text-muted);
    cursor: pointer; font-size: 13px; line-height: 1; opacity: 0; padding: 0 2px;
  }
  .slot:hover .unbind { opacity: 1; }
  .unbind:hover { color: var(--red); }
  .divider { height: 1px; background: var(--border-muted); margin: 4px 6px; }
  .hint {
    padding: 5px 12px; border-top: 1px solid var(--border-muted);
    color: var(--text-muted); font-size: 10.5px;
  }
</style>
