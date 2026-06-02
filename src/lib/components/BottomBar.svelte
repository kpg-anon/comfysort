<script lang="ts">
  const chips: [string, string][] = [
    ["↑↓ / j k", "navigate"],
    ["1-9", "move"],
    ["0", "trash"],
    ["⇧1-9", "copy"],
    ["click", "move to target"],
    ["u", "undo"],
  ];
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
