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
  history: "", // nf-fa-history
  edit: "", // nf-fa-pencil_square_o (rename)
  keyboard: "", // nf-fa-keyboard
} as const;

export function kindIcon(kind: MediaKind): string {
  return kind === "image" ? I.image : kind === "video" ? I.video : I.file;
}
