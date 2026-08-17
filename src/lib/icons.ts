// Nerd Font icon glyphs (private-use-area codepoints), rendered with the `.nf`
// class (see theme.css). Codepoints are stable FontAwesome-in-Nerd-Font values.
import type { MediaKind } from "./api";

export const I = {
  folder: "", // nf-fa-folder
  folderOpen: "", // nf-fa-folder_open
  image: "", // nf-fa-picture_o
  video: "", // nf-fa-video_camera
  file: "", // nf-fa-file
  trash: "", // nf-fa-trash
  search: "", // nf-fa-search
  undo: "", // nf-fa-undo
  copy: "", // nf-fa-copy
  arrowRight: "", // nf-fa-arrow_right
  plus: "", // nf-fa-plus
  close: "", // nf-fa-times
  chevronRight: "", // nf-fa-chevron_right
  levelUp: "", // nf-fa-level_up
  tag: "", // nf-fa-tag
  drive: "", // nf-fa-hdd_o
  inbox: "", // nf-fa-inbox
  cog: "", // nf-fa-cog
  warn: "", // nf-fa-exclamation_triangle
  refresh: "", // nf-fa-refresh
  eye: "", // nf-fa-eye
  eyeSlash: "", // nf-fa-eye_slash (ignored folder)
  history: "", // nf-fa-history
  edit: "", // nf-fa-pencil_square_o (rename)
  keyboard: "", // nf-fa-keyboard
  folderPlus: "", // nf-fa-folder_plus (add another input)
  check: "", // nf-fa-check (choice-card selected badge)
  // Recursion choice-card glyphs (built via fromCodePoint — the md one is in the
  // supplementary plane). Codepoints verified against the bundled font's cmap.
  folderSolid: String.fromCodePoint(0xf024b), // nf-md-folder (top-level only)
  folderTree: String.fromCodePoint(0xef81), // nf-fa-folder_tree (recursive / nested)
  // Cross-drive move choice-card glyphs (verified against the bundled font cmap).
  imageMove: String.fromCodePoint(0xf09f8), // nf-md-image_move (move once)
  pinOutline: String.fromCodePoint(0xf0931), // nf-md-pin_outline (always this session)
  deleteForever: String.fromCodePoint(0xf05e8), // nf-md-delete_forever (delete to trash)
  folderLock: String.fromCodePoint(0xf0250), // nf-md-folder_lock (keep folder)
} as const;

export function kindIcon(kind: MediaKind): string {
  return kind === "image" ? I.image : kind === "video" ? I.video : I.file;
}
