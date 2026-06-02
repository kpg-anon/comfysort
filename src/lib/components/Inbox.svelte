<script lang="ts">
  import { session } from "$lib/session.svelte";
  import { humanSize, extOf } from "$lib/api";

  let listEl: HTMLDivElement | undefined = $state();

  // Keep the selected row visible as the cursor moves.
  $effect(() => {
    const _ = session.cursor;
    listEl?.querySelector(".row.active")?.scrollIntoView({ block: "nearest" });
  });

  const totalBytes = $derived(session.inbox.reduce((a, i) => a + i.sizeBytes, 0));
</script>

<section class="pane">
  <div class="title">「 Inbox 」</div>
  <div class="cols">
    <span>Name</span><span class="r">Size</span><span class="r">Type</span>
  </div>
  <div class="list" bind:this={listEl}>
    {#each session.inbox as item, i (item.path)}
      <button
        class="row"
        class:active={i === session.cursor}
        onclick={() => session.select(i)}
      >
        <span class="cursor">{i === session.cursor ? "›" : ""}</span>
        <span class="name" title={item.fileName}>{item.fileName}</span>
        <span class="r size">{humanSize(item.sizeBytes)}</span>
        <span class="r"><span class="chip ext-{extOf(item.fileName) || 'other'}"
          >{extOf(item.fileName).toUpperCase() || "?"}</span></span>
      </button>
    {/each}
    {#if session.inbox.length === 0}
      <div class="empty">Inbox is empty — everything sorted.</div>
    {/if}
  </div>
  <div class="footer">
    <span>{session.total} items</span>
    <span>{humanSize(totalBytes)}</span>
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
  .title {
    padding: 8px 12px 4px;
    color: var(--text-primary);
    font-weight: 600;
  }
  .cols {
    display: grid;
    grid-template-columns: 16px 1fr auto auto;
    gap: 8px;
    padding: 2px 12px 6px;
    color: var(--text-muted);
    font-size: 11px;
    border-bottom: 1px solid var(--border-muted);
  }
  .cols .r { text-align: right; }
  .list {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 4px 6px;
  }
  .row {
    display: grid;
    grid-template-columns: 16px 1fr auto auto;
    gap: 8px;
    align-items: center;
    width: 100%;
    border: none;
    background: transparent;
    color: var(--text-secondary);
    text-align: left;
    padding: 4px 6px;
    border-radius: var(--radius-sm);
    cursor: pointer;
    font-size: 12.5px;
  }
  .row:hover { background: var(--bg-panel-alt); }
  .row.active {
    background: var(--bg-selected);
    color: var(--text-primary);
  }
  .cursor { color: var(--green); font-weight: 700; }
  .name {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .r { text-align: right; }
  .size { color: var(--text-muted); font-variant-numeric: tabular-nums; }
  .empty {
    padding: 24px 12px;
    color: var(--text-muted);
    text-align: center;
  }
  .footer {
    display: flex;
    justify-content: space-between;
    padding: 6px 12px;
    border-top: 1px solid var(--border-muted);
    color: var(--text-muted);
    font-size: 11px;
    font-variant-numeric: tabular-nums;
  }
</style>
