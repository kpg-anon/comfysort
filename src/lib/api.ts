// Typed wrapper around the Tauri IPC surface. Mirrors src-tauri/src/commands.rs
// and the DTOs in crates/engine/src/domain.rs. See ARCHITECTURE.md.
import { invoke } from "@tauri-apps/api/core";

export type MediaKind = "image" | "video" | "other";

export interface MediaItem {
  path: string;
  fileName: string;
  kind: MediaKind;
  sizeBytes: number;
  modifiedMs: number | null;
}

export interface Destination {
  label: string;
  path: string;
  hotkey: string | null;
  isTrash: boolean;
  mediaCount: number;
}

export interface SessionView {
  input: string;
  output: string;
  inbox: MediaItem[];
  destinations: Destination[];
}

export interface FolderEntry {
  name: string;
  path: string;
  mediaCount: number;
  subfolderCount: number;
}

export interface FolderListing {
  path: string;
  parent: string | null;
  rel: string;
  folders: FolderEntry[];
}

export type OpKind = "move" | "copy" | "trash" | "undo";

export interface OpOutcome {
  message: string;
  kind: OpKind;
  sourcePath: string;
  resolvedPath: string;
  sourceRemoved: boolean;
  restoredItem: MediaItem | null;
  canUndo: boolean;
  destinations: Destination[];
}

export const api = {
  pickDirectory: (title?: string): Promise<string | null> =>
    invoke("pick_directory", { title }),

  openSession: (input: string, output: string): Promise<SessionView> =>
    invoke("open_session", { input, output }),

  moveItem: (source: string, destDir: string): Promise<OpOutcome> =>
    invoke("move_item", { source, destDir }),

  copyItem: (source: string, destDir: string): Promise<OpOutcome> =>
    invoke("copy_item", { source, destDir }),

  moveToHotkey: (source: string, hotkey: string): Promise<OpOutcome> =>
    invoke("move_to_hotkey", { source, hotkey }),

  trashItem: (source: string): Promise<OpOutcome> =>
    invoke("trash_item", { source }),

  createFolder: (parent: string, name: string): Promise<Destination> =>
    invoke("create_folder", { parent, name }),

  undo: (): Promise<OpOutcome> => invoke("undo", {}),

  listFolders: (path: string): Promise<FolderListing> =>
    invoke("list_folders", { path }),
};

/** Human-readable byte size, e.g. 1.4 MB. */
export function humanSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KB", "MB", "GB", "TB"];
  let v = bytes / 1024;
  let i = 0;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return `${v < 10 ? v.toFixed(1) : Math.round(v)} ${units[i]}`;
}

/** Lowercase extension without the dot, for chip styling. */
export function extOf(name: string): string {
  const dot = name.lastIndexOf(".");
  return dot >= 0 ? name.slice(dot + 1).toLowerCase() : "";
}
