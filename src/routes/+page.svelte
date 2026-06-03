<script lang="ts">
  import "$lib/theme.css";
  import { session } from "$lib/session.svelte";
  import Header from "$lib/components/Header.svelte";
  import Inbox from "$lib/components/Inbox.svelte";
  import Preview from "$lib/components/Preview.svelte";
  import FileInfo from "$lib/components/FileInfo.svelte";
  import SortTargets from "$lib/components/SortTargets.svelte";
  import Navigator from "$lib/components/Navigator.svelte";
  import BottomBar from "$lib/components/BottomBar.svelte";
  import StartScreen from "$lib/components/StartScreen.svelte";
  import Settings from "$lib/components/Settings.svelte";
  import ContextMenu from "$lib/components/ContextMenu.svelte";
  import UpdateNotice from "$lib/components/UpdateNotice.svelte";
  import HistoryPanel from "$lib/components/HistoryPanel.svelte";
  import { settings } from "$lib/settings.svelte";
  import { I } from "$lib/icons";

  const open = $derived(session.input !== null && session.output !== null);

  // Load persisted settings (config.toml) once at startup so session defaults apply.
  $effect(() => {
    if (!settings.loaded) settings.load();
  });
  // If the user configured default folders, open straight into them (skip the
  // start screen). Runs once, after settings load.
  let autoOpened = false;
  $effect(() => {
    if (settings.loaded && !autoOpened) {
      autoOpened = true;
      if (!session.input && settings.defaultInput && settings.defaultOutput) {
        session.open(settings.defaultInput, settings.defaultOutput);
      }
    }
  });
  // Apply the active theme preset to the document (re-themes all tokens).
  $effect(() => {
    document.documentElement.dataset.theme = settings.theme;
  });
  // When the cross-drive prompt opens, drop focus from any text field so its
  // y/a/n keys aren't captured by an input.
  $effect(() => {
    if (session.crossPrompt) (document.activeElement as HTMLElement | null)?.blur();
  });

  // Resolve a layout-stable hotkey slot from a KeyboardEvent.code.
  function slotFromCode(code: string): string | null {
    const m = code.match(/^Digit([0-9])$/);
    if (m) return m[1];
    if (code === "Minus") return "-";
    if (code === "Equal") return "=";
    return null;
  }

  // Keyboard-first: every action has a key. Hotkey slots and undo are global
  // across panes; navigation keys route by which pane has focus.
  function onKey(e: KeyboardEvent) {
    if (!open) return;

    // Any keypress dismisses an open context menu (the action still proceeds).
    if (session.ctx) session.closeContext();
    if (session.navCtx) session.closeNavContext();

    // --- Global: F5 refreshes the inbox instead of reloading the webview
    //     (a page reload would drop the session back to the start screen). ---
    if (e.key === "F5") {
      e.preventDefault();
      session.refreshInbox();
      return;
    }
    // --- Global: stop Ctrl+R from reloading the webview; in the Inbox it flips
    //     the sort order. ---
    if (e.ctrlKey && (e.key === "r" || e.key === "R")) {
      e.preventDefault();
      if (session.focus === "inbox") session.toggleSortOrder();
      return;
    }

    // --- Modal: settings overlay swallows app shortcuts (its own controls work) ---
    if (settings.open) {
      if (e.key === "Escape") { e.preventDefault(); settings.close(); }
      return;
    }

    // --- Modal: history popup — Escape closes it ---
    if (session.showHistory && e.key === "Escape") {
      e.preventDefault();
      session.toggleHistory();
      return;
    }

    // --- Modal: cross-drive confirm swallows ALL input (incl. when a search/
    //     input field has focus — preventDefault stops stray text entry). ---
    if (session.crossPrompt) {
      e.preventDefault();
      const k = e.key.toLowerCase();
      if (k === "y") session.resolveCross("once");
      else if (k === "a") session.resolveCross("always");
      else if (k === "n" || e.key === "Escape") session.resolveCross("cancel");
      return;
    }

    const t = e.target as HTMLElement;
    if (t && (t.tagName === "INPUT" || t.tagName === "TEXTAREA")) return;

    // --- Global: Esc closes an open fuzzy search before any pane action, even
    //     if the search input lost focus (otherwise it would fall through to
    //     navigatorKey and swap focus to the Inbox). ---
    if (e.key === "Escape" && session.searching) {
      e.preventDefault();
      session.exitSearch();
      return;
    }

    // --- Global: focus switching ---
    if (e.key === "Tab") {
      e.preventDefault();
      session.toggleFocus();
      return;
    }

    // --- Global: hotkey slots (1-9, 0=trash, -, =). Shift = copy in the Inbox,
    //     bind the highlighted folder in the Navigator. ---
    const slot = slotFromCode(e.code);
    if (slot) {
      e.preventDefault();
      if (slot === "0") {
        if (!e.shiftKey) session.moveHotkey("0"); // trash
      } else if (e.shiftKey) {
        if (session.focus === "navigator") session.bindHighlighted(slot);
        else session.copyHotkey(slot);
      } else {
        session.moveHotkey(slot);
      }
      return;
    }

    // --- Global: undo (Ctrl+U) ---
    if (e.ctrlKey && (e.key === "u" || e.key === "U")) {
      e.preventDefault();
      session.undo();
      return;
    }

    // --- Inbox-only: "/" jumps to fuzzy folder search. In the Navigator you
    //     just start typing (see navigatorKey), so "/" isn't needed there. ---
    if (e.key === "/" && session.focus === "inbox") {
      e.preventDefault();
      session.startSearch();
      return;
    }

    // --- Global: new folder in the current Navigator directory (Ctrl+N) ---
    if (e.ctrlKey && (e.key === "n" || e.key === "N")) {
      e.preventDefault();
      session.startNewFolder();
      return;
    }

    // --- Global: Shift+D copies the current target(s) into the highlighted
    //     Navigator folder, regardless of focus (mirrors a folder's copy button). ---
    if (e.shiftKey && e.key === "D") {
      e.preventDefault();
      session.navCopy();
      return;
    }

    // --- Pane-routed navigation ---
    if (session.focus === "navigator") navigatorKey(e);
    else inboxKey(e);
  }

  function inboxKey(e: KeyboardEvent) {
    // Shift+arrows extend a contiguous multiselection.
    if (e.shiftKey && (e.key === "ArrowDown" || e.key === "ArrowUp")) {
      e.preventDefault();
      session.extendSelection(e.key === "ArrowDown" ? 1 : -1);
      return;
    }
    switch (e.key) {
      case "ArrowDown":
        e.preventDefault();
        if (e.altKey) session.bottom();
        else session.next();
        break;
      case "ArrowUp":
        e.preventDefault();
        if (e.altKey) session.top();
        else session.prev();
        break;
      case "s":
        e.preventDefault();
        session.cycleSortField();
        break;
      case "f":
        e.preventDefault();
        session.cycleFilter();
        break;
    }
  }

  function navigatorKey(e: KeyboardEvent) {
    // Type-to-search: a plain letter opens fuzzy search seeded with that char, so
    // the Navigator needs no "/". Digits stay as hotkey slots; modifiers pass
    // through (Ctrl+N/D, Shift+digit bind).
    if (e.key.length === 1 && /[a-z]/i.test(e.key) && !e.ctrlKey && !e.altKey && !e.metaKey) {
      e.preventDefault();
      session.startSearch();
      session.updateSearch(e.key);
      return;
    }
    switch (e.key) {
      case "ArrowDown":
        e.preventDefault();
        session.navDown();
        break;
      case "ArrowUp":
        e.preventDefault();
        session.navUp();
        break;
      case "ArrowRight":
        e.preventDefault();
        session.navDrill();
        break;
      case "ArrowLeft":
        e.preventDefault();
        session.navAscend();
        break;
      case "Enter":
        e.preventDefault();
        session.navEnterMove();
        break;
      case "d":
        if (e.ctrlKey) {
          e.preventDefault();
          session.deleteHighlightedFolder(); // Ctrl+D deletes folder to trash
        }
        break;
      case "Escape":
        e.preventDefault();
        session.focusInbox();
        break;
    }
  }
