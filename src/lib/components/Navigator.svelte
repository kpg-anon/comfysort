<script lang="ts">
  import { session } from "$lib/session.svelte";

  let creating = $state(false);
  let newName = $state("");
  let inputEl: HTMLInputElement | undefined = $state();

  // Breadcrumb segments with the path to jump to for each.
  const crumbs = $derived.by(() => {
    const root = session.output ?? "";
    const rel = session.nav?.rel ?? "";
    const segs = rel.split("/").filter(Boolean);
    return segs.map((name, i) => ({
      name,
      path: root + "/" + segs.slice(0, i + 1).join("/"),
    }));
  });

  function startCreate() {
    creating = true;
    newName = "";
    queueMicrotask(() => inputEl?.focus());
  }
  async function commitCreate() {
    const name = newName.trim();
    if (name) await session.createFolderHere(name);
    creating = false;
    newName = "";
  }
  function onCreateKey(e: KeyboardEvent) {
    e.stopPropagation();
    if (e.key === "Enter") commitCreate();
    else if (e.key === "Escape") {
      creating = false;
      newName = "";
    }
  }
</script>

<section class="pane">
  <div class="head">
    <div class="title">「 Navigator 」</div>
    <button class="new" title="New folder here" onclick={startCreate}>＋</button>
  </div>

  <div class="crumbs">
    <button class="crumb root" onclick={() => session.navHome()}>./</button>
    {#each crumbs as c}
      <span class="sep">›</span>
      <button class="crumb" onclick={() => session.loadFolders(c.path)}>{c.name}</button>
    {/each}
  </div>

  <div class="list">
    {#if session.nav?.parent}
      <button class="row up" onclick={() => session.navUp()}>
        <span class="icon">↩</span><span class="name">..</span>
      </button>
    {/if}

    {#each session.nav?.folders ?? [] as folder (folder.path)}
      <div class="row">
        <button class="drill" onclick={() => session.drillInto(folder)} title={folder.path}>
          <span class="icon"></span>
          <span class="name">{folder.name}</span>
          {#if folder.subfolderCount > 0}<span class="sub">{folder.subfolderCount}▸</span>{/if}
          <span class="count">({folder.mediaCount})</span>
        </button>
        <div class="acts">
          <button
            class="act move"
            title="Move file here"
            disabled={!session.current}
            onclick={() => session.moveInto(folder)}>→</button>
          <button
            class="act copy"
            title="Copy file here"
            disabled={!session.current}
            onclick={() => session.copyInto(folder)}>⧉</button>
        </div>
      </div>
    {/each}

    {#if creating}
      <div class="row creating">
        <span class="icon"></span>
        <input
          bind:this={inputEl}
          bind:value={newName}
          placeholder="new folder name…"
          onkeydown={onCreateKey}
          onblur={commitCreate}
        />
      </div>
    {/if}

    {#if (session.nav?.folders.length ?? 0) === 0 && !creating && !session.nav?.parent}
      <div class="empty">No subfolders here. Use ＋ to make one.</div>
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
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 10px 2px 12px;
  }
  .title { color: var(--text-primary); font-weight: 600; }
  .new {
    border: 1px solid var(--border);
    background: var(--bg-chip);
    color: var(--green);
    border-radius: var(--radius-sm);
    width: 22px;
    height: 22px;
    cursor: pointer;
    font-size: 14px;
    line-height: 1;
  }
  .new:hover { border-color: var(--green); }
  .crumbs {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 2px;
    padding: 2px 12px 6px;
    border-bottom: 1px solid var(--border-muted);
    font-family: var(--mono);
    font-size: 11px;
  }
  .crumb {
    border: none;
    background: transparent;
    color: var(--purple);
    cursor: pointer;
    padding: 1px 3px;
    border-radius: 3px;
  }
  .crumb:hover { background: var(--bg-panel-alt); }
  .crumb.root { color: var(--text-secondary); }
  .sep { color: var(--text-muted); }
  .list { flex: 1; min-height: 0; overflow-y: auto; padding: 4px 6px; }
  .row {
    display: flex;
    align-items: center;
    border-radius: var(--radius-sm);
  }
  .row:hover { background: var(--bg-panel-alt); }
  .drill {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 7px;
    border: none;
    background: transparent;
    color: var(--text-secondary);
    text-align: left;
    padding: 5px 6px;
    cursor: pointer;
    font-size: 12.5px;
  }
  .row:hover .drill { color: var(--text-primary); }
  .icon { color: var(--yellow); flex: none; width: 14px; text-align: center; }
  .up .icon { color: var(--text-muted); }
  .name { overflow: hidden; white-space: nowrap; text-overflow: ellipsis; flex: 1; }
  .sub { color: var(--text-muted); font-size: 10px; }
  .count { color: var(--text-muted); font-variant-numeric: tabular-nums; }
  .acts { display: flex; gap: 3px; padding-right: 5px; opacity: 0; }
  .row:hover .acts { opacity: 1; }
  .act {
    border: 1px solid var(--border);
    background: var(--bg-chip);
    border-radius: var(--radius-sm);
    width: 22px;
    height: 22px;
    cursor: pointer;
    font-size: 12px;
    line-height: 1;
  }
  .act.move { color: var(--green); }
  .act.move:hover:not(:disabled) { border-color: var(--green); }
  .act.copy { color: var(--cyan); }
  .act.copy:hover:not(:disabled) { border-color: var(--cyan); }
  .act:disabled { opacity: 0.35; cursor: default; }
  .creating input {
    flex: 1;
    margin: 3px 0;
    background: var(--bg-app);
    border: 1px solid var(--purple);
    border-radius: var(--radius-sm);
    color: var(--text-primary);
    padding: 4px 7px;
    font-family: var(--mono);
    font-size: 12px;
    outline: none;
  }
  .empty { padding: 16px 12px; color: var(--text-muted); text-align: center; font-size: 12px; }
</style>
