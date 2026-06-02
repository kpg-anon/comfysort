<script lang="ts">
  import { session } from "$lib/session.svelte";

  // [key, label, accent] — accent is a theme color var name, color-coded by the
  // action's semantic class (green=move, red=destructive, cyan=copy/neutral,
  // purple=bind/search, yellow=view, blue=navigation, orange=undo).
  type Chip = [string, string, string];

  const inboxChips: Chip[] = [
    ["tab", "navigator", "cyan"],
    ["↑↓ jk", "select", "blue"],
    ["1-9", "move", "green"],
    ["0", "trash", "red"],
    ["⇧1-9", "copy", "cyan"],
    ["s", "sort", "yellow"],
    ["f", "filter", "yellow"],
    ["/", "search", "purple"],
    ["u", "undo", "orange"],
  ];
  const navChips: Chip[] = [
    ["tab", "inbox", "cyan"],
    ["↑↓", "move", "blue"],
    ["→ ←", "drill", "blue"],
    ["⏎", "move here", "green"],
    ["⇧d", "copy", "cyan"],
    ["⇧1-9", "bind", "purple"],
    ["/", "search", "purple"],
    ["^d", "delete", "red"],
    ["＋", "new", "green"],
    ["u", "undo", "orange"],
  ];
  const chips = $derived(session.focus === "navigator" ? navChips : inboxChips);
</script>

<footer>
  {#each chips as [key, label, accent], i}
    {#if i > 0}<span class="sep">·</span>{/if}
    <span class="chip-key">
      <kbd style="color: var(--{accent}); border-color: color-mix(in srgb, var(--{accent}) 38%, var(--border));">{key}</kbd>
      {label}
    </span>
  {/each}
</footer>

<style>
  footer {
    display: flex;
    align-items: center;
    justify-content: center;
    flex-wrap: wrap;
    gap: 4px 8px;
    padding: 7px 14px;
    background: var(--bg-panel);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text-secondary);
    font-size: 11.5px;
  }
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
