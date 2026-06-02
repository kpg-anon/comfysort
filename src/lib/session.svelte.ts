// Central reactive session state (Svelte 5 runes). One instance drives the
// whole UI. Mutating actions call the backend, then apply the returned delta
// locally so the inbox never has to be re-serialized wholesale.
//
// Inbox model mirrors the TUI: `allItems` is the source of truth; `view` is the
// derived sort+filter projection the UI renders and the cursor indexes into.
import {
  api,
  volumeLabel,
  type Destination,
  type FolderEntry,
  type FolderListing,
  type MediaItem,
  type OpKind,
  type OpOutcome,
} from "./api";
import { settings } from "./settings.svelte";
import { openPath, revealItemInDir } from "@tauri-apps/plugin-opener";

type StatusKind = "info" | "good" | "bad";
export type Focus = "inbox" | "navigator";
export type SortField = "name" | "size" | "mod";
export type SortOrder = "asc" | "desc";
export type FilterMode = "all" | "images" | "videos";

/** A move that crosses a drive/share boundary, awaiting confirmation. */
export interface CrossPrompt {
  count: number;
  destLabel: string;
  sourceVolume: string;
  run: () => Promise<void>;
}

/** Hotkey slots that can be bound (mirrors the TUI's `is_bindable_hotkey`). */
export const BINDABLE = ["1", "2", "3", "4", "5", "6", "7", "8", "9", "-", "="];

// Reused across the whole inbox sort. `localeCompare` builds a collator on every
// call (very slow over 25k items); a single cached collator is far cheaper and
// `numeric` gives natural ordering (file2 before file10).
const NAME_COLLATOR = new Intl.Collator(undefined, { numeric: true, sensitivity: "base" });

/** Parent directory of a path (handles `\` and `/`), or null at a root. */
function parentOf(path: string): string | null {
  const norm = path.replace(/[\\/]+$/, "");
  const i = Math.max(norm.lastIndexOf("/"), norm.lastIndexOf("\\"));
  return i > 0 ? norm.slice(0, i) : null;
}
/** Normalize a path for comparison (forward slashes, no trailing slash, lower). */
function normPath(p: string): string {
  return p.replace(/\\/g, "/").replace(/\/+$/, "").toLowerCase();
}

class SessionStore {
  input = $state<string | null>(null);
  output = $state<string | null>(null);
  allItems = $state<MediaItem[]>([]);
  destinations = $state<Destination[]>([]);
  cursor = $state(0); // index into `view`
  status = $state("");
  statusKind = $state<StatusKind>("info");
  canUndo = $state(false);
  busy = $state(false);
  error = $state<string | null>(null);

  focus = $state<Focus>("inbox");

  // Inbox sort/filter (mod↓ = newest first, the triage default).
  sortField = $state<SortField>("mod");
  sortOrder = $state<SortOrder>("desc");
  filter = $state<FilterMode>("all");

  // Multiselection (contiguous range, anchored).
  selectedPaths = $state<Set<string>>(new Set());
  selectionAnchor = $state<number | null>(null);

  // Navigator browsing + keyboard cursor.
  nav = $state<FolderListing | null>(null);
  navCursor = $state(0);

  // Navigator fuzzy search mode.
  searching = $state(false);
  searchQuery = $state("");
  searchResults = $state<FolderEntry[]>([]);
  searchCursor = $state(0);
  #searchSeq = 0; // guards out-of-order async results

  // Cross-drive move confirmation (session opt-in like the TUI).
  allowCrossDevice = $state(false);
  crossPrompt = $state<CrossPrompt | null>(null);

  // Inline new-folder prompt, driven by the + button or Ctrl+N.
  creatingFolder = $state(false);

  // Free/total bytes on the output volume (footer readout).
  diskFree = $state<number | null>(null);
  diskTotal = $state<number | null>(null);
  #lastDisk = 0; // throttle timestamp for fetchDisk
  #navRefreshTimer: ReturnType<typeof setTimeout> | null = null;

  // Right-click context menu on an inbox item.
  ctx = $state<{ x: number; y: number; item: MediaItem } | null>(null);

