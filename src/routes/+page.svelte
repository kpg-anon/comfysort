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

  const open = $derived(session.input !== null && session.output !== null);

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

    // --- Modal: cross-drive confirm swallows all other input ---
    if (session.crossPrompt) {
      const k = e.key.toLowerCase();
      if (k === "y") { e.preventDefault(); session.resolveCross("once"); }
      else if (k === "a") { e.preventDefault(); session.resolveCross("always"); }
      else if (k === "n" || e.key === "Escape") { e.preventDefault(); session.resolveCross("cancel"); }
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

    // --- Global: undo ---
    if (e.key === "u" || e.key === "U") {
      e.preventDefault();
      session.undo();
      return;
    }

    // --- Global: fuzzy search (focuses the Navigator + opens search) ---
    if (e.key === "/") {
      e.preventDefault();
      session.startSearch();
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
      case "j":
        e.preventDefault();
        session.next();
        break;
      case "ArrowUp":
      case "k":
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
      case "r":
        if (e.ctrlKey) {
          e.preventDefault();
          session.toggleSortOrder();
        }
        break;
    }
  }

  function navigatorKey(e: KeyboardEvent) {
    switch (e.key) {
      case "ArrowDown":
      case "j":
        e.preventDefault();
        session.navDown();
        break;
      case "ArrowUp":
      case "k":
        e.preventDefault();
        session.navUp();
        break;
      case "ArrowRight":
      case "l":
        e.preventDefault();
        session.navDrill();
        break;
      case "ArrowLeft":
      case "h":
        e.preventDefault();
        session.navAscend();
        break;
      case "Enter":
        e.preventDefault();
        session.navEnterMove();
        break;
      case "D":
        e.preventDefault();
        session.navCopy(); // Shift+D copies into the highlighted folder
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

<svelte:window onkeydown={onKey} />

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
        <div class="mtitle">Cross-drive move</div>
        <p>
          Moving {session.crossPrompt.count}
          {session.crossPrompt.count === 1 ? "file" : "files"} from
          <b>{session.crossPrompt.sourceVolume}</b> into
          <b>{session.crossPrompt.destLabel}</b> copies across drives, then removes
          the source. This is slower than a same-drive move.
        </p>
        <div class="mrow">
          <button class="mbtn go" onclick={() => session.resolveCross("once")}><kbd>y</kbd> Move once</button>
          <button class="mbtn" onclick={() => session.resolveCross("always")}><kbd>a</kbd> Always this session</button>
          <button class="mbtn cancel" onclick={() => session.resolveCross("cancel")}><kbd>n</kbd> Cancel</button>
        </div>
      </div>
    </div>
  {/if}
{/if}

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
    background: rgba(8, 10, 13, 0.6);
    display: grid;
    place-items: center;
    z-index: 50;
  }
  .modal {
    width: 440px;
    background: var(--bg-panel);
    border: 1px solid var(--orange);
    border-radius: 10px;
    padding: 20px 22px;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5);
  }
  .mtitle { color: var(--orange); font-weight: 700; margin-bottom: 8px; }
  .modal p { color: var(--text-secondary); font-size: 12.5px; margin: 0 0 16px; line-height: 1.5; }
  .modal b { color: var(--text-primary); }
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
  .mbtn:hover { border-color: var(--text-muted); }
  .mbtn.go { color: var(--green); border-color: color-mix(in srgb, var(--green) 40%, var(--border)); }
  .mbtn.cancel { color: var(--text-muted); }
  .mbtn kbd {
    font-family: var(--mono);
    background: var(--bg-app);
    border: 1px solid var(--border);
    border-radius: 3px;
    padding: 0 4px;
    font-size: 11px;
  }
</style>
