// Markdown mínimo y seguro para la documentación de las solicitudes.
// Soporta: títulos, párrafos, negrita, cursiva, tachado, código en línea,
// bloques de código, enlaces (solo http(s) y mailto), listas, listas de tareas,
// citas, separadores y tablas simples. Todo el texto se escapa: nunca se
// interpreta HTML escrito por el usuario.

const escapeHtml = (text: string) =>
  text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;").replace(/'/g, "&#39;");

/** Solo se permiten enlaces http(s) y mailto. */
export function safeUrl(url: string): string | null {
  const value = url.trim();
  return /^(https?:\/\/|mailto:)/i.test(value) && !/[\s<>"']/.test(value) ? value : null;
}

const HOLE = "\u0000";

export function renderInline(source: string): string {
  const slots: string[] = [];
  const hold = (html: string) => `${HOLE}${slots.push(html) - 1}${HOLE}`;
  let s = source.replace(/\u0000/g, "");
  s = s.replace(/\\([\\`*_{}[\]()#+\-.!~|>])/g, (_, c: string) => hold(escapeHtml(c)));
  s = s.replace(/(`+)([\s\S]*?[^`])\1(?!`)/g, (_, _ticks: string, code: string) => hold(`<code>${escapeHtml(code.replace(/^ (.*) $/, "$1"))}</code>`));
  s = s.replace(/\[([^\]]*)\]\(\s*([^)\s]*)(?:\s+"[^"]*")?\s*\)/g, (_, label: string, url: string) => {
    const href = safeUrl(url);
    const text = renderInline(label || url);
    return hold(href ? `<a href="${escapeHtml(href)}" target="_blank" rel="noopener noreferrer">${text}</a>` : text);
  });
  s = s.replace(/<((?:https?:\/\/|mailto:)[^\s<>]+)>/gi, (_, url: string) => hold(`<a href="${escapeHtml(url)}" target="_blank" rel="noopener noreferrer">${escapeHtml(url)}</a>`));
  s = escapeHtml(s);
  s = s.replace(/\*\*(?=\S)([\s\S]*?\S)\*\*/g, "<strong>$1</strong>");
  s = s.replace(/(^|[^\w])__(?=\S)([\s\S]*?\S)__(?!\w)/g, "$1<strong>$2</strong>");
  s = s.replace(/~~(?=\S)([\s\S]*?\S)~~/g, "<del>$1</del>");
  s = s.replace(/\*(?=[^\s*])([^*]*?[^\s*]|[^\s*])\*/g, "<em>$1</em>");
  s = s.replace(/(^|[^\w])_(?=[^\s_])([^_]*?[^\s_]|[^\s_])_(?!\w)/g, "$1<em>$2</em>");
  s = s.replace(/\n/g, "<br>");
  const restore = (text: string): string => text.replace(/\u0000(\d+)\u0000/g, (_, n: string) => restore(slots[Number(n)]));
  return restore(s);
}

const FENCE = /^\s{0,3}(```+|~~~+)\s*([\w+-]*)\s*$/;
const HEADING = /^\s{0,3}(#{1,6})\s+(.*?)\s*#*\s*$/;
const RULE = /^\s{0,3}([-*_])(\s*\1){2,}\s*$/;
const LIST = /^(\s*)([-*+]|\d{1,9}[.)])\s+(.*)$/;
const QUOTE = /^\s{0,3}>\s?(.*)$/;
const TABLE_SEP = /^\s*\|?\s*:?-+:?\s*(\|\s*:?-+:?\s*)*\|?\s*$/;

function splitRow(line: string): string[] {
  let row = line.trim();
  if (row.startsWith("|")) row = row.slice(1);
  if (row.endsWith("|") && !row.endsWith("\\|")) row = row.slice(0, -1);
  const cells: string[] = [];
  let current = "";
  for (let i = 0; i < row.length; i++) {
    if (row[i] === "\\" && row[i + 1] === "|") { current += "\\|"; i++; }
    else if (row[i] === "|") { cells.push(current.trim()); current = ""; }
    else current += row[i];
  }
  cells.push(current.trim());
  return cells;
}

const isBlockStart = (line: string, next?: string) =>
  FENCE.test(line) || HEADING.test(line) || RULE.test(line) || LIST.test(line) || QUOTE.test(line) || (line.includes("|") && next !== undefined && TABLE_SEP.test(next) && next.includes("-"));

type Item = { indent: number; ordered: boolean; start: number; text: string[]; task: null | boolean };

function renderList(items: Item[]): string {
  // Construye listas anidadas según la sangría de cada elemento.
  let html = "";
  const stack: { indent: number; tag: string }[] = [];
  for (const item of items) {
    while (stack.length && item.indent < stack[stack.length - 1].indent) html += `</li></${stack.pop()!.tag}>`;
    const top = stack[stack.length - 1];
    const tag = item.ordered ? "ol" : "ul";
    if (!top || item.indent > top.indent) {
      const attrs = item.ordered && item.start !== 1 ? ` start="${item.start}"` : item.task !== null ? ` class="task-list"` : "";
      html += `<${tag}${attrs}>`;
      stack.push({ indent: item.indent, tag });
    } else {
      html += "</li>";
      if (top.tag !== tag) { html += `</${stack.pop()!.tag}><${tag}${item.task !== null ? ' class="task-list"' : ""}>`; stack.push({ indent: item.indent, tag }); }
    }
    const body = renderInline(item.text.join("\n"));
    html += item.task === null ? `<li>${body}` : `<li class="task"><input type="checkbox" disabled${item.task ? " checked" : ""}> ${body}`;
  }
  while (stack.length) html += `</li></${stack.pop()!.tag}>`;
  return html;
}

/** Convierte Markdown en HTML seguro. */
export function renderMarkdown(source: string): string {
  const lines = source.replace(/\r\n?/g, "\n").split("\n");
  const out: string[] = [];
  let i = 0;
  while (i < lines.length) {
    const line = lines[i];
    if (!line.trim()) { i++; continue; }
    const fence = line.match(FENCE);
    if (fence) {
      const code: string[] = [];
      i++;
      while (i < lines.length && !(lines[i].trim().startsWith(fence[1][0].repeat(3)) && lines[i].trim().replace(/[`~]/g, "") === "")) code.push(lines[i++]);
      i++;
      const lang = fence[2] ? ` data-lang="${escapeHtml(fence[2])}"` : "";
      out.push(`<pre${lang}><code>${escapeHtml(code.join("\n"))}</code></pre>`);
      continue;
    }
    const heading = line.match(HEADING);
    if (heading) { const n = heading[1].length; out.push(`<h${n}>${renderInline(heading[2])}</h${n}>`); i++; continue; }
    if (RULE.test(line)) { out.push("<hr>"); i++; continue; }
    if (line.includes("|") && i + 1 < lines.length && TABLE_SEP.test(lines[i + 1]) && lines[i + 1].includes("-")) {
      const head = splitRow(line);
      const align = splitRow(lines[i + 1]).map((cell) => (cell.startsWith(":") && cell.endsWith(":") ? "center" : cell.endsWith(":") ? "right" : ""));
      const cell = (tag: string, text: string, c: number) => `<${tag}${align[c] ? ` style="text-align:${align[c]}"` : ""}>${renderInline(text)}</${tag}>`;
      let html = `<table><thead><tr>${head.map((text, c) => cell("th", text, c)).join("")}</tr></thead><tbody>`;
      i += 2;
      while (i < lines.length && lines[i].trim() && lines[i].includes("|")) {
        const row = splitRow(lines[i++]);
        html += `<tr>${head.map((_, c) => cell("td", row[c] ?? "", c)).join("")}</tr>`;
      }
      out.push(html + "</tbody></table>");
      continue;
    }
    if (QUOTE.test(line)) {
      const quoted: string[] = [];
      while (i < lines.length && QUOTE.test(lines[i])) quoted.push(lines[i++].match(QUOTE)![1]);
      out.push(`<blockquote>${renderMarkdown(quoted.join("\n"))}</blockquote>`);
      continue;
    }
    if (LIST.test(line)) {
      const items: Item[] = [];
      while (i < lines.length) {
        const m = lines[i].match(LIST);
        if (m) {
          const task = m[3].match(/^\[([ xX])\]\s+(.*)$/);
          items.push({ indent: m[1].replace(/\t/g, "  ").length, ordered: /\d/.test(m[2]), start: parseInt(m[2], 10) || 1, text: [task ? task[2] : m[3]], task: task ? task[1] !== " " : null });
          i++;
        } else if (lines[i].trim() && /^\s+/.test(lines[i]) && items.length) {
          items[items.length - 1].text.push(lines[i++].trim());
        } else break;
      }
      out.push(renderList(items));
      continue;
    }
    const para: string[] = [];
    while (i < lines.length && lines[i].trim() && !(para.length && isBlockStart(lines[i], lines[i + 1]))) para.push(lines[i++].trim());
    out.push(`<p>${renderInline(para.join("\n"))}</p>`);
  }
  return out.join("\n");
}

