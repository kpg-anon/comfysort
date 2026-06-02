<script lang="ts">
  import { session } from "$lib/session.svelte";
  import { I } from "$lib/icons";

  let newName = $state("");
  let inputEl: HTMLInputElement | undefined = $state();
  let searchEl: HTMLInputElement | undefined = $state();
  let listEl: HTMLDivElement | undefined = $state();

  const focused = $derived(session.focus === "navigator");
  const hasParent = $derived(session.navHasParent);

  const crumbs = $derived.by(() => {
    const root = session.output ?? "";
    const rel = session.nav?.rel ?? "";
    const segs = rel.split("/").filter(Boolean);
    return segs.map((name, i) => ({ name, path: root + "/" + segs.slice(0, i + 1).join("/") }));
  });

  function relOf(path: string): string {
    const root = (session.output ?? "").replace(/\\/g, "/");
    const p = path.replace(/\\/g, "/");
    return p.startsWith(root) ? p.slice(root.length).replace(/^\/+/, "") : p;
  }

  // Focus the search box when search mode opens.
  $effect(() => {
    if (session.searching) queueMicrotask(() => searchEl?.focus());
  });
  // Focus the new-folder input when the prompt opens (＋ button or Ctrl+N).
  $effect(() => {
    if (session.creatingFolder) {
      newName = "";
      queueMicrotask(() => inputEl?.focus());
    }
  });
  // Scroll the keyboard cursor row into view (tree or search).
  $effect(() => {
    const _ = session.navCursor + session.searchCursor;
    listEl?.querySelector(".row.cursor")?.scrollIntoView({ block: "nearest" });
  });

  function rowIndex(folderIdx: number): number {
    return (hasParent ? 1 : 0) + folderIdx;
  }
  async function commitCreate() {
    const name = newName.trim();
    newName = "";
    if (name) await session.createFolderHere(name);
    else session.cancelNewFolder();
  }
  function onCreateKey(e: KeyboardEvent) {
    e.stopPropagation();
    if (e.key === "Enter") commitCreate();
    else if (e.key === "Escape") { session.cancelNewFolder(); newName = ""; }
  }
  function onSearchKey(e: KeyboardEvent) {
    e.stopPropagation();
    const el = e.currentTarget as HTMLInputElement;
    switch (e.key) {
      case "ArrowDown": e.preventDefault(); session.searchDown(); break;
      case "ArrowUp": e.preventDefault(); session.searchUp(); break;
      case "ArrowRight":
        // → drills into the highlighted match — but only when the caret is at
        // the end of the query, so mid-text editing still moves the cursor.
        if (el.selectionStart === el.value.length && el.selectionEnd === el.value.length) {
          e.preventDefault();
          session.searchDrill();
        }
        break;
      case "Enter": e.preventDefault(); session.searchMove(); break;
      case "Escape": e.preventDefault(); session.exitSearch(); break;
    }
  }
</script>

