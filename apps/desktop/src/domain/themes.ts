export const themes = [
  { id: "dark", name: "Harpoon Dark", detail: "El tema original", bg: "#17191d", panel: "#24282d", accent: "#4fa3c4", text: "#e2e5e8" },
  { id: "light", name: "Harpoon Light", detail: "Claro y limpio", bg: "#f6f6f4", panel: "#ffffff", accent: "#1f6f8b", text: "#1e2024" },
  { id: "dark-plus", name: "Dark+", detail: "Inspirado en VS Code", bg: "#1e1e1e", panel: "#252526", accent: "#569cd6", text: "#d4d4d4" },
  { id: "dracula", name: "Dracula", detail: "Púrpura y vibrante", bg: "#282a36", panel: "#343746", accent: "#bd93f9", text: "#f8f8f2" },
  { id: "nord", name: "Nord", detail: "Azules suaves", bg: "#2e3440", panel: "#3b4252", accent: "#88c0d0", text: "#eceff4" },
  { id: "monokai", name: "Monokai", detail: "Contraste cálido", bg: "#272822", panel: "#34352c", accent: "#a6e22e", text: "#f8f8f2" },
  { id: "tokyo-night", name: "Tokyo Night", detail: "Azul nocturno", bg: "#1a1b26", panel: "#24283b", accent: "#7aa2f7", text: "#c0caf5" },
] as const;

export type Theme = (typeof themes)[number]["id"];