// ---------------------------------------------------------------- HTML → Markdown

/** Subconjunto de la interfaz de nodos del DOM que usa la conversión. */
interface DomNode {
  nodeType: number;
  nodeName: string;
  textContent: string | null;
  childNodes: ArrayLike<DomNode>;
  getAttribute?: (name: string) => string | null;
  checked?: boolean;
}

const tagOf = (node: DomNode) => node.nodeName.toLowerCase();
const children = (node: DomNode) => Array.from(node.childNodes);
const BLOCKS = new Set(["p", "div", "h1", "h2", "h3", "h4", "h5", "h6", "ul", "ol", "pre", "blockquote", "table", "hr"]);

function escapeText(text: string): string {
  return text
    .replace(/\\/g, "\\\\")
    .replace(/([`*~[\]])/g, "\\$1")
    .replace(/(^|\W)_|_(?=\W|$)/g, (m) => m.replace("_", "\\_"));
}

function wrap(inner: string, mark: string): string {
  const m = inner.match(/^(\s*)([\s\S]*?)(\s*)$/)!;
  return m[2] ? `${m[1]}${mark}${m[2]}${mark}${m[3]}` : inner;
}

function inlineMd(node: DomNode, inTable = false): string {
  if (node.nodeType === 3) {
    const text = escapeText((node.textContent ?? "").replace(/ /g, " ").replace(/\n/g, " "));
    return inTable ? text.replace(/\|/g, "\\|") : text;
  }
  if (node.nodeType !== 1) return "";
  const tag = tagOf(node);
  const inner = () => children(node).map((child) => inlineMd(child, inTable)).join("");
  switch (tag) {
    case "br": return inTable ? " " : "\n";
    case "strong": case "b": return wrap(inner(), "**");
    case "em": case "i": return wrap(inner(), "*");
    case "s": case "del": case "strike": return wrap(inner(), "~~");
    case "code": {
      const code = (node.textContent ?? "").replace(/ /g, " ");
      if (!code) return "";
      const ticks = code.includes("`") ? "``" : "`";
      return ticks + (code.startsWith("`") || code.endsWith("`") ? ` ${code} ` : code) + ticks;
    }
    case "a": {
      const href = safeUrl(node.getAttribute?.("href") ?? "");
      const text = inner();
      return href ? `[${text || href}](${href})` : text;
    }
    case "input": return "";
    default: return BLOCKS.has(tag) || tag === "li" ? "" : inner();
  }
}

/** Texto de un bloque: une los hijos en línea y convierte bloques anidados (p. ej. <div> dentro de <li>). */
function blockText(node: DomNode): string {
  let out = "";
  for (const child of children(node)) {
    const tag = child.nodeType === 1 ? tagOf(child) : "";
    if (tag === "div" || tag === "p") out += (out && !out.endsWith("\n") ? "\n" : "") + blockText(child);
    else if (!BLOCKS.has(tag)) out += inlineMd(child);
  }
  return out.replace(/[ \t]+\n/g, "\n").replace(/\n+$/, "").trim();
}

function guardLine(text: string): string {
  return text
    .split("\n")
    .map((line) => (RULE.test(line) ? "\\" + line.trimStart() : line.replace(/^(\s*)(#{1,6}\s|[-+]\s|>)/, "$1\\$2").replace(/^(\s*\d+)([.)]\s)/, "$1\\$2")))
    .join("\n");
}

function listMd(node: DomNode, depth: number): string {
  const ordered = tagOf(node) === "ol";
  let n = Number(node.getAttribute?.("start")) || 1;
  const lines: string[] = [];
  const pad = "   ".repeat(depth);
  for (const li of children(node)) {
    if (li.nodeType !== 1) continue;
    const tag = tagOf(li);
    if (tag === "ul" || tag === "ol") { lines.push(listMd(li, depth + 1)); continue; }
    if (tag !== "li") continue;
    const box = children(li).find((c) => c.nodeType === 1 && tagOf(c) === "input");
    const checked = box && (box.checked === undefined ? box.getAttribute?.("checked") != null : box.checked);
    const task = box ? `[${checked ? "x" : " "}] ` : "";
    const marker = ordered ? `${n++}. ` : "- ";
    const text = blockText(li).replace(/^\s+/, "");
    lines.push(pad + marker + task + text.replace(/\n/g, "\n" + pad + " ".repeat(marker.length)));
    for (const child of children(li)) {
      if (child.nodeType === 1 && (tagOf(child) === "ul" || tagOf(child) === "ol")) lines.push(listMd(child, depth + 1));
    }
  }
  return lines.join("\n");
}

function tableMd(node: DomNode): string {
  const rows: string[][] = [];
  const walk = (n: DomNode) => {
    for (const child of children(n)) {
      if (child.nodeType !== 1) continue;
      const tag = tagOf(child);
      if (tag === "tr") rows.push(children(child).filter((c) => c.nodeType === 1 && /^t[hd]$/i.test(c.nodeName)).map((cell) => children(cell).map((c) => inlineMd(c, true)).join("").trim()));
      else walk(child);
    }
  };
  walk(node);
  if (!rows.length) return "";
  const width = Math.max(...rows.map((row) => row.length));
  const line = (row: string[]) => `| ${Array.from({ length: width }, (_, c) => row[c] ?? "").join(" | ")} |`;
  return [line(rows[0]), `| ${Array(width).fill("---").join(" | ")} |`, ...rows.slice(1).map(line)].join("\n");
}

/** Convierte el contenido de un editor enriquecido (DOM) en Markdown. */
export function htmlToMarkdown(root: DomNode): string {
  const blocks: string[] = [];
  let inline = "";
  const flush = () => { const text = inline.trim(); if (text) blocks.push(guardLine(text)); inline = ""; };
  for (const node of children(root)) {
    const tag = node.nodeType === 1 ? tagOf(node) : "";
    if (tag === "li") { flush(); const text = listMd({ ...node, nodeName: "UL", childNodes: [node] } as DomNode, 0); if (text.trim()) blocks.push(text); continue; }
    if (!BLOCKS.has(tag)) { inline += inlineMd(node); continue; }
    flush();
    if (/^h[1-6]$/.test(tag)) { const text = blockText(node).replace(/\n/g, " "); if (text) blocks.push(`${"#".repeat(Number(tag[1]))} ${text}`); }
    else if (tag === "p" || tag === "div") {
      // Los navegadores a veces anidan listas o tablas dentro de <p>: se tratan como contenedor.
      const nested = children(node).some((child) => child.nodeType === 1 && BLOCKS.has(tagOf(child)) && !/^(p|div)$/.test(tagOf(child)));
      const text = nested ? htmlToMarkdown(node) : guardLine(blockText(node));
      if (text) blocks.push(text);
    }
    else if (tag === "ul" || tag === "ol") { const text = listMd(node, 0); if (text.trim()) blocks.push(text); }
    else if (tag === "pre") {
      const code = (node.textContent ?? "").replace(/ /g, " ").replace(/\n$/, "");
      const lang = node.getAttribute?.("data-lang") ?? "";
      const fence = code.includes("```") ? "~~~" : "```";
      blocks.push(`${fence}${lang}\n${code}\n${fence}`);
    } else if (tag === "blockquote") { const text = htmlToMarkdown(node); if (text) blocks.push(text.split("\n").map((l) => (l ? "> " + l : ">")).join("\n")); }
    else if (tag === "table") { const text = tableMd(node); if (text) blocks.push(text); }
    else if (tag === "hr") blocks.push("---");
  }
  flush();
  return blocks.join("\n\n");
}
