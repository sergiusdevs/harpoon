// Ámbitos donde se puede guardar un secreto para la solicitud abierta.
import { store } from "../../state/store.svelte";
import { folderOf } from "../../domain/secrets";
import type { SecretScope } from "../../domain/types";

export interface ScopeOption { id: string; label: string; scope: SecretScope }

/** Entorno activo (si hay), colección y carpeta de la solicitud (si está en una). El primero es el predeterminado. */
export function scopeOptions(): ScopeOption[] {
  const options: ScopeOption[] = [];
  if (store.env) options.push({ id: "environment", label: `Entorno ${store.env}`, scope: { kind: "environment", name: store.env } });
  options.push({ id: "collection", label: "Colección", scope: { kind: "collection" } });
  const folder = folderOf(store.selected);
  if (folder) options.push({ id: "folder", label: `Carpeta ${folder}`, scope: { kind: "folder", path: folder } });
  return options;
}

/** Variable del sistema que también resuelve el secreto. */
export const envVarFor = (name: string) => `HARPOON_${name.toUpperCase().replace(/[^A-Z0-9]/g, "_")}`;
