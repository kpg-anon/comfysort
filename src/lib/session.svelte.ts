// Central reactive session state (Svelte 5 runes). One instance drives the
// whole UI. Mutating actions call the backend, then apply the returned delta
// locally so the inbox never has to be re-serialized wholesale.
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

class SessionStore {
  input = $state<string | null>(null);
  output = $state<string | null>(null);
  inbox = $state<MediaItem[]>([]);
  destinations = $state<Destination[]>([]);
  cursor = $state(0);
  status = $state("");
  statusKind = $state<StatusKind>("info");
  canUndo = $state(false);
  busy = $state(false);
  error = $state<string | null>(null);

  /** Navigator: the directory currently being browsed under the output root. */
  nav = $state<FolderListing | null>(null);

  get current(): MediaItem | null {
    return this.inbox[this.cursor] ?? null;
  }
  get total(): number {
    return this.inbox.length;
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

  select(i: number) {
    if (i < 0 || i >= this.inbox.length) return;
    this.cursor = i;
  }
  next() {
    if (this.cursor < this.inbox.length - 1) this.cursor++;
  }
  prev() {
    if (this.cursor > 0) this.cursor--;
  }

  async open(input: string, output: string) {
    this.busy = true;
    this.error = null;
    try {
      const view = await api.openSession(input, output);
      this.input = view.input;
      this.output = view.output;
      this.inbox = view.inbox;
      this.destinations = view.destinations;
      this.cursor = 0;
      this.canUndo = false;
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

  // ---- Navigator -----------------------------------------------------------

  async loadFolders(path: string) {
    try {
      this.nav = await api.listFolders(path);
    } catch (e) {
      this.setStatus(String(e), "bad");
    }
  }
  drillInto(folder: FolderEntry) {
    this.loadFolders(folder.path);
  }
  navUp() {
    if (this.nav?.parent) this.loadFolders(this.nav.parent);
  }
  navHome() {
    if (this.output) this.loadFolders(this.output);
  }

  /** Move the current item into a Navigator folder. */
  async moveInto(folder: FolderEntry) {
    const item = this.current;
    if (!item) return;
    await this.run(() => api.moveItem(item.path, folder.path));
  }
  /** Copy the current item into a Navigator folder (source stays in inbox). */
  async copyInto(folder: FolderEntry) {
    const item = this.current;
    if (!item) return;
    await this.run(() => api.copyItem(item.path, folder.path));
  }
  /** Create a folder inside the directory the Navigator is showing. */
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

  /** Move/trash via a hotkey slot (0 = trash). */
  async moveHotkey(hotkey: string) {
    const item = this.current;
    if (!item) return;
    await this.run(() => api.moveToHotkey(item.path, hotkey));
  }

  /** Copy the current item into the hotkey's destination (source stays). */
  async copyHotkey(hotkey: string) {
    const item = this.current;
    if (!item) return;
    const dest = this.destForHotkey(hotkey);
    if (!dest || dest.isTrash) return;
    await this.run(() => api.copyItem(item.path, dest.path));
  }

  /** Move the current item into an explicit destination (click on a target). */
  async moveToDest(dest: Destination) {
    const item = this.current;
    if (!item) return;
    if (dest.isTrash) {
      await this.run(() => api.trashItem(item.path));
    } else {
      await this.run(() => api.moveItem(item.path, dest.path));
    }
  }

  async undo() {
    if (!this.canUndo) {
      this.setStatus("Nothing to undo", "info");
      return;
    }
    await this.run(() => api.undo());
  }

  /** Run a mutating op and reconcile local state from the outcome. */
  private async run(op: () => Promise<OpOutcome>) {
    if (this.busy) return;
    this.busy = true;
    try {
      const out = await op();
      this.applyOutcome(out);
      this.setStatus(out.message, kindToStatus(out.kind));
      // Keep Navigator media counts in sync with what just moved/copied.
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
      const idx = this.inbox.findIndex((i) => i.path === out.sourcePath);
      if (idx >= 0) {
        this.inbox = [...this.inbox.slice(0, idx), ...this.inbox.slice(idx + 1)];
        // Sticky cursor: keep the same row index so the next file slides in.
        if (this.cursor > this.inbox.length - 1) {
          this.cursor = Math.max(0, this.inbox.length - 1);
        }
      }
    }
    if (out.restoredItem) {
      const at = Math.min(this.cursor, this.inbox.length);
      this.inbox = [...this.inbox.slice(0, at), out.restoredItem, ...this.inbox.slice(at)];
      this.cursor = at;
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
    case "trash":
    case "undo":
      return "info";
    default:
      return "info";
  }
}

export const session = new SessionStore();
