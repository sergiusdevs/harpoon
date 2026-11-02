/** Formatea el HTML para mostrarlo sin modificar la respuesta original. */
export function formatHtml(source: string): string {
  // Los elementos de texto literal se conservan enteros, incluidos sus espacios.
  const tokenPattern = /<(script|style|pre|textarea)\b(?:"[^"]*"|'[^']*'|[^'">])*>[\s\S]*?<\/\1\s*>|<!--[\s\S]*?-->|<![^>]*>|<\/?[a-zA-Z][\w:.-]*(?:"[^"]*"|'[^']*'|[^'">])*>|[^<]+|</gi;
  const tokens = source.match(tokenPattern);
  if (!tokens || tokens.join("") !== source) return source;

  const voidElements = new Set(["area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source", "track", "wbr"]);
  const lines: string[] = [];
  let depth = 0;
  let lastWasText = false;
  for (const token of tokens) {
    if (!token.trim()) continue;
    const closing = /^<\/([\w:.-]+)/.exec(token);
    const opening = /^<([\w:.-]+)/.exec(token);
    const literal = /^<(script|style|pre|textarea)\b/i.test(token) && /<\/(?:script|style|pre|textarea)\s*>$/i.test(token);
    if (closing) {
      depth = Math.max(0, depth - 1);
      if (lastWasText && lines.length) lines[lines.length - 1] += token;
      else lines.push(`${"  ".repeat(depth)}${token}`);
    } else if (opening || token.startsWith("<!")) {
      lines.push(`${"  ".repeat(depth)}${token}`);
      if (opening && !literal && !token.endsWith("/>") && !voidElements.has(opening[1].toLowerCase())) depth++;
    } else if (lines.length && !lastWasText && !token.includes("\n")) {
      lines[lines.length - 1] += token;
    } else {
      lines.push(`${"  ".repeat(depth)}${token}`);
    }
    lastWasText = !opening && !closing && !token.startsWith("<!");
  }
  return lines.join("\n");
}
