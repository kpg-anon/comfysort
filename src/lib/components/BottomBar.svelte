<script lang="ts">
  import { session } from "$lib/session.svelte";
  import { humanSize } from "$lib/api";
  import { I } from "$lib/icons";

  // [key, label, accent] — accent is a theme color var name, color-coded by the
  // action's semantic class (green=move, red=destructive, cyan=copy/neutral,
  // purple=bind/search, yellow=view, blue=navigation, orange=undo).
  type Chip = [string, string, string];

  const inboxChips: Chip[] = [
    ["tab", "navigator", "cyan"],
    ["↑↓", "select", "blue"],
    ["⌥↑↓", "top/bottom", "blue"],
    ["1-9", "move", "green"],
    ["0", "trash", "red"],
    ["⇧1-9", "copy", "cyan"],
    ["s", "sort", "yellow"],
    ["f", "filter", "yellow"],
    ["/", "search", "purple"],
    ["^u", "undo", "orange"],
  ];
  const navChips: Chip[] = [
    ["tab", "inbox", "cyan"],
    ["↑↓", "move", "blue"],
    ["→ ←", "drill", "blue"],
    ["⏎", "move here", "green"],
    ["⇧d", "copy", "cyan"],
    ["⇧1-9", "bind", "purple"],
    ["type", "search", "purple"],
    ["^d", "delete", "red"],
    ["^n", "new", "green"],
    ["^u", "undo", "orange"],
  ];
  const chips = $derived(session.focus === "navigator" ? navChips : inboxChips);

  // Drive icon turns red when the output volume is nearly full.
  const lowDisk = $derived(
    session.diskFree != null && session.diskTotal
      ? session.diskFree / session.diskTotal < 0.1
      : false,
  );
</script>

<footer>
  <div class="side"></div>
  <div class="legend">
    {#each chips as [key, label, accent], i}
      {#if i > 0}<span class="sep">·</span>{/if}
      <span class="chip-key">
        <kbd style="color: var(--{accent}); border-color: color-mix(in srgb, var(--{accent}) 38%, var(--border));">{key}</kbd>
        {label}
      </span>
    {/each}
  </div>
  <div class="side disk">
    {#if session.diskFree != null && session.diskTotal != null}
      <span class="nf drive" class:low={lowDisk}>{I.drive}</span>
      <span class="free">{humanSize(session.diskFree)} free</span>
      <span class="sep">/</span>
      <span class="total">{humanSize(session.diskTotal)}</span>
    {/if}
  </div>
</footer>

<style>
  footer {
    display: grid;
    grid-template-columns: 1fr auto 1fr;
    align-items: center;
    gap: 8px;
    padding: 7px 14px;
    background: var(--bg-panel);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text-secondary);
    font-size: 11.5px;
  }
  .legend {
    display: flex;
    align-items: center;
    justify-content: center;
    flex-wrap: wrap;
    gap: 4px 8px;
  }
  .side { display: flex; align-items: center; gap: 6px; min-width: 0; }
  .disk { justify-content: flex-end; font-variant-numeric: tabular-nums; white-space: nowrap; }
  .drive { color: var(--cyan); font-size: 12px; }
  .drive.low { color: var(--red); }
  .free { color: var(--text-secondary); }
  .total { color: var(--text-muted); }
  .sep { color: var(--border); }
  .chip-key { display: inline-flex; align-items: center; gap: 5px; }
  kbd {
    font-family: var(--mono);
    font-size: 11px;
    background: var(--bg-chip);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 1px 5px;
  }
</style>