  view = $derived.by<MediaItem[]>(() => {
    let items = this.allItems;
    if (this.filter !== "all") {
      const want = this.filter === "images" ? "image" : "video";
      items = items.filter((i) => i.kind === want);
    }
    const sorted = [...items].sort((a, b) => {
      switch (this.sortField) {
        case "name":
          return NAME_COLLATOR.compare(a.fileName, b.fileName);
        case "size":
          return a.sizeBytes - b.sizeBytes;
        case "mod":
          return (a.modifiedMs ?? 0) - (b.modifiedMs ?? 0);
      }
    });
    if (this.sortOrder === "desc") sorted.reverse();
    return sorted;
  });

  // Memoized so the footer total doesn't re-reduce 25k items on every render.
  viewBytes = $derived(this.view.reduce((a, i) => a + i.sizeBytes, 0));

  get current(): MediaItem | null {
    return this.view[this.cursor] ?? null;
  }
  get total(): number {
    return this.view.length;
  }
  get selectionCount(): number {
    return this.selectedPaths.size || (this.current ? 1 : 0);
  }
  isSelected(path: string): boolean {
    return this.selectedPaths.has(path);
  }

  get sortedTargets(): Destination[] {
    const keyed = this.destinations.filter((d) => d.hotkey && !d.isTrash);
    keyed.sort((a, b) => (a.hotkey! < b.hotkey! ? -1 : 1));
    const trash = this.destinations.filter((d) => d.isTrash);
    return [...keyed, ...trash];
  }
  destForHotkey(hotkey: string): Destination | undefined {
    return this.destinations.find((d) => d.hotkey === hotkey);
  }

  // ---- Inbox selection / sort / filter -------------------------------------

  /** Paths the next op acts on, in view order (selection, or the cursor item). */
  private targetPaths(): string[] {
    if (this.selectedPaths.size > 0) {
      return this.view.filter((i) => this.selectedPaths.has(i.path)).map((i) => i.path);
    }
    return this.current ? [this.current.path] : [];
  }

  select(i: number) {
    if (i < 0 || i >= this.view.length) return;
    this.cursor = i;
    this.clearSelection();
  }
  next() {
    if (this.cursor < this.view.length - 1) this.cursor++;
    this.clearSelection();
  }
  prev() {
    if (this.cursor > 0) this.cursor--;
    this.clearSelection();
  }
  top() {
    this.cursor = 0;
    this.clearSelection();
  }
  bottom() {
    this.cursor = Math.max(0, this.view.length - 1);
    this.clearSelection();
  }
  /** Click a row: plain click selects one; Shift+click selects the range from
   *  the anchor (last single selection / cursor) to the clicked row. */
  clickRow(i: number, shift: boolean) {
    this.focusInbox();
    if (shift) this.selectRangeTo(i);
    else this.select(i);
  }
  private selectRangeTo(i: number) {
    if (i < 0 || i >= this.view.length) return;
    const anchor = this.selectionAnchor ?? this.cursor;
    this.selectionAnchor = anchor;
    this.cursor = i;
    const lo = Math.min(anchor, i);
    const hi = Math.max(anchor, i);
    this.selectedPaths = new Set(this.view.slice(lo, hi + 1).map((x) => x.path));
  }
  clearSelection() {
    if (this.selectedPaths.size) this.selectedPaths = new Set();
    this.selectionAnchor = null;
  }
  /** Shift+↑/↓: extend a contiguous selection from the anchor to the cursor. */
  extendSelection(delta: number) {
    if (this.view.length === 0) return;
    const max = this.view.length - 1;
    const anchor = Math.min(this.selectionAnchor ?? this.cursor, max);
    if (this.selectionAnchor === null) this.selectionAnchor = anchor;
    const nextC = Math.min(Math.max(this.cursor + delta, 0), max);
    this.cursor = nextC;
    const lo = Math.min(anchor, nextC);
    const hi = Math.max(anchor, nextC);
    this.selectedPaths = new Set(this.view.slice(lo, hi + 1).map((i) => i.path));
  }
  cycleSortField() {
    this.sortField = this.sortField === "name" ? "size" : this.sortField === "size" ? "mod" : "name";
    this.clearSelection();
    this.clampCursor();
  }
  toggleSortOrder() {
    this.sortOrder = this.sortOrder === "asc" ? "desc" : "asc";
    this.clearSelection();
    this.clampCursor();
  }
  cycleFilter() {
    this.filter = this.filter === "all" ? "images" : this.filter === "images" ? "videos" : "all";
    this.clearSelection();
    this.clampCursor();
  }
  private clampCursor() {
    if (this.cursor > this.view.length - 1) this.cursor = Math.max(0, this.view.length - 1);
  }

