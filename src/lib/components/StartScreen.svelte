<script lang="ts">
  import { session } from "$lib/session.svelte";
  import { api } from "$lib/api";

  let input = $state<string>("");
  let output = $state<string>("");

  async function pick(which: "input" | "output") {
    const title = which === "input" ? "Choose the inbox folder to sort" : "Choose the destination root";
    const dir = await api.pickDirectory(title);
    if (dir) {
      if (which === "input") input = dir;
      else output = dir;
    }
  }

  const ready = $derived(input.length > 0 && output.length > 0);
</script>

<div class="start">
  <div class="card">
    <h1><span>comfysort</span></h1>
    <p class="tag">Preview a file. Press a key. It moves. Press <kbd>u</kbd> to undo.</p>

    <div class="field">
      <div class="label">Inbox <small>files to sort</small></div>
      <button class="pick" onclick={() => pick("input")}>
        {input || "Choose folder…"}
      </button>
    </div>

    <div class="field">
      <div class="label">Destination root <small>its child folders become targets</small></div>
      <button class="pick" onclick={() => pick("output")}>
        {output || "Choose folder…"}
      </button>
    </div>

    {#if session.error}<div class="err">{session.error}</div>{/if}

    <button class="go" disabled={!ready || session.busy} onclick={() => session.open(input, output)}>
      {session.busy ? "Opening…" : "Start sorting"}
    </button>
  </div>
</div>

<style>
  .start {
    height: 100%;
    display: grid;
    place-items: center;
    background:
      radial-gradient(circle at 50% 30%, #161b22 0%, var(--bg-app) 70%);
  }
  .card {
    width: 460px;
    background: var(--bg-panel);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 28px 30px 26px;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.45);
  }
  h1 { margin: 0; font-family: var(--mono); font-weight: 700; font-size: 26px; }
  h1 span { color: var(--purple); }
  .tag { margin: 4px 0 22px; color: var(--text-muted); font-size: 12.5px; }
  .tag kbd {
    font-family: var(--mono);
    background: var(--bg-chip);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 0 4px;
  }
  .field { margin-bottom: 16px; }
  .label {
    display: flex;
    align-items: baseline;
    gap: 8px;
    margin-bottom: 6px;
    color: var(--text-secondary);
    font-weight: 600;
  }
  .label small { color: var(--text-muted); font-weight: 400; }
  .pick {
    width: 100%;
    text-align: left;
    padding: 9px 12px;
    border-radius: var(--radius);
    border: 1px solid var(--border);
    background: var(--bg-chip);
    color: var(--text-primary);
    font-family: var(--mono);
    font-size: 12px;
    cursor: pointer;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .pick:hover { border-color: var(--purple); }
  .go {
    width: 100%;
    margin-top: 10px;
    padding: 11px;
    border-radius: var(--radius);
    border: 1px solid transparent;
    background: var(--green);
    color: var(--text-inverse);
    font-weight: 700;
    font-size: 13.5px;
    cursor: pointer;
  }
  .go:disabled { opacity: 0.4; cursor: default; }
  .err {
    margin-bottom: 12px;
    color: var(--red);
    font-size: 12px;
    word-break: break-word;
  }
</style>
