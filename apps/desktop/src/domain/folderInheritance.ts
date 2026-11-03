// Lo que una solicitud (o carpeta) hereda de sus carpetas: igual que
// `Collection::with_folder_inheritance` en harpoon-core.
import type { Auth, Collection } from "./types";

export interface InheritedHeader { name: string; value: string; source: string }

/** Cabeceras de las carpetas indicadas (de fuera hacia dentro); la interior sustituye a la exterior. */
export function inheritedHeaders(col: Collection, folders: string[]): InheritedHeader[] {
  const out: InheritedHeader[] = [];
  for (const folder of folders) {
    for (const [name, value] of Object.entries(col.folder_configs[folder]?.headers ?? {})) {
      const i = out.findIndex((h) => h.name.toLowerCase() === name.toLowerCase());
      if (i >= 0) out.splice(i, 1);
      out.push({ name, value, source: `Carpeta ${folder}` });
    }
  }
  return out;
}

/** Auth efectiva al heredar: la de la carpeta más cercana que defina una, o la de la colección. */
export function inheritedAuth(col: Collection, folders: string[]): { auth: Auth; source: string } {
  for (const folder of [...folders].reverse()) {
    const auth = col.folder_configs[folder]?.auth;
    if (auth && auth.type !== "inherit") return { auth, source: `carpeta ${folder}` };
  }
  return { auth: col.meta.auth, source: "la colección" };
}
