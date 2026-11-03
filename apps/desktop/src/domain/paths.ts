/** `a/b/c` → `["a", "a/b"]` (las carpetas que contienen la clave). */
export function folderChain(key: string): string[] {
  const parts = key.split("/");
  const out: string[] = [];
  for (let i = 1; i < parts.length; i++) out.push(parts.slice(0, i).join("/"));
  return out;
}

