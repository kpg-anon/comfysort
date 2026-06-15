<script lang="ts">
  import { session } from "$lib/session.svelte";
  import { settings } from "$lib/settings.svelte";
  import { api } from "$lib/api";
  import { I } from "$lib/icons";

  let input = $state<string>("");
  let output = $state<string>("");

  // Seed the pickers with the configured defaults (once settings load) so when
  // the start screen is shown despite defaults being set — i.e. a divergent last
  // session exists — "Start sorting" is still a one-click path into the defaults.
  let seeded = false;
  $effect(() => {
    if (!seeded && settings.loaded) {
      seeded = true;
      if (settings.defaultInput) input = settings.defaultInput;
      if (settings.defaultOutput) output = settings.defaultOutput;
    }
  });

  async function pick(which: "input" | "output") {
    const title = which === "input" ? "Choose the inbox folder to sort" : "Choose the destination root";
    const dir = await api.pickDirectory(title);
    if (dir) {
      if (which === "input") input = dir;
      else output = dir;
    }
  }

  const ready = $derived(input.length > 0 && output.length > 0);

  // "Restore last session" — only when a prior session's roots differ from the
  // configured defaults (matching defaults already auto-open, skipping this screen).
  function leaf(p: string): string {
    const n = p.replace(/[\\/]+$/, "");
    return n.slice(Math.max(n.lastIndexOf("/"), n.lastIndexOf("\\")) + 1) || n;
  }
  const lastInputLabel = $derived.by(() => {
    const parts = settings.lastInput.split(";").map((s) => s.trim()).filter(Boolean);
    return parts.length > 1 ? `${parts.length} folders` : parts[0] ? leaf(parts[0]) : "";
  });
</script>

<div class="start">
  <div class="card">
    <div class="brandhead">
      <img class="logo" src="/icon.png" alt="comfysort" />
      <h1><span>comfysort</span></h1>
    </div>
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

    {#if settings.canRestoreLast}
      <button
        class="restore"
        disabled={session.busy}
        onclick={() => session.open(settings.lastInput, settings.lastOutput)}
      >
        <span class="ricon nf">{I.history}</span>
        <span class="rtext">
          Restore last session
          <small>{lastInputLabel} → {leaf(settings.lastOutput)}</small>
        </span>
      </button>
    {/if}
  </div>
</div>

<style>
  .start {
    height: 100%;
    display: grid;
    place-items: center;
    /* Flat --bg-app (no gradient) so the start screen is the same single color
       as the boot screen and the loaded app — one cohesive background. */
    background: var(--bg-app);
  }
  .card {
    width: 460px;
    background: var(--bg-panel);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 28px 30px 26px;
    /* Layered shadow + a faint top highlight reads as a crisp raised panel.
       A single huge soft shadow bands on the flat near-black background, so we
       use tighter, lower-alpha layers and let the 1px border define the edge. */
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.04),
      0 1px 2px rgba(0, 0, 0, 0.4),
      0 8px 24px rgba(0, 0, 0, 0.34);
  }
  .brandhead { display: flex; align-items: center; gap: 14px; margin-bottom: 4px; }
  .logo { width: 66px; height: 66px; flex: none; filter: drop-shadow(0 4px 14px rgba(0, 0, 0, 0.5)); }
  h1 { margin: 0; font-family: var(--mono); font-weight: 700; font-size: 28px; }
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
  .restore {
    width: 100%;
    margin-top: 10px;
    padding: 9px 12px;
    display: flex;
    align-items: center;
    gap: 10px;
    border-radius: var(--radius);
    border: 1px solid var(--border);
    background: var(--bg-chip);
    color: var(--text-secondary);
    cursor: pointer;
    text-align: left;
  }
  .restore:hover:not(:disabled) { border-color: var(--purple); color: var(--text-primary); }
  .restore:disabled { opacity: 0.4; cursor: default; }
  .restore .ricon { color: var(--purple); font-size: 13px; flex: none; }
  .rtext { display: flex; flex-direction: column; min-width: 0; font-size: 13px; font-weight: 600; }
  .rtext small {
    font-weight: 400; font-size: 11px; color: var(--text-muted);
    font-family: var(--mono);
    overflow: hidden; white-space: nowrap; text-overflow: ellipsis;
  }
  .err {
    margin-bottom: 12px;
    color: var(--red);
    font-size: 12px;
    word-break: break-word;
  }
</style>
