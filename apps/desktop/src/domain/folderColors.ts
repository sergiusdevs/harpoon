// Colores de carpeta: los de la paleta tienen un tono para temas oscuros y
// otro para claros, legibles sobre el fondo de la barra lateral.
export const FOLDER_COLORS = [
  { id: "red", name: "Rojo", dark: "#f07a72", light: "#c6362c" },
  { id: "orange", name: "Naranja", dark: "#f0a060", light: "#b8560e" },
  { id: "yellow", name: "Amarillo", dark: "#e8c560", light: "#8f6b00" },
  { id: "green", name: "Verde", dark: "#7cc77f", light: "#2e7d32" },
  { id: "teal", name: "Turquesa", dark: "#5ec8c0", light: "#0f7a73" },
  { id: "blue", name: "Azul", dark: "#7aaaf2", light: "#2563c9" },
  { id: "purple", name: "Morado", dark: "#b796f2", light: "#6f3fc4" },
  { id: "pink", name: "Rosa", dark: "#ee8fc0", light: "#b5357a" },
  { id: "gray", name: "Gris", dark: "#a3adb8", light: "#5d6670" },
] as const;

/** Valor CSS de un color de carpeta (o `null` si no tiene). */
export function folderColor(color: string | null | undefined, kind: "dark" | "light"): string | null {
  if (!color) return null;
  if (/^#[0-9a-f]{6}$/i.test(color)) return color;
  const named = FOLDER_COLORS.find((c) => c.id === color);
  return named ? named[kind] : null;
}
