<script lang="ts">
  import { settings } from "$lib/settings.svelte";
  import { I } from "$lib/icons";
  import { THEMES } from "$lib/themes";
  import type { Settings } from "$lib/api";

  function backdrop(e: MouseEvent) {
    if (e.target === e.currentTarget) settings.close();
  }
</script>

{#snippet toggleRow(name: string, desc: string, value: boolean, key: keyof Settings)}
  <div class="row">
    <div class="meta">
      <div class="name">{name}</div>
      <div class="desc">{desc}</div>
    </div>
    <button
      class="switch"
      class:on={value}
      role="switch"
      aria-checked={value}
      aria-label={name}
      onclick={() => settings.set(key, !value as never)}
    ><span class="knob"></span></button>
  </div>
{/snippet}

{#snippet selectRow(name: string, desc: string, value: string, key: keyof Settings, opts: [string, string][])}
  <div class="row">
    <div class="meta">
      <div class="name">{name}</div>
      <div class="desc">{desc}</div>
    </div>
    <select {value} onchange={(e) => settings.set(key, e.currentTarget.value as never)}>
      {#each opts as [v, label]}<option value={v}>{label}</option>{/each}
    </select>
  </div>
{/snippet}

{#snippet pathRow(name: string, desc: string, value: string, which: "input" | "output")}
  <div class="row">
    <div class="meta">
      <div class="name">{name}</div>
      <div class="desc">{desc}</div>
    </div>
    <div class="pathctl">
      <button class="pathbtn" class:set={!!value} title={value || "Not set"} onclick={() => settings.pickDefault(which)}>
        {value || "Choose folder…"}
      </button>
      {#if value}
        <button class="pathx nf" title="Clear" onclick={() => settings.clearDefault(which)}>{I.close}</button>
      {/if}
    </div>
  </div>
{/snippet}

{#if settings.open}
  <div class="scrim" onclick={backdrop} role="presentation">
    <div class="panel">
      <header>
        <span class="nf cog">{I.cog}</span>
        <h2>Settings</h2>
        <button class="x nf" title="Close (Esc)" onclick={() => settings.close()}>{I.close}</button>
      </header>

      <div class="body">
        <section>
          <h3>Appearance</h3>
          <div class="themes">
            {#each THEMES as t (t.id)}
              <button
                class="theme"
                class:active={settings.theme === t.id}
                onclick={() => settings.set("theme", t.id)}
              >
                <span class="swatches">
                  {#each t.swatches as c}<span class="sw" style="background:{c}"></span>{/each}
                </span>
                <span class="tname">{t.name}</span>
              </button>
            {/each}
          </div>
        </section>

        <section>
          <h3>Startup</h3>
          <p class="note">When both default folders are set, the app opens straight into them and skips the start screen.</p>
          {@render pathRow("Default inbox", "Folder of media to sort on launch.", settings.defaultInput, "input")}
          {@render pathRow("Default destination", "Root whose child folders become sort targets.", settings.defaultOutput, "output")}
        </section>

        <section>
          <h3>Behavior</h3>
          {@render selectRow(
            "Collision policy",
            "When a file with the same name already exists at the destination.",
            settings.collisionPolicy,
            "collisionPolicy",
            [["rename", "Rename — name (2).ext"], ["skip", "Skip the move"], ["overwrite", "Overwrite"]],
          )}
          {@render toggleRow(
            "Confirm folder delete",
            "Ask before sending a folder to trash (it's reversible either way).",
            settings.confirmFolderDelete,
            "confirmFolderDelete",
          )}
          {@render toggleRow(
            "Confirm cross-drive moves",
            "Ask before a move that copies across drives, then removes the source.",
            settings.confirmCrossDrive,
            "confirmCrossDrive",
          )}
        </section>

        <section>
          <h3>Inbox defaults</h3>
          <p class="note">Applied when a session opens — you can still change them live with s / ^r / f.</p>
          {@render selectRow("Sort field", "Initial column the inbox sorts by.", settings.defaultSortField, "defaultSortField",
            [["mod", "Modified"], ["name", "Name"], ["size", "Size"]])}
          {@render selectRow("Sort order", "Ascending or descending.", settings.defaultSortOrder, "defaultSortOrder",
            [["desc", "Descending"], ["asc", "Ascending"]])}
          {@render selectRow("Filter", "Which media kinds to show.", settings.defaultFilter, "defaultFilter",
            [["all", "All"], ["images", "Images"], ["videos", "Videos"]])}
        </section>

        <section>
          <h3>Video previews</h3>
          {@render toggleRow("Autoplay", "Start playing a video as soon as it's previewed.", settings.videoAutoplay, "videoAutoplay")}
          {@render toggleRow("Loop", "Restart the video when it ends.", settings.videoLoop, "videoLoop")}
          {@render toggleRow("Muted", "Mute video preview audio.", settings.videoMuted, "videoMuted")}
        </section>
      </div>

      <footer>
        <span>Saved automatically to <code>config.toml</code>.</span>
        <button class="openconf" onclick={() => settings.openConfigFile()} title="Open config.toml in your editor">
          <span class="nf">{I.eye}</span> Open config.toml
        </button>
      </footer>
    </div>
  </div>
{/if}

<style>
  .scrim {
    position: fixed; inset: 0; z-index: 60;
    background: rgba(8, 10, 13, 0.62);
    display: grid; place-items: center;
    animation: fade 0.12s ease-out;
  }
  .panel {
    width: 540px; max-height: 86vh; display: flex; flex-direction: column;
    background: var(--bg-panel); border: 1px solid var(--border);
    border-radius: 12px; box-shadow: 0 24px 70px rgba(0, 0, 0, 0.5);
    overflow: hidden;
    animation: pop 0.13s ease-out;
  }
  @keyframes fade { from { opacity: 0; } }
  @keyframes pop { from { opacity: 0; transform: translateY(6px) scale(0.985); } }

  .themes { display: grid; grid-template-columns: 1fr 1fr; gap: 8px; }
  .theme {
    display: flex; align-items: center; gap: 10px;
    padding: 9px 11px; border-radius: var(--radius);
    border: 1px solid var(--border); background: var(--bg-chip);
    color: var(--text-secondary); cursor: pointer; text-align: left;
  }
  .theme:hover { border-color: var(--text-muted); }
  .theme.active { border-color: var(--purple); color: var(--text-primary); }
  .swatches { display: inline-flex; flex: none; border-radius: 4px; overflow: hidden; border: 1px solid var(--border); }
  .swatches .sw { width: 13px; height: 22px; }
  .tname { font-size: 12.5px; }
  header {
    display: flex; align-items: center; gap: 10px;
    padding: 14px 18px; border-bottom: 1px solid var(--border-muted);
  }
  header .cog { color: var(--purple); font-size: 15px; }
  header h2 { margin: 0; font-size: 15px; color: var(--text-primary); flex: 1; }
  .x {
    border: 1px solid var(--border); background: var(--bg-chip); color: var(--text-muted);
    width: 26px; height: 26px; border-radius: var(--radius-sm); cursor: pointer;
    display: grid; place-items: center; font-size: 12px;
  }
  .x:hover { color: var(--red); border-color: var(--red); }
  /* overflow-x hidden + overscroll-contain stops trackpad horizontal scroll /
     rubber-banding past the panel bounds. */
  .body { overflow-y: auto; overflow-x: hidden; overscroll-behavior: contain; padding: 6px 18px 14px; }
  section { padding: 12px 0; border-bottom: 1px solid var(--border-muted); }
  section:last-child { border-bottom: none; }
  h3 { margin: 0 0 8px; font-size: 11px; text-transform: uppercase; letter-spacing: 0.07em; color: var(--purple); }
  .note { margin: -4px 0 10px; font-size: 11px; color: var(--text-muted); }
  .row {
    display: flex; align-items: center; justify-content: space-between; gap: 16px;
    padding: 7px 0;
  }
  .meta { min-width: 0; }
  .name { color: var(--text-primary); font-size: 13px; }
  .desc { color: var(--text-muted); font-size: 11px; margin-top: 1px; }
  select {
    flex: none; min-width: 168px;
    background: var(--bg-chip); color: var(--text-primary);
    border: 1px solid var(--border); border-radius: var(--radius-sm);
    padding: 5px 8px; font-size: 12px; font-family: inherit; cursor: pointer;
  }
  select:hover { border-color: var(--purple); }
  .switch {
    flex: none; width: 40px; height: 22px; border-radius: 11px;
    border: 1px solid var(--border); background: var(--bg-chip); cursor: pointer;
    position: relative; transition: background 0.15s, border-color 0.15s; padding: 0;
  }
  .switch .knob {
    position: absolute; top: 2px; left: 2px; width: 16px; height: 16px;
    border-radius: 50%; background: var(--text-muted); transition: left 0.15s, background 0.15s;
  }
  .switch.on { background: color-mix(in srgb, var(--green) 30%, var(--bg-chip)); border-color: var(--green); }
  .switch.on .knob { left: 20px; background: var(--green); }
  footer {
    display: flex; align-items: center; justify-content: space-between; gap: 12px;
    padding: 10px 18px; border-top: 1px solid var(--border-muted);
    color: var(--text-muted); font-size: 11px;
  }
  footer code { font-family: var(--mono); color: var(--text-secondary); }
  .openconf {
    display: inline-flex; align-items: center; gap: 6px; flex: none;
    border: 1px solid var(--border); background: var(--bg-chip); color: var(--text-secondary);
    border-radius: var(--radius-sm); padding: 5px 10px; cursor: pointer; font-size: 12px;
  }
  .openconf:hover { border-color: var(--purple); color: var(--text-primary); }
  .openconf .nf { color: var(--cyan); font-size: 12px; }

  /* default-folder picker rows */
  .pathctl { display: flex; align-items: center; gap: 6px; flex: none; max-width: 230px; }
  .pathbtn {
    min-width: 150px; max-width: 200px;
    border: 1px solid var(--border); background: var(--bg-chip); color: var(--text-muted);
    border-radius: var(--radius-sm); padding: 5px 9px; cursor: pointer;
    font-family: var(--mono); font-size: 11.5px; text-align: left;
    overflow: hidden; white-space: nowrap; text-overflow: ellipsis;
  }
  .pathbtn.set { color: var(--text-primary); }
  .pathbtn:hover { border-color: var(--purple); }
  .pathx {
    flex: none; border: 1px solid var(--border); background: var(--bg-chip); color: var(--text-muted);
    border-radius: var(--radius-sm); width: 26px; height: 26px; cursor: pointer;
    display: grid; place-items: center; font-size: 11px;
  }
  .pathx:hover { color: var(--red); border-color: var(--red); }
</style>
