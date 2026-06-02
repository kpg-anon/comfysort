// Theme presets selectable in Settings. Each `id` maps to a [data-theme="id"]
// block in theme.css; `swatches` are representative colors for the picker
// preview (panel bg, then accent roles). comfy-dark is the :root default.
export interface ThemePreset {
  id: string;
  name: string;
  swatches: string[];
}

export const THEMES: ThemePreset[] = [
  { id: "comfy-dark", name: "Comfy Dark", swatches: ["#141920", "#c287ff", "#82d65a", "#57c7e3"] },
  { id: "nord", name: "Nord", swatches: ["#323947", "#b48ead", "#a3be8c", "#88c0d0"] },
  { id: "gruvbox", name: "Gruvbox", swatches: ["#282828", "#d3869b", "#b8bb26", "#fabd2f"] },
  { id: "catppuccin", name: "Catppuccin", swatches: ["#242438", "#cba6f7", "#a6e3a1", "#89b4fa"] },
];
