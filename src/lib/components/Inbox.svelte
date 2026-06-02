<script lang="ts">
  import { session } from "$lib/session.svelte";
  import { humanSize, extOf } from "$lib/api";

  let listEl: HTMLDivElement | undefined = $state();
  const focused = $derived(session.focus === "inbox");

  // Keep the selected row visible as the cursor moves.
  $effect(() => {
    const _ = session.cursor;
    listEl?.querySelector(".row.active")?.scrollIntoView({ block: "nearest" });
  });

  const arrow = $derived(session.sortOrder === "desc" ? "↓" : "↑");
</script>

<section class="pane" class:focused>
  <div class="title">
    <span>「 Inbox{focused ? " *" : ""} 」</span>
    <span class="modes">sort {session.sortField}{arrow} · filter {session.filter}</span>
  </div>
  <div class="cols">
    <span>Name</span><span class="r">Size</span><span class="r">Type</span>
  </div>
  <div class="list" bind:this={listEl}>
    {#each session.view as item, i (item.path)}
      <button
        class="row"
        class:active={i === session.cursor}
        onclick={() => { session.focusInbox(); session.select(i); }}
      >
        <span class="cursor">{i === session.cursor ? "›" : ""}</span>
        <span class="name" title={item.fileName}>{item.fileName}</span>
        <span class="r size">{humanSize(item.sizeBytes)}</span>
        <span class="r"><span class="chip ext-{extOf(item.fileName) || 'other'}"
          >{extOf(item.fileName).toUpperCase() || "?"}</span></span>
      </button>
    {/each}
    {#if session.view.length === 0}
      <div class="empty">
        {session.total === 0 && session.filter !== "all"
          ? `No ${session.filter} here — press f to change filter.`
          : "Inbox is empty — everything sorted."}
      </div>
    {/if}
  </div>
  <div class="footer">
    <span>{session.total} items</span>
    <span>{humanSize(session.viewBytes)}</span>
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
  .pane.focused { border-color: var(--purple); }
  .title {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 8px;
    padding: 8px 12px 4px;
    color: var(--text-primary);
    font-weight: 600;
  }
  .focused .title > span:first-child { color: var(--purple); }
  .modes { font-size: 10.5px; font-weight: 400; color: var(--text-muted); font-family: var(--mono); }
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
  .list { flex: 1; min-height: 0; overflow-y: auto; padding: 4px 6px; }
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
  .row.active { background: var(--bg-selected); color: var(--text-primary); }
  .focused .row.active { box-shadow: inset 2px 0 0 var(--green); }
  .cursor { color: var(--green); font-weight: 700; }
  .name { overflow: hidden; white-space: nowrap; text-overflow: ellipsis; }
  .r { text-align: right; }
  .size { color: var(--text-muted); font-variant-numeric: tabular-nums; }
  .empty { padding: 24px 12px; color: var(--text-muted); text-align: center; }
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
