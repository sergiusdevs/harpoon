export const themes = [
  { id: "system", name: "Automático", detail: "Sigue el modo del sistema", kind: "system", bg: "#2b313c", panel: "#f6f6f4", accent: "#7cc4c8", text: "#f3f6f9" },
  { id: "dark", name: "Harpoon Dark", detail: "El tema original", kind: "dark", bg: "#2b313c", panel: "#343b47", accent: "#7cc4c8", text: "#f3f6f9" },
  { id: "light", name: "Harpoon Light", detail: "Claro y limpio", kind: "light", bg: "#f6f6f4", panel: "#ffffff", accent: "#1f6f8b", text: "#1e2024" },
  { id: "dark-plus", name: "Dark+", detail: "Inspirado en VS Code", kind: "dark", bg: "#1e1e1e", panel: "#252526", accent: "#569cd6", text: "#d4d4d4" },
  { id: "dracula", name: "Dracula", detail: "Púrpura y vibrante", kind: "dark", bg: "#282a36", panel: "#343746", accent: "#bd93f9", text: "#f8f8f2" },
  { id: "nord", name: "Nord", detail: "Azules suaves", kind: "dark", bg: "#2e3440", panel: "#3b4252", accent: "#88c0d0", text: "#eceff4" },
  { id: "monokai", name: "Monokai", detail: "Contraste cálido", kind: "dark", bg: "#272822", panel: "#34352c", accent: "#a6e22e", text: "#f8f8f2" },
  { id: "tokyo-night", name: "Tokyo Night", detail: "Azul nocturno", kind: "dark", bg: "#1a1b26", panel: "#24283b", accent: "#7aa2f7", text: "#c0caf5" },
  { id: "one-dark", name: "One Dark", detail: "Clásico de Atom", kind: "dark", bg: "#282c34", panel: "#2c313a", accent: "#61afef", text: "#d7dae0" },
  { id: "github-dark", name: "GitHub Dark", detail: "Sobrio y nítido", kind: "dark", bg: "#0d1117", panel: "#161b22", accent: "#4493f8", text: "#e6edf3" },
  { id: "catppuccin-mocha", name: "Catppuccin Mocha", detail: "Pastel nocturno", kind: "dark", bg: "#1e1e2e", panel: "#2a2b3d", accent: "#cba6f7", text: "#cdd6f4" },
  { id: "gruvbox-dark", name: "Gruvbox Dark", detail: "Retro y cálido", kind: "dark", bg: "#282828", panel: "#32302f", accent: "#fe8019", text: "#ebdbb2" },
  { id: "solarized-dark", name: "Solarized Dark", detail: "Precisión clásica", kind: "dark", bg: "#002b36", panel: "#073642", accent: "#2aa198", text: "#c4cfcf" },
  { id: "rose-pine", name: "Rosé Pine", detail: "Elegante y suave", kind: "dark", bg: "#191724", panel: "#1f1d2e", accent: "#c4a7e7", text: "#e0def4" },
  { id: "kanagawa", name: "Kanagawa", detail: "Inspirado en la ola", kind: "dark", bg: "#1f1f28", panel: "#2a2a37", accent: "#7e9cd8", text: "#dcd7ba" },
  { id: "everforest-dark", name: "Everforest Dark", detail: "Bosque relajado", kind: "dark", bg: "#2d353b", panel: "#343f44", accent: "#a7c080", text: "#d3c6aa" },
  { id: "ayu-mirage", name: "Ayu Mirage", detail: "Cálido y moderno", kind: "dark", bg: "#1f2430", panel: "#262b38", accent: "#ffcc66", text: "#cccac2" },
  { id: "night-owl", name: "Night Owl", detail: "Para trasnochar", kind: "dark", bg: "#011627", panel: "#0b2942", accent: "#82aaff", text: "#d6deeb" },
  { id: "github-light", name: "GitHub Light", detail: "Claro de toda la vida", kind: "light", bg: "#f6f8fa", panel: "#fbfcfd", accent: "#0969da", text: "#1f2328" },
  { id: "catppuccin-latte", name: "Catppuccin Latte", detail: "Pastel de día", kind: "light", bg: "#eff1f5", panel: "#f5f6f9", accent: "#8839ef", text: "#4c4f69" },
  { id: "solarized-light", name: "Solarized Light", detail: "Papel cálido", kind: "light", bg: "#fdf6e3", panel: "#fefbf1", accent: "#2075b5", text: "#3c4a50" },
  { id: "gruvbox-light", name: "Gruvbox Light", detail: "Retro de día", kind: "light", bg: "#fbf1c7", panel: "#fdf5d9", accent: "#af3a03", text: "#3c3836" },
  { id: "rose-pine-dawn", name: "Rosé Pine Dawn", detail: "Amanecer suave", kind: "light", bg: "#faf4ed", panel: "#fffaf3", accent: "#286983", text: "#575279" },
  { id: "one-light", name: "One Light", detail: "Atom de día", kind: "light", bg: "#fafafa", panel: "#fdfdfd", accent: "#4078f2", text: "#383a42" },
] as const;

export type Theme = (typeof themes)[number]["id"];

export type ThemeKind = (typeof themes)[number]["kind"];

/** Tema que se aplica de verdad: "system" se resuelve según el modo del sistema. */
export function resolveTheme(theme: Theme, prefersDark: boolean): Exclude<Theme, "system"> {
  return theme === "system" ? (prefersDark ? "dark" : "light") : theme;
}
