<script lang="ts">
  import { session } from "$lib/session.svelte";
  import { settings } from "$lib/settings.svelte";
  import { I } from "$lib/icons";

  function norm(p: string): string {
    return p.replace(/\\/g, "/");
  }
  function leaf(p: string): string {
    const n = norm(p).replace(/\/+$/, "");
    return n.slice(n.lastIndexOf("/") + 1) || n;
  }
</script>

<header>
  <button
    class="chip input"
    title={"Inbox: " + (session.input ?? "") + "\nClick to choose a different folder"}
    onclick={() => session.changeInput()}
  >
    <span class="nf gi">{I.inbox}</span>
    <span class="txt">{session.input ? norm(session.input) : ""}</span>
  </button>

  {#if session.status}
    <div class="status status-{session.statusKind}">▸ {session.status}</div>
  {:else}
    <div></div>
  {/if}

  <div class="right">
    <button
      class="chip output"
      title={"Destination root: " + (session.output ?? "") + "\nClick to choose a different root"}
      onclick={() => session.changeOutput()}
    >
      <span class="nf gi">{I.drive}</span>
      <span class="txt">{session.output ? leaf(session.output) : ""}</span>
    </button>
    <span class="brand">comfysort</span>
    <button class="cog nf" title="Settings" onclick={() => settings.toggleOpen()}>{I.cog}</button>
  </div>
</header>

<style>
  header {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto minmax(0, 1fr);
    align-items: center;
    gap: var(--gap);
    padding: 6px 10px;
    background: var(--bg-panel);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    max-width: 100%;
    overflow: hidden;
    border: 1px solid transparent;
    background: var(--bg-chip);
    padding: 4px 11px;
    border-radius: 20px;
    cursor: pointer;
    font-family: var(--mono);
    font-size: 12px;
  }
  .chip .txt { overflow: hidden; white-space: nowrap; text-overflow: ellipsis; }
  .input { justify-self: start; color: var(--purple); }
  .input:hover { border-color: var(--purple); }
  .gi { font-size: 12px; flex: none; opacity: 0.9; }
  .status {
    justify-self: center;
    font-size: 12px;
    color: var(--cyan);
    background: var(--bg-chip);
    padding: 2px 12px;
    border-radius: 20px;
    white-space: nowrap;
  }
  .status-good { color: var(--green); }
  .status-bad { color: var(--red); }
  .status-info { color: var(--cyan); }
  .right { justify-self: end; display: inline-flex; align-items: center; gap: 10px; min-width: 0; }
  .output { color: var(--cyan); }
  .output:hover { border-color: var(--cyan); }
  .brand { color: var(--text-muted); font-family: var(--mono); font-size: 12px; flex: none; }
  .cog {
    display: grid; place-items: center;
    width: 26px; height: 26px; flex: none;
    border: 1px solid var(--border); background: var(--bg-chip); color: var(--text-muted);
    border-radius: var(--radius-sm); cursor: pointer; font-size: 13px; padding: 0;
  }
  .cog:hover { color: var(--purple); border-color: var(--purple); }
</style>
