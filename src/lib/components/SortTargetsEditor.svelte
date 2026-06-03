<script lang="ts">
  import { settings } from "$lib/settings.svelte";
  import { session } from "$lib/session.svelte";
  import { I } from "$lib/icons";

  const slots = ["1", "2", "3", "4", "5", "6", "7", "8", "9", "=", "-"];
  function fallback(key: string): string {
    return key === "=" ? "Archive" : "(unset)";
  }
  function shortPath(p: string): string {
    const n = p.replace(/\\/g, "/").replace(/\/+$/, "");
    const segs = n.split("/");
    return segs.slice(-2).join("/");
  }
</script>

{#if settings.targetsOpen}
  <div class="scrim" role="presentation" onclick={() => settings.closeTargets()}></div>
  <div class="panel">
    <header>
      <span class="nf hicon">{I.keyboard}</span>
      <h2>Sort targets</h2>
      <button class="x nf" title="Close (Esc)" onclick={() => settings.closeTargets()}>{I.close}</button>
    </header>
    <p class="note">
      Bind a hotkey slot to any folder — including folders outside the destination root.
      <kbd>⇧</kbd>+digit on a highlighted Navigator folder also binds folders under the root.
    </p>
    <div class="grid">
      {#each slots as key}
        {@const d = session.destForHotkey(key)}
        <div class="card" class:set={!!d}>
          <span class="key">{key}</span>
          <div class="body">
            <div class="label" title={d ? d.path : ""}>{d ? d.label : fallback(key)}</div>
            <div class="path">{d ? shortPath(d.path) : "not set"}</div>
          </div>
          <div class="acts">
            <button class="setbtn" onclick={() => session.bindSlotViaPicker(key)}>{d ? "Change" : "Set"}</button>
            {#if d}
              <button class="clearbtn nf" title="Clear binding" onclick={() => session.unbind(key)}>{I.close}</button>
            {/if}
          </div>
        </div>
      {/each}
    </div>
  </div>
{/if}

<style>
  .scrim {
    position: fixed; inset: 0; z-index: 65;
    background: rgba(8, 10, 13, 0.6);
    animation: fade 0.12s ease-out;
  }
  .panel {
    position: fixed; z-index: 66; left: 50%; top: 50%;
    transform: translate(-50%, -50%);
    width: min(860px, 94vw); max-height: 86vh; display: flex; flex-direction: column;
    background: var(--bg-panel); border: 1px solid var(--border);
    border-radius: 12px; box-shadow: 0 24px 70px rgba(0, 0, 0, 0.55);
    overflow: hidden; animation: pop 0.13s ease-out;
  }
  @keyframes fade { from { opacity: 0; } }
  @keyframes pop { from { opacity: 0; transform: translate(-50%, -48%); } }
  header {
    display: flex; align-items: center; gap: 10px;
    padding: 13px 16px; border-bottom: 1px solid var(--border-muted);
  }
  header .hicon { color: var(--purple); font-size: 14px; }
  header h2 { margin: 0; font-size: 15px; color: var(--text-primary); flex: 1; }
  .x {
    border: 1px solid var(--border); background: var(--bg-chip); color: var(--text-muted);
    width: 26px; height: 26px; border-radius: var(--radius-sm); cursor: pointer;
    display: grid; place-items: center; font-size: 12px;
  }
  .x:hover { color: var(--red); border-color: var(--red); }
  .note { margin: 12px 16px 4px; font-size: 11.5px; color: var(--text-muted); }
  .note kbd { font-family: var(--mono); background: var(--bg-app); border: 1px solid var(--border); border-radius: 3px; padding: 0 4px; }
  /* Horizontal layout: slot cards flow across, wrapping as needed. */
  .grid {
    display: grid; grid-template-columns: repeat(auto-fill, minmax(232px, 1fr));
    gap: 10px; padding: 12px 16px 18px; overflow-y: auto; overscroll-behavior: contain;
  }
  .card {
    display: grid; grid-template-columns: auto 1fr auto; gap: 10px; align-items: center;
    padding: 10px 12px; border-radius: var(--radius);
    border: 1px solid var(--border); background: var(--bg-chip);
  }
  .card.set { border-color: color-mix(in srgb, var(--purple) 45%, var(--border)); }
  .key {
    display: inline-grid; place-items: center; flex: none;
    width: 26px; height: 26px; border-radius: var(--radius-sm);
    background: var(--bg-app); border: 1px solid var(--border);
    color: var(--green); font-family: var(--mono); font-weight: 700; font-size: 13px;
  }
  .body { min-width: 0; }
  .label { color: var(--text-primary); font-size: 13px; overflow: hidden; white-space: nowrap; text-overflow: ellipsis; }
  .card:not(.set) .label { color: var(--text-muted); }
  .path { color: var(--text-muted); font-size: 10.5px; font-family: var(--mono); overflow: hidden; white-space: nowrap; text-overflow: ellipsis; }
  .acts { display: flex; align-items: center; gap: 5px; flex: none; }
  .setbtn {
    border: 1px solid var(--border); background: var(--bg-panel); color: var(--text-secondary);
    border-radius: var(--radius-sm); padding: 5px 10px; cursor: pointer; font-size: 12px;
  }
  .setbtn:hover { border-color: var(--purple); color: var(--text-primary); }
  .clearbtn {
    border: 1px solid var(--border); background: var(--bg-panel); color: var(--text-muted);
    width: 26px; height: 26px; border-radius: var(--radius-sm); cursor: pointer;
    display: grid; place-items: center; font-size: 11px;
  }
  .clearbtn:hover { color: var(--red); border-color: var(--red); }
</style>
