<script lang="ts">
  import { session } from "$lib/session.svelte";
  import { settings } from "$lib/settings.svelte";
  import { I } from "$lib/icons";
  import { getVersion } from "@tauri-apps/api/app";

  // App version (from tauri.conf.json) shown next to the brand.
  let version = $state("");
  $effect(() => {
    getVersion().then((v) => (version = v)).catch(() => {});
  });

  function norm(p: string): string {
    return p.replace(/\\/g, "/");
  }
  function leaf(p: string): string {
    const n = norm(p).replace(/\/+$/, "");
    return n.slice(n.lastIndexOf("/") + 1) || n;
  }

  // One-shot spin on the refresh button as click feedback.
  let refreshing = $state(false);
  function doRefresh() {
    refreshing = true;
    session.refreshInbox();
  }

  // Inputs are a `;`-joined list; show a count when there's more than one.
  const inputParts = $derived(
    (session.input ?? "").split(";").map((s) => s.trim()).filter(Boolean),
  );
  const inputLabel = $derived(
    inputParts.length > 1 ? `${inputParts.length} folders` : inputParts[0] ? norm(inputParts[0]) : "",
  );
  const inputTitle = $derived(
    inputParts.length
      ? "Inbox:\n" + inputParts.map(norm).join("\n") + "\n\nClick to choose a different folder"
      : "Choose an inbox folder",
  );
</script>

<header>
  <div class="left">
    <button class="chip input" title={inputTitle} onclick={() => session.changeInput()}>
      <span class="nf gi">{I.inbox}</span>
      <span class="txt">{inputLabel}</span>
    </button>
    <button
      class="iconbtn"
      class:spinning={refreshing}
      title="Rescan the inbox for new files (F5)"
      onclick={doRefresh}
      onanimationend={() => (refreshing = false)}
    >
      <span class="nf">{I.refresh}</span>
    </button>
    <button class="iconbtn" title="Add another inbox folder" onclick={() => session.addInputFolder()}>
      <span class="nf">{I.folderPlus}</span>
    </button>
  </div>

  {#if session.status}
    <div class="status status-{session.statusKind}" class:busy={session.busy}>
      {#if session.busy}<span class="spinner"></span>{:else}▸{/if}
      {session.status}
    </div>
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
    <span class="brand">comfysort{version ? ` ${version}` : ""}</span>
    <button class="cog nf" title="Action history" onclick={() => session.toggleHistory()}>{I.history}</button>
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
  .left { justify-self: start; display: inline-flex; align-items: center; gap: 6px; min-width: 0; }
  .input { min-width: 0; color: var(--purple); }
  .input:hover { border-color: var(--purple); }
  .iconbtn {
    display: grid; place-items: center; flex: none;
    width: 28px; height: 28px; padding: 0;
    border: 1px solid transparent; background: var(--bg-chip); color: var(--text-muted);
    border-radius: 20px; cursor: pointer; font-size: 12px;
  }
  .iconbtn:hover { color: var(--purple); border-color: var(--purple); }
  .iconbtn:active { transform: scale(0.9); }
  .iconbtn .nf { display: inline-block; line-height: 1; }
  .iconbtn.spinning .nf { animation: cs-spin 0.6s ease; }
  @keyframes cs-spin { from { transform: rotate(0deg); } to { transform: rotate(360deg); } }
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
  .status.busy { color: var(--cyan); }
  .status .spinner {
    display: inline-block;
    width: 10px; height: 10px;
    border: 2px solid color-mix(in srgb, var(--cyan) 30%, transparent);
    border-top-color: var(--cyan);
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
    vertical-align: -1px;
    margin-right: 3px;
  }
  @keyframes spin { to { transform: rotate(360deg); } }
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
