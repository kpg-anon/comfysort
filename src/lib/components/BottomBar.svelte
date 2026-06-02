<script lang="ts">
  import { session } from "$lib/session.svelte";

  const inboxChips: [string, string][] = [
    ["tab", "navigator"],
    ["↑↓ jk", "select"],
    ["1-9", "move"],
    ["0", "trash"],
    ["⇧1-9", "copy"],
    ["s", "sort"],
    ["f", "filter"],
    ["^r", "order"],
    ["u", "undo"],
  ];
  const navChips: [string, string][] = [
    ["tab", "inbox"],
    ["↑↓", "move"],
    ["→ ←", "drill"],
    ["⏎", "move here"],
    ["⇧d", "copy"],
    ["⇧1-9", "bind"],
    ["/", "search"],
    ["^d", "delete"],
    ["＋", "new"],
    ["u", "undo"],
  ];
  const chips = $derived(session.focus === "navigator" ? navChips : inboxChips);
</script>

<footer>
  {#each chips as [key, label], i}
    {#if i > 0}<span class="sep">·</span>{/if}
    <span class="chip-key"><kbd>{key}</kbd> {label}</span>
  {/each}
</footer>

<style>
  footer {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 4px 8px;
    padding: 7px 14px;
    background: var(--bg-panel);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text-muted);
    font-size: 11.5px;
  }
  .sep { color: var(--border); }
  .chip-key { display: inline-flex; align-items: center; gap: 5px; }
  kbd {
    font-family: var(--mono);
    font-size: 11px;
    color: var(--text-secondary);
    background: var(--bg-chip);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 1px 5px;
  }
</style>