</script>

<svelte:window onkeydown={onKey} oncontextmenu={(e) => e.preventDefault()} />

{#if !open}
  <StartScreen />
{:else}
  <div class="app">
    <Header />
    <main>
      <Inbox />
      <Preview />
      <div class="right">
        <FileInfo />
        <SortTargets />
        <Navigator />
      </div>
    </main>
    <BottomBar />
  </div>

  {#if session.crossPrompt}
    <div class="modal-scrim">
      <div class="modal">
        <div class="mhead">
          <span class="micon nf">{I.warn}</span>
          <div class="mheadtext">
            <div class="mtitle">Cross-drive move</div>
            <div class="msub">Copies across drives, then removes the source — slower than a same-drive move.</div>
          </div>
        </div>
        <p class="mbody">
          Move <b>{session.crossPrompt.count}
          {session.crossPrompt.count === 1 ? "file" : "files"}</b> from
          <b class="vol">{session.crossPrompt.sourceVolume}</b> into
          <b class="dest">{session.crossPrompt.destLabel}</b>?
        </p>
        <div class="mrow">
          <button class="mbtn go" onclick={() => session.resolveCross("once")}><kbd>y</kbd> Move once</button>
          <button class="mbtn always" onclick={() => session.resolveCross("always")}><kbd>a</kbd> Always this session</button>
          <button class="mbtn cancel" onclick={() => session.resolveCross("cancel")}><kbd>n</kbd> Cancel</button>
        </div>
      </div>
    </div>
  {/if}

  <Settings />
  <ContextMenu />
{/if}

<UpdateNotice />
<HistoryPanel />

<style>
  .app {
    height: 100vh;
    display: grid;
    grid-template-rows: auto 1fr auto;
    gap: var(--gap);
    padding: var(--gap);
  }
  main {
    display: grid;
    grid-template-columns: minmax(220px, 280px) 1fr minmax(260px, 320px);
    gap: var(--gap);
    min-height: 0;
  }
  .right {
    display: grid;
    grid-template-rows: auto auto 1fr;
    gap: var(--gap);
    min-height: 0;
  }
  .modal-scrim {
    position: fixed;
    inset: 0;
    background: rgba(8, 10, 13, 0.62);
    display: grid;
    place-items: center;
    z-index: 50;
    animation: mfade 0.12s ease-out;
  }
  .modal {
    width: 460px;
    background: var(--bg-panel);
    border: 1px solid var(--border);
    border-top: 2px solid var(--orange);
    border-radius: 12px;
    padding: 20px 22px 18px;
    box-shadow: 0 24px 70px rgba(0, 0, 0, 0.55);
    animation: mpop 0.13s ease-out;
  }
  @keyframes mfade { from { opacity: 0; } }
  @keyframes mpop { from { opacity: 0; transform: translateY(6px) scale(0.985); } }
  .mhead { display: flex; gap: 13px; align-items: flex-start; margin-bottom: 14px; }
  .micon {
    flex: none; width: 34px; height: 34px; border-radius: 9px;
    display: grid; place-items: center; font-size: 16px;
    color: var(--orange);
    background: color-mix(in srgb, var(--orange) 16%, transparent);
    border: 1px solid color-mix(in srgb, var(--orange) 35%, var(--border));
  }
  .mtitle { color: var(--text-primary); font-weight: 700; font-size: 14px; }
  .msub { color: var(--text-muted); font-size: 11.5px; margin-top: 2px; line-height: 1.4; }
  .mbody { color: var(--text-secondary); font-size: 13px; margin: 0 0 16px; line-height: 1.5; }
  .mbody b { color: var(--text-primary); }
  .mbody .vol { color: var(--orange); }
  .mbody .dest { color: var(--green); }
  .mrow { display: flex; gap: 8px; }
  .mbtn {
    flex: 1;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 9px;
    border-radius: var(--radius);
    border: 1px solid var(--border);
    background: var(--bg-chip);
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 12px;
  }
  .mbtn:hover { border-color: var(--text-muted); color: var(--text-primary); }
  .mbtn.go {
    background: var(--green); color: var(--text-inverse);
    border-color: var(--green); font-weight: 600;
  }
  .mbtn.go:hover { filter: brightness(1.06); border-color: var(--green); }
  .mbtn.always { color: var(--orange); }
  .mbtn.always:hover { border-color: var(--orange); }
  .mbtn.cancel { color: var(--text-muted); }
  .mbtn kbd {
    font-family: var(--mono);
    background: var(--bg-app);
    border: 1px solid var(--border);
    border-radius: 3px;
    padding: 0 4px;
    font-size: 11px;
  }
  .mbtn.go kbd { background: rgba(0, 0, 0, 0.18); border-color: rgba(0, 0, 0, 0.22); color: var(--text-inverse); }
</style>
