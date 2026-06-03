<script lang="ts">
  import { tooltip } from "$lib/tooltip.svelte";
</script>

{#if tooltip.visible && tooltip.text}
  <div class="tt" style="left:{tooltip.x}px; top:{tooltip.y}px">{tooltip.text}</div>
{/if}

<style>
  /* Matches the header tooltip styling for a uniform look across the app. */
  .tt {
    position: fixed;
    z-index: 90;
    width: max-content;
    max-width: 460px;
    background: var(--bg-panel);
    color: var(--text-secondary);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 7px 10px;
    font-size: 11px;
    line-height: 1.55;
    white-space: pre-line;
    box-shadow: 0 12px 34px rgba(0, 0, 0, 0.5);
    pointer-events: none;
  }
</style>
