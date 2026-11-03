export interface DiffLine { kind: "same" | "added" | "removed"; text: string }

/** Diferencia por líneas, limitada para evitar bloquear la interfaz con respuestas enormes. */
export function diffLines(before: string, after: string): DiffLine[] | null {
  const a = before.split("\n"), b = after.split("\n");
  if (a.length * b.length > 1_000_000 || before.length + after.length > 2_000_000) return null;
  const width = b.length + 1;
  const table = new Uint32Array((a.length + 1) * width);
  for (let i = a.length - 1; i >= 0; i--) {
    for (let j = b.length - 1; j >= 0; j--) {
      table[i * width + j] = a[i] === b[j] ? 1 + table[(i + 1) * width + j + 1] : Math.max(table[(i + 1) * width + j], table[i * width + j + 1]);
    }
  }
  const result: DiffLine[] = [];
  let i = 0, j = 0;
  while (i < a.length || j < b.length) {
    if (i < a.length && j < b.length && a[i] === b[j]) { result.push({ kind: "same", text: a[i] }); i++; j++; }
    else if (j < b.length && (i === a.length || table[i * width + j + 1] > table[(i + 1) * width + j])) result.push({ kind: "added", text: b[j++] });
    else result.push({ kind: "removed", text: a[i++] });
  }
  return result;
}

export function formatComparisonBody(body: string, contentType: string): string {
  if (/json/i.test(contentType)) {
    try { return JSON.stringify(JSON.parse(body), null, 2); } catch { /* conservar texto original */ }
  }
  return body;
}

export function formatComparisonHeaders(headers: Record<string, string>): string {
  return Object.entries(headers).map(([name, value]) => [name.toLowerCase(), value] as const)
    .sort(([a], [b]) => a.localeCompare(b)).map(([name, value]) => `${name}: ${value}`).join("\n");
}
