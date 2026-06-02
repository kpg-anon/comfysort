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

  function onKey(e: KeyboardEvent) {
    if (!open) return;
    // Don't hijack typing in inputs.
    const t = e.target as HTMLElement;
    if (t && (t.tagName === "INPUT" || t.tagName === "TEXTAREA")) return;

    // Digit slots: event.code is layout-stable (Shift+1 != "!").
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

    switch (e.key.toLowerCase()) {
      case "arrowdown":
      case "j":
        e.preventDefault();
        session.next();
        break;
      case "arrowup":
      case "k":
        e.preventDefault();
        session.prev();
        break;
      case "u":
        e.preventDefault();
        session.undo();
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