  // ---- Focus ---------------------------------------------------------------

  toggleFocus() {
    this.focus = this.focus === "inbox" ? "navigator" : "inbox";
    if (this.focus === "navigator") this.clampNavCursor();
  }
  focusInbox() {
    this.focus = "inbox";
  }
  focusNavigator() {
    this.focus = "navigator";
    this.clampNavCursor();
  }

  // ---- Session lifecycle ---------------------------------------------------

  async open(input: string, output: string) {
    this.busy = true;
    this.error = null;
    try {
      const view = await api.openSession(input, output);
      this.input = view.input;
      this.output = view.output;
      this.allItems = view.inbox;
      this.destinations = view.destinations;
      this.cursor = 0;
      this.canUndo = false;
      this.focus = "inbox";
      // Apply configured inbox defaults (the user can still change them in-session).
      this.sortField = settings.defaultSortField;
      this.sortOrder = settings.defaultSortOrder;
      this.filter = settings.defaultFilter;
      this.clearSelection();
      this.exitSearch();
      this.creatingFolder = false;
      await this.loadFolders(view.output);
      this.fetchDisk(true);
      this.setStatus(`${view.inbox.length} items to sort`, "info");
    } catch (e) {
      // Shown on the start screen, and as a status if we're already in a session
      // (e.g. a re-picked folder that no longer exists).
      this.error = String(e);
      this.setStatus(`Couldn't open folder: ${e}`, "bad");
    } finally {
      this.busy = false;
    }
  }
  async changeInput() {
    const dir = await api.pickDirectory("Choose the inbox folder to sort");
    if (dir && this.output) await this.open(dir, this.output);
  }
  async changeOutput() {
    const dir = await api.pickDirectory("Choose the destination root");
    if (dir && this.input) await this.open(this.input, dir);
  }

  // ---- Operations (act on the selection, or the cursor item) ---------------

  async moveHotkey(hotkey: string) {
    if (hotkey === "0") {
      await this.runMany(this.targetPaths(), (p) => api.trashItem(p), true);
      return;
    }
    const dest = this.destForHotkey(hotkey);
    if (!dest) {
      this.setStatus(`No destination bound to ${hotkey}`, "info");
      return;
    }
    await this.moveTargetsTo(dest.path, dest.label);
  }
  async copyHotkey(hotkey: string) {
    const dest = this.destForHotkey(hotkey);
    if (!dest || dest.isTrash) return;
    await this.runMany(this.targetPaths(), (p) => api.copyItem(p, dest.path), false);
  }
  async moveToDest(dest: Destination) {
    if (dest.isTrash) await this.runMany(this.targetPaths(), (p) => api.trashItem(p), true);
    else await this.moveTargetsTo(dest.path, dest.label);
  }

  /** Move the active targets into `destDir`, gating cross-drive moves behind a
   *  confirmation prompt (once per session unless the user picks "always"). */
  private async moveTargetsTo(destDir: string, label?: string) {
    const paths = this.targetPaths();
    if (paths.length === 0) return;
    const name = label ?? destDir.replace(/[\\/]+$/, "").split(/[\\/]/).pop() ?? destDir;
    if (settings.confirmCrossDrive && !this.allowCrossDevice && (await api.wouldCrossVolume(paths[0], destDir))) {
      this.crossPrompt = {
        count: paths.length,
        destLabel: name,
        sourceVolume: volumeLabel(paths[0]),
        run: () => this.runMany(paths, (p) => api.moveItem(p, destDir), true),
      };
      this.setStatus("Confirm cross-drive move", "info");
      return;
    }
    await this.runMany(paths, (p) => api.moveItem(p, destDir), true);
  }

  /** Resolve the cross-drive prompt: confirm once, "always" this session, or cancel. */
  async resolveCross(choice: "once" | "always" | "cancel") {
    const prompt = this.crossPrompt;
    this.crossPrompt = null;
    if (choice === "cancel" || !prompt) return;
    if (choice === "always") this.allowCrossDevice = true;
    await prompt.run();
  }

