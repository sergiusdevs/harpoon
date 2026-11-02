import { emptyRequest, METHODS, type Method, type Request } from "./types";

/** Analiza argumentos de shell sin ejecutarlos ni expandir variables. */
function tokenize(input: string): string[] {
  const words: string[] = [];
  let word = "", quote = "", started = false;
  for (let i = 0; i < input.length; i++) {
    const c = input[i];
    if (c === "\\" && quote !== "'") {
      if (++i >= input.length) throw new Error("Barra invertida incompleta");
      if (input[i] !== "\n") { word += input[i]; started = true; }
    } else if (quote) {
      if (c === quote) quote = "";
      else { if (quote === '"' && (c === "$" || c === "`")) throw new Error("La expansión de shell no está admitida"); word += c; }
    } else if (c === "'" || c === '"') { quote = c; started = true; }
    else if (/\s/.test(c)) { if (started) { words.push(word); word = ""; started = false; } }
    else { if ("|;&<>$`".includes(c)) throw new Error("Los operadores y expansiones de shell no están admitidos"); word += c; started = true; }
  }
  if (quote) throw new Error("Hay una comilla sin cerrar");
  if (started) words.push(word);
  return words;
}

export function parseCurl(input: string): Request {
  const args = tokenize(input.trim());
  if (args.shift() !== "curl") throw new Error("El comando debe comenzar por curl");
  const req = emptyRequest("Solicitud importada");
  req.auth = { type: "none" };
  let method: string | undefined, url = "", data: string[] = [], get = false;
  const arg = (index: number, option: string) => { if (index >= args.length) throw new Error(`Falta el valor de ${option}`); return args[index]; };
  for (let i = 0; i < args.length; i++) {
    const item = args[i];
    if (item === "-X" || item === "--request") method = arg(++i, item).toUpperCase();
    else if (item === "-H" || item === "--header") {
      const header = arg(++i, item), colon = header.indexOf(":");
      if (colon <= 0) throw new Error("Cabecera inválida: usa Nombre: valor");
      const name = header.slice(0, colon).trim();
      if (Object.keys(req.headers).some((key) => key.toLowerCase() === name.toLowerCase())) throw new Error(`Cabecera repetida: ${name}`);
      req.headers[name] = header.slice(colon + 1).trim();
    } else if (["-d", "--data", "--data-raw", "--data-binary"].includes(item)) {
      const value = arg(++i, item);
      if (item !== "--data-raw" && value.startsWith("@")) throw new Error("La lectura de archivos no está admitida");
      data.push(value);
    } else if (item === "-u" || item === "--user") {
      const user = arg(++i, item), colon = user.indexOf(":");
      if (colon < 0) throw new Error("Indica usuario y contraseña separados por dos puntos");
      req.auth = { type: "basic", username: user.slice(0, colon), password: user.slice(colon + 1) };
    } else if (item === "-G" || item === "--get") get = true;
    else if (item === "-I" || item === "--head") method = "HEAD";
    else if (item === "--url") { if (url) throw new Error("Solo se admite una URL"); url = arg(++i, item); }
    else if (item === "--compressed") { /* la compresión la gestiona el cliente */ }
    else if (item.startsWith("-")) throw new Error(`Opción no admitida: ${item}`);
    else { if (url) throw new Error("Solo se admite una URL"); url = item; }
  }
  if (!url) throw new Error("Falta la URL");
  if (get && data.length) {
    const hash = url.indexOf("#");
    const fragment = hash < 0 ? "" : url.slice(hash);
    const base = hash < 0 ? url : url.slice(0, hash);
    url = base + (base.includes("?") ? "&" : "?") + data.join("&") + fragment;
  }
  req.url = url;
  const selected = method ?? (data.length && !get ? "POST" : "GET");
  if (!(METHODS as readonly string[]).includes(selected)) throw new Error(`Método no admitido: ${selected}`);
  req.method = selected as Method;
  if (data.length && !get) {
    const body = data.join("&");
    const contentType = Object.entries(req.headers).find(([key]) => key.toLowerCase() === "content-type")?.[1] ?? "";
    if (!contentType) req.headers["Content-Type"] = "application/x-www-form-urlencoded";
    req.body = /\bjson\b/i.test(contentType) ? { type: "json", json: body } : { type: "text", text: body };
  }
  return req;
}
