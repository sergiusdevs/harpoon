/** `disabled`: la línea empezaba por `//` (fila conservada pero no enviada). */
export type BulkRow = { k: string; v: string; d: string; disabled?: boolean };
export function parseBulkEntries(text: string): BulkRow[] {
  const seen = new Set<string>();
  return text.split(/\r?\n/).flatMap((raw, index) => {
    if (!raw.trim()) return [];
    const disabled = raw.trimStart().startsWith("//");
    const line = disabled ? raw.trimStart().slice(2).trimStart() : raw;
    if (!line.trim()) return [];
    let k: string, v: string, d = "";
    if (line.includes("\t")) {
      const columns = line.split("\t");
      if (columns.length > 3) throw new Error(`Línea ${index + 1}: usa como máximo nombre, valor y descripción.`);
      [k, v, d = ""] = columns;
    } else {
      const match = line.match(/^([^:=]+?)\s*[:=]\s*(.*)$/);
      if (!match) throw new Error(`Línea ${index + 1}: usa Nombre: valor o columnas separadas por tabuladores.`);
      [, k, v] = match;
    }
    k = k.trim();
    if (!k) throw new Error(`Línea ${index + 1}: falta el nombre.`);
    if (seen.has(k)) throw new Error(`Línea ${index + 1}: el nombre ${k} está repetido.`);
    seen.add(k); return [{ k, v, d, ...(disabled ? { disabled: true } : {}) }];
  });
}

/** Texto del editor en bloque: `Nombre: valor` por línea y `//` delante de las desactivadas. */
export function serializeBulkEntries(entries: Record<string, string>, isDisabled: (key: string) => boolean = () => false): string {
  return Object.entries(entries).map(([k, v]) => `${isDisabled(k) ? "//" : ""}${k}: ${v}`).join("\n");
}