  /** Bind a folder to a hotkey slot. Targets the highlighted folder (or search
   *  match); when nothing is highlighted — e.g. inside a leaf folder with no
   *  subdirectories, or on the ".." row — it binds the directory you're in. */
  async bindHighlighted(hotkey: string) {
    let path: string | undefined;
    let name: string | undefined;
    if (this.searching) {
      path = this.searchSelected?.path;
      name = this.searchSelected?.name;
    } else if (this.navHighlighted) {
      path = this.navHighlighted.path;
      name = this.navHighlighted.name;
    } else if (this.nav) {
      path = this.nav.path;
      name = this.nav.path.replace(/[\\/]+$/, "").split(/[\\/]/).pop() || this.nav.path;
    }
    if (!path) return;
    try {
      this.destinations = await api.bindFolder(path, hotkey);
      this.setStatus(`Bound [${hotkey}] → ${name}`, "good");
    } catch (e) {
      this.setStatus(String(e), "bad");
    }
  }
  async unbind(hotkey: string) {
    try {
      this.destinations = await api.unbindHotkey(hotkey);
      this.setStatus(`Unbound [${hotkey}]`, "info");
    } catch (e) {
      this.setStatus(String(e), "bad");
    }
  }
  async undo() {
    if (!this.canUndo) {
      this.setStatus("Nothing to undo", "info");
      return;
    }
    await this.runOne(() => api.undo());
  }

  // ---- Context menu + item actions -----------------------------------------

  openContext(e: MouseEvent, item: MediaItem, index: number) {
    e.preventDefault();
    this.focusInbox();
    this.select(index);
    this.ctx = { x: e.clientX, y: e.clientY, item };
  }
  closeContext() {
    if (this.ctx) this.ctx = null;
  }
  async openInDefault(path: string) {
    this.closeContext();
    try {
      await openPath(path);
    } catch (e) {
      this.setStatus(`Open failed: ${e}`, "bad");
    }
  }
  async revealInExplorer(path: string) {
    this.closeContext();
    try {
      await revealItemInDir(path);
    } catch (e) {
      this.setStatus(`Reveal failed: ${e}`, "bad");
    }
  }
  async trashPath(path: string) {
    this.closeContext();
    await this.runMany([path], (p) => api.trashItem(p), true);
  }

  /** Re-scan the input directory and adopt the fresh list, keeping the cursor. */
  async refreshInbox() {
    this.closeContext();
    try {
      const items = await api.rescanInbox();
      const curPath = this.current?.path;
      this.allItems = items;
      const i = curPath ? this.view.findIndex((x) => x.path === curPath) : -1;
      this.cursor = i >= 0 ? i : Math.min(this.cursor, Math.max(0, this.view.length - 1));
      this.clearSelection();
      this.fetchDisk(true);
      this.setStatus(`Refreshed — ${items.length} items`, "info");
    } catch (e) {
      this.setStatus(String(e), "bad");
    }
  }

  // ---- Navigator -----------------------------------------------------------

  get navHasParent(): boolean {
    return !!this.nav?.parent;
  }
  get navRowCount(): number {
    return (this.navHasParent ? 1 : 0) + (this.nav?.folders.length ?? 0);
  }
  get navOnParent(): boolean {
    return this.navHasParent && this.navCursor === 0;
  }
  get navHighlighted(): FolderEntry | null {
    if (this.navOnParent) return null;
    const offset = this.navHasParent ? 1 : 0;
    return this.nav?.folders[this.navCursor - offset] ?? null;
  }

  async loadFolders(path: string, preserveCursor = false) {
    const prev = this.navCursor;
    try {
      this.nav = await api.listFolders(path);
      this.navCursor = preserveCursor ? Math.min(prev, Math.max(0, this.navRowCount - 1)) : 0;
    } catch (e) {
      this.setStatus(String(e), "bad");
    }
  }

