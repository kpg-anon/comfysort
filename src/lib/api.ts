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

  openSession: (input: string, output: string, recursive: boolean): Promise<SessionView> =>
    invoke("open_session", { input, output, recursive }),

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

  deleteFolder: (path: string): Promise<OpOutcome> =>
    invoke("delete_folder", { path }),

  searchFolders: (query: string): Promise<FolderEntry[]> =>
    invoke("search_folders", { query }),

  bindFolder: (path: string, hotkey: string): Promise<Destination[]> =>
    invoke("bind_folder", { path, hotkey }),

  bindPath: (path: string, hotkey: string): Promise<Destination[]> =>
    invoke("bind_path", { path, hotkey }),

  renameFolder: (path: string, newName: string): Promise<FolderListing> =>
    invoke("rename_folder", { path, newName }),

  revertOp: (source: string, resolved: string): Promise<OpOutcome> =>
    invoke("revert_op", { source, resolved }),

  unbindHotkey: (hotkey: string): Promise<Destination[]> =>
    invoke("unbind_hotkey", { hotkey }),

  wouldCrossVolume: (source: string, destDir: string): Promise<boolean> =>
    invoke("would_cross_volume", { source, destDir }),

  diskSpace: (path: string): Promise<DiskSpace | null> =>
    invoke("disk_space", { path }),

  rescanInbox: (): Promise<MediaItem[]> => invoke("rescan_inbox", {}),

  getSettings: (): Promise<Settings> => invoke("get_settings", {}),
  configFilePath: (): Promise<string> => invoke("config_file_path", {}),
  setSettings: (settings: Settings): Promise<void> => invoke("set_settings", { settings }),
  setCollisionPolicy: (policy: string): Promise<void> =>
    invoke("set_collision_policy", { policy }),
  setRecursiveInbox: (recursive: boolean): Promise<void> =>
    invoke("set_recursive_inbox", { recursive }),
};

export interface DiskSpace {
  freeBytes: number;
  totalBytes: number;
}

export type CollisionPolicyName = "rename" | "skip" | "overwrite";

export interface Settings {
  collisionPolicy: CollisionPolicyName;
  confirmFolderDelete: boolean;
  confirmCrossDrive: boolean;
  defaultSortField: "name" | "size" | "mod";
  defaultSortOrder: "asc" | "desc";
  defaultFilter: "all" | "images" | "videos";
  videoAutoplay: boolean;
  videoLoop: boolean;
  videoMuted: boolean;
  autoUpdateCheck: boolean;
  recursiveInbox: boolean;
  theme: string;
  defaultInput: string;
  defaultOutput: string;
}

export const DEFAULT_SETTINGS: Settings = {
  collisionPolicy: "rename",
  confirmFolderDelete: true,
  confirmCrossDrive: true,
  defaultSortField: "mod",
  defaultSortOrder: "desc",
  defaultFilter: "all",
  videoAutoplay: true,
  videoLoop: true,
  videoMuted: true,
  autoUpdateCheck: true,
  recursiveInbox: false,
  theme: "comfy-dark",
  defaultInput: "",
  defaultOutput: "",
};

/** The drive/share prefix of a path, for the cross-drive prompt (e.g. "X:"). */
export function volumeLabel(path: string): string {
  const m = path.match(/^[a-zA-Z]:/) || path.match(/^\\\\[^\\]+\\[^\\]+/);
  return m ? m[0] : "another drive";
}

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
