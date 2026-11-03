export interface VariableSuggestion { name: string; source: string }
export interface VariableLayer { source: string; names: string[] }

/** Combina capas en orden de prioridad, sin exponer sus valores. */
export function variableSuggestions(layers: VariableLayer[]): VariableSuggestion[] {
  const variables = new Map<string, VariableSuggestion>();
  for (const layer of layers) {
    for (const name of layer.names) variables.set(name, { name, source: layer.source });
  }
  for (const name of ["$uuid", "$guid", "$timestamp", "$isoTimestamp", "$randomInt", "$randomHex"]) {
    variables.set(name, { name, source: "Dinámica" });
  }
  return [...variables.values()].sort((a, b) => a.name.localeCompare(b.name));
}

/** Fragmento entre llaves que contiene el cursor, si lo hay. */
export function variableAtCursor(value: string, cursor: number): { start: number; end: number; query: string } | null {
  const before = value.slice(0, cursor);
  const match = /\{\{([^{}]*)$/.exec(before);
  if (!match) return null;
  const after = value.slice(cursor);
  const closing = /^[^{}]*\}\}/.exec(after);
  return { start: match.index, end: closing ? cursor + closing[0].length : cursor, query: match[1].trim().toLowerCase() };
}

export function isKnownVariable(text: string, suggestions: VariableSuggestion[]): boolean {
  const name = text.trim();
  if (name.startsWith("$")) return suggestions.some((item) => item.name === name.split(/\s+/)[0] && item.source === "Dinámica");
  return suggestions.some((item) => item.name === name);
}