  /** Debounced refresh of the current Navigator dir after operations. Over a
   *  remote output tree, re-listing + counting on every op is costly, so we
   *  coalesce bursts into one refresh and preserve the cursor position. */
  private scheduleNavRefresh() {
    if (!this.nav) return;
    if (this.#navRefreshTimer != null) clearTimeout(this.#navRefreshTimer);
    this.#navRefreshTimer = setTimeout(() => {
      this.#navRefreshTimer = null;
      if (this.nav) this.loadFolders(this.nav.path, true);
    }, 600);
  }
  navDown() {
    if (this.navCursor < this.navRowCount - 1) this.navCursor++;
  }
  navUp() {
    if (this.navCursor > 0) this.navCursor--;
  }
  navHome() {
    if (this.output) this.loadFolders(this.output);
  }
  async navDrill() {
    if (this.navOnParent) await this.navAscend();
    else if (this.navHighlighted) await this.loadFolders(this.navHighlighted.path);
  }
  async navAscend() {
    if (this.nav?.parent) await this.loadFolders(this.nav.parent);
  }
  private clampNavCursor() {
    if (this.navCursor > this.navRowCount - 1) this.navCursor = Math.max(0, this.navRowCount - 1);
  }

  async navEnterMove() {
    const folder = this.navOnParent ? null : this.navHighlighted;
    const target = this.navOnParent ? this.nav?.path : folder?.path;
    if (!target) return;
    await this.moveTargetsTo(target, folder?.name);
  }
  async navCopy() {
    const target = this.navHighlighted?.path;
    if (!target) return;
    await this.runMany(this.targetPaths(), (p) => api.copyItem(p, target), false);
  }
  async moveInto(folder: FolderEntry) {
    await this.moveTargetsTo(folder.path, folder.name);
  }
  async copyInto(folder: FolderEntry) {
    await this.runMany(this.targetPaths(), (p) => api.copyItem(p, folder.path), false);
  }
  /** Open the inline new-folder prompt (＋ button or Ctrl+N). */
  startNewFolder() {
    this.exitSearch();
    this.focusNavigator();
    this.creatingFolder = true;
  }
  cancelNewFolder() {
    this.creatingFolder = false;
  }
  async createFolderHere(name: string) {
    this.creatingFolder = false;
    const clean = name.trim();
    if (!this.nav || !clean) return;
    const parent = this.nav.path;
    try {
      const created = await api.createFolder(parent, clean);
      await this.loadFolders(parent);
      // Highlight the just-created folder so it's the active selection.
      const target = normPath(created.path);
      const idx = this.nav?.folders.findIndex((f) => normPath(f.path) === target) ?? -1;
      if (idx >= 0) {
        this.focusNavigator();
        this.navCursor = (this.navHasParent ? 1 : 0) + idx;
      }
      this.setStatus(`Created ${clean}`, "good");
    } catch (e) {
      this.setStatus(String(e), "bad");
    }
  }

  /** Refresh the footer disk readout for the output volume. Throttled: free
   *  space barely moves and the query hits the (possibly remote) volume, so
   *  per-op calls are coalesced. `force` bypasses the throttle (session open). */
  async fetchDisk(force = false) {
    if (!this.output) return;
    const now = Date.now();
    if (!force && now - this.#lastDisk < 4000) return;
    this.#lastDisk = now;
    const info = await api.diskSpace(this.output);
    this.diskFree = info?.freeBytes ?? null;
    this.diskTotal = info?.totalBytes ?? null;
  }
  async deleteHighlightedFolder() {
    const folder = this.navHighlighted;
    if (!folder) return;
    const tag = folder.mediaCount + folder.subfolderCount > 0 ? " (not empty)" : "";
    if (settings.confirmFolderDelete && !confirm(`Move "${folder.name}"${tag} to trash? This can be undone.`))
      return;
    await this.runOne(() => api.deleteFolder(folder.path));
    if (this.nav) await this.loadFolders(this.nav.path);
  }

  // ---- Navigator fuzzy search ----------------------------------------------

  startSearch() {
    this.focusNavigator();
    this.searching = true;
    this.searchQuery = "";
    this.searchResults = [];
    this.searchCursor = 0;
  }
  exitSearch() {
    this.searching = false;
    this.searchQuery = "";
    this.searchResults = [];
    this.searchCursor = 0;
  }
  async updateSearch(query: string) {
    this.searchQuery = query;
    const seq = ++this.#searchSeq;
    if (!query.trim()) {
      this.searchResults = [];
      this.searchCursor = 0;
      return;
    }
    try {
      const results = await api.searchFolders(query);
      if (seq === this.#searchSeq) {
        this.searchResults = results;
        this.searchCursor = 0;
      }
    } catch (e) {
      this.setStatus(String(e), "bad");
    }
  }
  searchDown() {
    if (this.searchCursor < this.searchResults.length - 1) this.searchCursor++;
  }
  searchUp() {
    if (this.searchCursor > 0) this.searchCursor--;
  }
  get searchSelected(): FolderEntry | null {
    return this.searchResults[this.searchCursor] ?? null;
  }
  /** Enter in search: move current file(s) into the highlighted match, then
   *  leave search and browse the match's parent with the match highlighted, so
   *  repeated Enter moves into the same folder (mirrors the TUI). */
  async searchMove() {
    const match = this.searchSelected;
    if (!match) return;
    await this.moveTargetsTo(match.path, match.name);
    if (this.crossPrompt) return; // deferred for confirmation; skip the nav jump
    this.exitSearch();
    await this.focusFolderInParent(match.path);
  }
  /** Drill into the highlighted search match and leave search mode. */
  async searchDrill(folder?: FolderEntry) {
    const target = folder ?? this.searchSelected;
    if (!target) return;
    this.exitSearch();
    await this.loadFolders(target.path);
  }
  /** Browse the parent of `folderPath` with that folder highlighted. */
  private async focusFolderInParent(folderPath: string) {
    const parent = parentOf(folderPath);
    if (!parent) return;
    await this.loadFolders(parent);
    const target = normPath(folderPath);
    const idx = this.nav?.folders.findIndex((f) => normPath(f.path) === target) ?? -1;
    if (idx >= 0) this.navCursor = (this.navHasParent ? 1 : 0) + idx;
  }

  // ---- Internal ------------------------------------------------------------

  /** Single-outcome op (undo, folder delete). */
  private async runOne(op: () => Promise<OpOutcome>) {
    if (this.busy) return;
    this.busy = true;
    try {
      const out = await op();
      this.applyOutcome(out);
      this.setStatus(out.message, kindToStatus(out.kind));
      this.scheduleNavRefresh();
      this.fetchDisk();
    } catch (e) {
      this.setStatus(String(e), "bad");
    } finally {
      this.busy = false;
    }
  }

  /** Batch op over `paths`; each call is journaled + individually undoable. */
  private async runMany(
    paths: string[],
    op: (path: string) => Promise<OpOutcome>,
    clearSel: boolean,
  ) {
    if (this.busy || paths.length === 0) return;
    this.busy = true;
    const removed = new Set<string>();
    let done = 0;
    let last: OpOutcome | null = null;
    try {
      for (const p of paths) {
        last = await op(p);
        if (last.sourceRemoved) removed.add(last.sourcePath);
        done++;
      }
      // Single removal pass + one view re-derivation, instead of rebuilding the
      // (potentially 25k-item) array once per moved file.
      if (removed.size) {
        this.allItems = this.allItems.filter((i) => !removed.has(i.path));
        this.clampCursor();
      }
      if (last) {
        this.destinations = last.destinations;
        this.canUndo = last.canUndo;
        const verb = last.kind === "copy" ? "Copied" : last.kind === "trash" ? "Trashed" : "Moved";
        this.setStatus(done > 1 ? `${verb} ${done} items` : last.message, kindToStatus(last.kind));
      }
      if (clearSel) this.clearSelection();
      this.scheduleNavRefresh();
      this.fetchDisk();
    } catch (e) {
      this.setStatus(String(e), "bad");
    } finally {
      this.busy = false;
    }
  }

  private applyOutcome(out: OpOutcome) {
    this.destinations = out.destinations;
    this.canUndo = out.canUndo;
    if (out.sourceRemoved) {
      const idx = this.allItems.findIndex((i) => i.path === out.sourcePath);
      if (idx >= 0) {
        this.allItems = [...this.allItems.slice(0, idx), ...this.allItems.slice(idx + 1)];
      }
      this.clampCursor();
    }
    if (out.restoredItem) {
      this.allItems = [...this.allItems, out.restoredItem];
      const i = this.view.findIndex((x) => x.path === out.restoredItem!.path);
      if (i >= 0) this.cursor = i;
    }
  }

  private setStatus(msg: string, kind: StatusKind) {
    this.status = msg;
    this.statusKind = kind;
  }
}

function kindToStatus(kind: OpKind): StatusKind {
  return kind === "move" || kind === "copy" ? "good" : "info";
}

export const session = new SessionStore();
