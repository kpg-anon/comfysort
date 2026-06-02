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

  const DIGITS = new Set(["1", "2", "3", "4", "5", "6", "7", "8", "9"]);

  // Keyboard-first: every action has a key. Hotkeys (digits) and undo are global
  // across panes; navigation keys route by which pane has focus.
  function onKey(e: KeyboardEvent) {
    if (!open) return;
    const t = e.target as HTMLElement;
    if (t && (t.tagName === "INPUT" || t.tagName === "TEXTAREA")) return;

    // --- Global: focus switching ---
    if (e.key === "Tab") {
      e.preventDefault();
      session.toggleFocus();
      return;
    }

    // --- Global: digit hotkey slots (event.code is layout-stable) ---
    const m = e.code.match(/^Digit([0-9])$/);
    if (m) {
      const d = m[1];
      e.preventDefault();
      if (d === "0") {
        if (!e.shiftKey) session.moveHotkey("0"); // trash
      } else if (DIGITS.has(d)) {
        if (e.shiftKey) session.copyHotkey(d);
        else session.moveHotkey(d);
      }
      return;
    }

    // --- Global: undo ---
    if (e.key === "u" || e.key === "U") {
      e.preventDefault();
      session.undo();
      return;
    }

    // --- Pane-routed navigation ---
    if (session.focus === "navigator") {
      navigatorKey(e);
    } else {
      inboxKey(e);
    }
  }

  function inboxKey(e: KeyboardEvent) {
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
</style>
