export const interfaceFonts = [
  { id: "system", name: "Sistema" },
  { id: "inter", name: "Inter" },
  { id: "rounded", name: "Redondeada" },
  { id: "serif", name: "Serif" },
] as const;
export const editorFonts = [
  { id: "default", name: "Predeterminada" },
  { id: "jetbrains", name: "JetBrains Mono" },
  { id: "fira", name: "Fira Code" },
  { id: "source", name: "Source Code Pro" },
  { id: "system", name: "Monoespaciada del sistema" },
] as const;
export type InterfaceFont = (typeof interfaceFonts)[number]["id"];
export type EditorFont = (typeof editorFonts)[number]["id"];
