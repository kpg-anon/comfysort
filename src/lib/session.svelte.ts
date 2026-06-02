// Central reactive session state (Svelte 5 runes). One instance drives the
// whole UI. Mutating actions call the backend, then apply the returned delta
// locally so the inbox never has to be re-serialized wholesale.
//
// Inbox model mirrors the TUI: `allItems` is the source of truth; `view` is the
// derived sort+filter projection the UI renders and the cursor indexes into.
import {
  api,
  type Destination,
  type FolderEntry,
  type FolderListing,
  type MediaItem,
  type OpKind,
  type OpOutcome,
} from "./api";

type StatusKind = "info" | "good" | "bad";
export type Focus = "inbox" | "navigator";
export type SortField = "name" | "size" | "mod";
export type SortOrder = "asc" | "desc";
export type FilterMode = "all" | "images" | "videos";

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

  // Which pane has keyboard focus.
  focus = $state<Focus>("inbox");

  // Inbox sort/filter (mod↓ = newest first, the triage default).
  sortField = $state<SortField>("mod");
  sortOrder = $state<SortOrder>("desc");
  filter = $state<FilterMode>("all");

  // Navigator browsing state + keyboard cursor.
  nav = $state<FolderListing | null>(null);
  navCursor = $state(0); // index into navRows (".." counts as row 0 when present)

  // Derived sort+filter projection of the inbox.
  view = $derived.by<MediaItem[]>(() => {
    let items = this.allItems;
    if (this.filter !== "all") {
      const want = this.filter === "images" ? "image" : "video";
      items = items.filter((i) => i.kind === want);
    }
    const sorted = [...items].sort((a, b) => {
      switch (this.sortField) {
        case "name":
          return a.fileName.localeCompare(b.fileName);
        case "size":
          return a.sizeBytes - b.sizeBytes;
        case "mod":
          return (a.modifiedMs ?? 0) - (b.modifiedMs ?? 0);
      }
    });
    if (this.sortOrder === "desc") sorted.reverse();
    return sorted;
  });

  get current(): MediaItem | null {
    return this.view[this.cursor] ?? null;
  }
  get total(): number {
    return this.view.length;
  }
  get viewBytes(): number {
    return this.view.reduce((a, i) => a + i.sizeBytes, 0);
  }

  /** Sort targets in hotkey order (1-9 then 0/trash), trash last. */
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

  select(i: number) {
    if (i < 0 || i >= this.view.length) return;
    this.cursor = i;
  }
  next() {
    if (this.cursor < this.view.length - 1) this.cursor++;
  }
  prev() {
    if (this.cursor > 0) this.cursor--;
  }
  top() {
    this.cursor = 0;
  }
  cycleSortField() {
    this.sortField = this.sortField === "name" ? "size" : this.sortField === "size" ? "mod" : "name";
    this.clampCursor();
  }
  toggleSortOrder() {
    this.sortOrder = this.sortOrder === "asc" ? "desc" : "asc";
    this.clampCursor();
  }
  cycleFilter() {
    this.filter = this.filter === "all" ? "images" : this.filter === "images" ? "videos" : "all";
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
      await this.loadFolders(view.output);
      this.setStatus(`${view.inbox.length} items to sort`, "info");
    } catch (e) {
      this.error = String(e);
    } finally {
      this.busy = false;
    }
  }

  /** Re-pick the inbox folder mid-session (keeps the same destination root). */
  async changeInput() {
    const dir = await api.pickDirectory("Choose the inbox folder to sort");
    if (dir && this.output) await this.open(dir, this.output);
  }
  /** Re-pick the destination root mid-session (keeps the same inbox). */
  async changeOutput() {
    const dir = await api.pickDirectory("Choose the destination root");
    if (dir && this.input) await this.open(this.input, dir);
  }

  // ---- Operations (hotkeys / clicks) ---------------------------------------

  async moveHotkey(hotkey: string) {
    const item = this.current;
    if (!item) return;
    await this.run(() => api.moveToHotkey(item.path, hotkey));
  }
  async copyHotkey(hotkey: string) {
    const item = this.current;
    if (!item) return;
    const dest = this.destForHotkey(hotkey);
    if (!dest || dest.isTrash) return;
    await this.run(() => api.copyItem(item.path, dest.path));
  }
  async moveToDest(dest: Destination) {
    const item = this.current;
    if (!item) return;
    if (dest.isTrash) await this.run(() => api.trashItem(item.path));
    else await this.run(() => api.moveItem(item.path, dest.path));
  }
  async undo() {
    if (!this.canUndo) {
      this.setStatus("Nothing to undo", "info");
      return;
    }
    await this.run(() => api.undo());
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
  /** The folder under the nav cursor, or null when the cursor is on "..". */
  get navHighlighted(): FolderEntry | null {
    if (this.navOnParent) return null;
    const offset = this.navHasParent ? 1 : 0;
    return this.nav?.folders[this.navCursor - offset] ?? null;
  }

  async loadFolders(path: string) {
    try {
      this.nav = await api.listFolders(path);
      this.navCursor = 0;
    } catch (e) {
      this.setStatus(String(e), "bad");
    }
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
  /** → drill into the highlighted folder (or ascend when on ".."). */
  async navDrill() {
    if (this.navOnParent) await this.navAscend();
    else if (this.navHighlighted) await this.loadFolders(this.navHighlighted.path);
  }
  /** ← ascend to the parent directory. */
  async navAscend() {
    if (this.nav?.parent) await this.loadFolders(this.nav.parent);
  }
  private clampNavCursor() {
    if (this.navCursor > this.navRowCount - 1) this.navCursor = Math.max(0, this.navRowCount - 1);
  }

  /** Enter: move the current file into the highlighted folder (or current dir on ".."). */
  async navEnterMove() {
    const item = this.current;
    if (!item) return;
    const target = this.navOnParent ? this.nav?.path : this.navHighlighted?.path;
    if (!target) return;
    await this.run(() => api.moveItem(item.path, target));
  }
  /** Shift+D: copy the current file into the highlighted folder (source stays). */
  async navCopy() {
    const item = this.current;
    if (!item) return;
    const target = this.navHighlighted?.path;
    if (!target) return;
    await this.run(() => api.copyItem(item.path, target));
  }
  /** Click/keyboard move into an explicit folder. */
  async moveInto(folder: FolderEntry) {
    const item = this.current;
    if (!item) return;
    await this.run(() => api.moveItem(item.path, folder.path));
  }
  async copyInto(folder: FolderEntry) {
    const item = this.current;
    if (!item) return;
    await this.run(() => api.copyItem(item.path, folder.path));
  }
  async createFolderHere(name: string) {
    if (!this.nav || !name.trim()) return;
    try {
      await api.createFolder(this.nav.path, name.trim());
      await this.loadFolders(this.nav.path);
      this.setStatus(`Created ${name.trim()}`, "good");
    } catch (e) {
      this.setStatus(String(e), "bad");
    }
  }
  /** Ctrl+D / button: delete the highlighted folder to trash (reversible). */
  async deleteHighlightedFolder() {
    const folder = this.navHighlighted;
    if (!folder) return;
    const label = folder.mediaCount + folder.subfolderCount > 0 ? " (not empty)" : "";
    if (!confirm(`Move "${folder.name}"${label} to trash? This can be undone.`)) return;
    await this.run(() => api.deleteFolder(folder.path));
    if (this.nav) await this.loadFolders(this.nav.path);
  }

  // ---- Internal ------------------------------------------------------------

  private async run(op: () => Promise<OpOutcome>) {
    if (this.busy) return;
    this.busy = true;
    try {
      const out = await op();
      this.applyOutcome(out);
      this.setStatus(out.message, kindToStatus(out.kind));
      if (this.nav) await this.loadFolders(this.nav.path);
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
      // Sticky cursor: keep the same row index so the next file slides in.
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
  switch (kind) {
    case "move":
    case "copy":
      return "good";
    default:
      return "info";
  }
}

export const session = new SessionStore();
