// Sincroniza la barra de URL con la pestaña Params.
//
// En disco la solicitud guarda `url` (sin query) y `query` por separado; la
// barra muestra la URL completa con los parámetros activos. Al escribir o
// pegar `?a=1&b=2` en la barra, los parámetros pasan a Params; al editar
// Params, la barra se recompone. Los valores se dejan tal cual (sin
// decodificar) para no romper `{{variables}}`.

export interface QueryState {
  url: string;
  query: Record<string, string>;
  disabled_query: string[];
}

/** Posición del primer `ch` fuera de `{{…}}` (o -1). */
function indexOutsideVars(text: string, ch: string, from = 0): number {
  let depth = 0;
  for (let i = from; i < text.length; i++) {
    if (text.startsWith("{{", i)) { depth++; i++; continue; }
    if (depth && text.startsWith("}}", i)) { depth--; i++; continue; }
    if (!depth && text[i] === ch) return i;
  }
  return -1;
}

function splitPairs(query: string): [string, string][] {
  const out: [string, string][] = [];
  let rest = query;
  while (rest.length) {
    const amp = indexOutsideVars(rest, "&");
    const part = amp < 0 ? rest : rest.slice(0, amp);
    rest = amp < 0 ? "" : rest.slice(amp + 1);
    if (!part) continue;
    const eq = indexOutsideVars(part, "=");
    out.push(eq < 0 ? [part, ""] : [part.slice(0, eq), part.slice(eq + 1)]);
  }
  return out;
}

/** `base?a=1#h` → { base: "base#h", pairs: [["a","1"]] }. */
export function splitUrl(text: string): { base: string; pairs: [string, string][]; hasQuery: boolean } {
  const q = indexOutsideVars(text, "?");
  if (q < 0) return { base: text, pairs: [], hasQuery: false };
  const hash = indexOutsideVars(text, "#", q);
  const query = hash < 0 ? text.slice(q + 1) : text.slice(q + 1, hash);
  const base = text.slice(0, q) + (hash < 0 ? "" : text.slice(hash));
  return { base, pairs: splitPairs(query), hasQuery: true };
}

/** URL completa para la barra: base + parámetros activos (antes del `#`). */
export function composeUrl(url: string, query: Record<string, string>, disabled: string[] = []): string {
  const pairs = Object.entries(query).filter(([k]) => !disabled.includes(k)).map(([k, v]) => (v === "" ? k : `${k}=${v}`));
  if (!pairs.length) return url;
  const hash = indexOutsideVars(url, "#");
  const base = hash < 0 ? url : url.slice(0, hash);
  const fragment = hash < 0 ? "" : url.slice(hash);
  const joiner = indexOutsideVars(base, "?") < 0 ? "?" : "&";
  return base + joiner + pairs.join("&") + fragment;
}

/**
 * Lo que queda en la solicitud tras escribir `text` en la barra. Los
 * parámetros escritos sustituyen a los activos (mismo orden que en la URL);
 * los desactivados se conservan aunque no aparezcan en la barra.
 */
export function applyUrlInput(text: string, prev: QueryState): QueryState {
  const { base, pairs } = splitUrl(text);
  const query: Record<string, string> = {};
  for (const [k, v] of pairs) query[k] = v;
  const disabled_query = prev.disabled_query.filter((k) => !(k in query));
  for (const k of disabled_query) if (k in prev.query) query[k] = prev.query[k];
  return { url: base, query, disabled_query };
}