<section class="pane" class:focused>
  <div class="head">
    <div class="title">「 Navigator{focused ? " *" : ""} 」</div>
    <div class="actions">
      {#if session.searching}
        <button class="hbtn nf" title="Close search (Esc)" onclick={() => session.exitSearch()}>{I.close}</button>
      {:else}
        <button class="hbtn nf" title="Fuzzy search folders (/)" onclick={() => session.startSearch()}>{I.search}</button>
        <button class="hbtn nf" title="New folder here (Ctrl+N)" onclick={() => session.startNewFolder()}>{I.plus}</button>
      {/if}
    </div>
  </div>

  {#if session.searching}
    <div class="searchbar">
      <span class="sicon nf">{I.search}</span>
      <input
        bind:this={searchEl}
        value={session.searchQuery}
        oninput={(e) => session.updateSearch(e.currentTarget.value)}
        onkeydown={onSearchKey}
        placeholder="fuzzy search folders…"
      />
    </div>
  {:else}
    <div class="crumbs">
      <button class="crumb root" onclick={() => session.navHome()}>./</button>
      {#each crumbs as c}
        <span class="sep">›</span>
        <button class="crumb" onclick={() => session.loadFolders(c.path)}>{c.name}</button>
      {/each}
    </div>
  {/if}

  <div class="list" bind:this={listEl}>
    {#if session.searching}
      {#each session.searchResults as r, i (r.path)}
        <div class="row" class:cursor={i === session.searchCursor}>
          <button class="drill" title={r.path} onclick={() => session.searchDrill(r)}>
            <span class="nf icon">{I.folder}</span>
            <span class="rname">
              <span class="name">{r.name}</span>
              <span class="rel">{relOf(r.path)}</span>
            </span>
            <span class="count">({r.mediaCount})</span>
          </button>
          <div class="acts">
            <button class="act move nf" title="Move here (Enter)" disabled={!session.current}
              onclick={() => session.moveInto(r)}>{I.arrowRight}</button>
            <button class="act copy nf" title="Copy here" disabled={!session.current}
              onclick={() => session.copyInto(r)}>{I.copy}</button>
          </div>
        </div>
      {/each}
      {#if session.searchQuery && session.searchResults.length === 0}
        <div class="empty">No folders match “{session.searchQuery}”.</div>
      {:else if !session.searchQuery}
        <div class="empty">Type to fuzzy-search every folder under the root.</div>
      {/if}
    {:else}
      {#if hasParent}
        <button class="row up" class:cursor={focused && session.navCursor === 0}
          onclick={() => { session.focusNavigator(); session.navAscend(); }}>
          <span class="nf icon">{I.levelUp}</span><span class="name">..</span>
        </button>
      {/if}
      {#each session.nav?.folders ?? [] as folder, fi (folder.path)}
        <div class="row" class:cursor={focused && session.navCursor === rowIndex(fi)}>
          <button class="drill" title={folder.path}
            onclick={() => { session.focusNavigator(); session.navCursor = rowIndex(fi); session.loadFolders(folder.path); }}>
            <span class="nf icon">{I.folder}</span>
            <span class="name">{folder.name}</span>
            {#if folder.subfolderCount > 0}<span class="sub">{folder.subfolderCount}▸</span>{/if}
            <span class="count">({folder.mediaCount})</span>
          </button>
          <div class="acts">
            <button class="act move nf" title="Move file here (Enter)" disabled={!session.current}
              onclick={() => session.moveInto(folder)}>{I.arrowRight}</button>
            <button class="act copy nf" title="Copy file here (Shift+D)" disabled={!session.current}
              onclick={() => session.copyInto(folder)}>{I.copy}</button>
          </div>
        </div>
      {/each}
      {#if session.creatingFolder}
        <div class="row creating">
          <span class="nf icon">{I.folder}</span>
          <input bind:this={inputEl} bind:value={newName} placeholder="new folder name…"
            onkeydown={onCreateKey} onblur={commitCreate} />
        </div>
      {/if}
      {#if (session.nav?.folders.length ?? 0) === 0 && !session.creatingFolder && !hasParent}
        <div class="empty">No subfolders here. Use ＋ to make one, or / to search.</div>
      {/if}
    {/if}
  </div>
</section>

<style>
  .pane {
    display: flex; flex-direction: column; min-height: 0;
    background: var(--bg-panel); border: 1px solid var(--border);
    border-radius: var(--radius); overflow: hidden;
  }
  .pane.focused { border-color: var(--purple); }
  .head { display: flex; align-items: center; justify-content: space-between; padding: 8px 10px 2px 12px; }
  .title { color: var(--purple); font-weight: 600; }
  .actions { display: flex; gap: 6px; }
  .hbtn {
    display: grid; place-items: center;
    border: 1px solid var(--border); background: var(--bg-chip); color: var(--text-secondary);
    border-radius: var(--radius-sm); width: 22px; height: 22px; cursor: pointer; font-size: 12px;
    padding: 0;
  }
  .hbtn:hover { border-color: var(--purple); color: var(--purple); }
  .crumbs {
    display: flex; align-items: center; flex-wrap: wrap; gap: 2px;
    padding: 2px 12px 6px; border-bottom: 1px solid var(--border-muted);
    font-family: var(--mono); font-size: 11px;
  }
  .crumb { border: none; background: transparent; color: var(--purple); cursor: pointer; padding: 1px 3px; border-radius: 3px; }
  .crumb:hover { background: var(--bg-panel-alt); }
  .crumb.root { color: var(--text-secondary); }
  .sep { color: var(--text-muted); }
  .searchbar {
    display: flex; align-items: center; gap: 6px; margin: 2px 10px 6px;
    border-bottom: 1px solid var(--border-muted); padding-bottom: 6px;
  }
  .sicon { color: var(--purple); font-family: var(--mono); font-weight: 700; }
  .searchbar input {
    flex: 1; background: var(--bg-app); border: 1px solid var(--purple);
    border-radius: var(--radius-sm); color: var(--text-primary); padding: 4px 8px;
    font-size: 12.5px; outline: none;
  }
  .list { flex: 1; min-height: 0; overflow-y: auto; padding: 4px 6px; }
  .row { display: flex; align-items: center; border-radius: var(--radius-sm); }
  .row:hover { background: var(--bg-panel-alt); }
  .row.cursor { background: var(--bg-selected-active); }
  .row.cursor .name { color: var(--text-primary); }
  .drill {
    flex: 1; min-width: 0; display: flex; align-items: center; gap: 7px;
    border: none; background: transparent; color: var(--text-secondary);
    text-align: left; padding: 5px 6px; cursor: pointer; font-size: 12.5px;
  }
  .row:hover .drill { color: var(--text-primary); }
  .icon { color: var(--yellow); flex: none; width: 14px; text-align: center; }
  /* the ".." ascend row is a bare button — strip the default button chrome so it
     reads as a plain list entry like the folder rows (yazi-style). */
  .up {
    width: 100%;
    border: none;
    background: transparent;
    cursor: pointer;
    text-align: left;
    gap: 7px;
    padding: 5px 6px;
    color: var(--text-secondary);
    font-size: 12.5px;
  }
  .up:hover { color: var(--text-primary); }
  .up .icon { color: var(--text-muted); }
  .name { overflow: hidden; white-space: nowrap; text-overflow: ellipsis; }
  .rname { display: flex; flex-direction: column; min-width: 0; flex: 1; }
  .rname .name { flex: none; }
  .rel { font-size: 10px; color: var(--text-muted); overflow: hidden; white-space: nowrap; text-overflow: ellipsis; font-family: var(--mono); }
  .sub { color: var(--text-muted); font-size: 10px; }
  .count { color: var(--text-muted); font-variant-numeric: tabular-nums; flex: none; }
  .acts { display: flex; gap: 3px; padding-right: 5px; opacity: 0; }
  .row:hover .acts, .row.cursor .acts { opacity: 1; }
  .act {
    border: 1px solid var(--border); background: var(--bg-chip);
    border-radius: var(--radius-sm); width: 22px; height: 22px; cursor: pointer; font-size: 12px; line-height: 1;
  }
  .act.move { color: var(--green); }
  .act.move:hover:not(:disabled) { border-color: var(--green); }
  .act.copy { color: var(--cyan); }
  .act.copy:hover:not(:disabled) { border-color: var(--cyan); }
  .act:disabled { opacity: 0.35; cursor: default; }
  .creating input {
    flex: 1; margin: 3px 0; background: var(--bg-app);
    border: 1px solid var(--purple); border-radius: var(--radius-sm);
    color: var(--text-primary); padding: 4px 7px; font-family: var(--mono); font-size: 12px; outline: none;
  }
  .empty { padding: 16px 12px; color: var(--text-muted); text-align: center; font-size: 12px; }
</style>
