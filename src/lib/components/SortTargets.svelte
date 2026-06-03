<script lang="ts">
  import { session } from "$lib/session.svelte";
  import { settings } from "$lib/settings.svelte";
  import { I } from "$lib/icons";
  import type { Destination } from "$lib/api";

  // Numbered binds (1–9) sit on top; the =, − and trash slots form a fixed group
  // below a dashed separator. = defaults to an "Archive" slot, − to unused.
  const numbered = $derived(
    session.sortedTargets.filter((d) => d.hotkey && d.hotkey >= "1" && d.hotkey <= "9"),
  );
  const trash = $derived(session.sortedTargets.find((d) => d.isTrash) ?? null);
  const eqDest = $derived(session.destForHotkey("=") ?? null);
  const dashDest = $derived(session.destForHotkey("-") ?? null);
  const hasUserBinds = $derived(numbered.length > 0 || !!eqDest || !!dashDest);
</script>

{#snippet boundSlot(dest: Destination, special: boolean)}
  <div class="slot">
    <span class="key" class:special>{dest.hotkey}</span>
    <span class="nf icon">{I.folder}</span>
    <button class="label" title={dest.path} disabled={!session.current}
      onclick={() => session.moveToDest(dest)}>{dest.label}</button>
    <span class="count">({dest.mediaCount})</span>
    <button class="unbind nf" title="Unbind {dest.hotkey}" onclick={() => session.unbind(dest.hotkey!)}>{I.close}</button>
  </div>
{/snippet}

{#snippet specialSlot(key: string, dest: Destination | null, fallback: string)}
  {#if dest}
    {@render boundSlot(dest, true)}
  {:else}
    <button class="slot placeholder" title="Set in the sort-target editor" onclick={() => settings.openTargets()}>
      <span class="key special">{key}</span>
      <span class="nf icon">{I.folder}</span>
      <span class="label muted">{fallback}</span>
      <span class="count"></span>
      <span class="spacer"></span>
    </button>
  {/if}
{/snippet}

<section class="pane">
  <div class="title">「 Sort Targets 」</div>
  <div class="list">
    {#each numbered as dest (dest.path)}
      {@render boundSlot(dest, false)}
    {/each}

    <div class="sep" aria-hidden="true"></div>

    {#if trash}
      <div class="slot trash">
        <span class="key trashkey">{trash.hotkey}</span>
        <span class="nf icon">{I.trash}</span>
        <button class="label" title={trash.path} disabled={!session.current}
          onclick={() => session.moveToDest(trash)}>{trash.label}</button>
        <span class="count">({trash.mediaCount})</span>
        <span class="spacer"></span>
      </div>
    {/if}
    <!-- − only appears once the user binds it (in the sort-target editor). -->
    {#if dashDest}{@render boundSlot(dashDest, true)}{/if}
    {@render specialSlot("=", eqDest, "Archive")}
  </div>
  {#if !hasUserBinds}
    <div class="hint">
      <span class="nf">{I.tag}</span>
      bind a folder: focus the Navigator, highlight it, press ⇧1-9 — or set any folder in Settings
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
  .sep { height: 1px; background: color-mix(in srgb, var(--border), black 30%); border-radius: 1px; margin: 6px 10px; }
  .placeholder { font: inherit; text-align: left; width: 100%; cursor: pointer; }
  .placeholder .label.muted { color: var(--text-muted); }
  .key {
    display: inline-grid; place-items: center;
    width: 20px; height: 20px; border-radius: var(--radius-sm);
    background: var(--bg-chip); border: 1px solid var(--border);
    color: var(--green); font-family: var(--mono); font-weight: 700; font-size: 11px;
  }
  .trashkey { color: var(--red); }
  .key.special { color: var(--text-muted); }
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
