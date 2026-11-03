// Convertir en secreto un valor que el detector encontró en una solicitud.
// `location` es la que devuelve harpoon_core::scan ("header Authorization",
// "auth.token", "parámetro api_key", "url (usuario:contraseña)"…).

import type { Request } from "./types";

const SCHEME = /^(\s*(?:Bearer|Basic|Token|Digest|ApiKey)\s+)/i;

/**
 * Sustituye la parte escrita a mano de `value` por `{{name}}`, respetando el
 * esquema (`Bearer `) y las `{{variables}}` que ya hubiera.
 * Devuelve el valor nuevo y el secreto extraído.
 */
export function extractLiteral(value: string, name: string): { next: string; secret: string } | null {
  const scheme = SCHEME.exec(value)?.[1] ?? "";
  const rest = value.slice(scheme.length);
  // Trozos literales entre variables; el secreto es el trozo más largo.
  const pieces = rest.split(/(\{\{[^}]*\}\})/);
  let best = -1;
  pieces.forEach((piece, i) => {
    if (!piece.startsWith("{{") && piece.trim().length > (best < 0 ? 0 : pieces[best].trim().length)) best = i;
  });
  if (best < 0) return null;
  const raw = pieces[best];
  const secret = raw.trim();
  if (!secret) return null;
  pieces[best] = raw.replace(secret, `{{${name}}}`);
  return { next: scheme + pieces.join(""), secret };
}

/** Aplica la conversión sobre una copia de la solicitud. */
export function applySecret(request: Request, location: string, name: string): { request: Request; value: string } | null {
  const req = structuredClone(request);
  const inMap = (map: Record<string, string>, key: string) => {
    const real = Object.keys(map).find((k) => k === key) ?? Object.keys(map).find((k) => k.toLowerCase() === key.toLowerCase());
    if (real === undefined) return null;
    const r = extractLiteral(map[real], name);
    if (!r) return null;
    map[real] = r.next;
    return r.secret;
  };
  let value: string | null = null;
  const after = (prefix: string) => location.startsWith(prefix) ? location.slice(prefix.length) : null;
  const header = after("header "), param = after("parámetro "), field = after("campo del formulario ");
  if (header !== null) value = inMap(req.headers, header);
  else if (param !== null) value = inMap(req.query, param);
  else if (field !== null && req.body.type === "form") value = inMap((req.body as { fields: Record<string, string> }).fields, field);
  else if (location.startsWith("auth.")) {
    const prop = location.slice(5);
    const auth = req.auth as unknown as Record<string, string>;
    if (typeof auth[prop] === "string") {
      const r = extractLiteral(auth[prop], name);
      if (r) { auth[prop] = r.next; value = r.secret; }
    }
  } else if (location.startsWith("url")) {
    const m = /^([a-z][a-z0-9+.-]*:\/\/[^/?#:@]*:)([^/?#@]+)(@)/i.exec(req.url);
    if (m) { value = decodeURIComponent(m[2]); req.url = m[1] + `{{${name}}}` + m[3] + req.url.slice(m[0].length); }
  }
  return value === null ? null : { request: req, value };
}

// ---------- ayudas para la interfaz (mismo criterio que harpoon_core::scan) ----------

const SENSITIVE_EXACT = new Set(["authorization", "proxyauthorization", "cookie", "auth", "pwd", "pass"]);
const SENSITIVE_PARTS = ["password", "passwd", "secret", "token", "apikey", "accesskey", "privatekey", "credential", "signature", "sessionid"];

/** ¿El nombre (cabecera, parámetro, campo, variable) suele llevar un secreto? */
export function isSensitiveName(name: string): boolean {
  const n = name.toLowerCase().replace(/[^a-z0-9]/g, "");
  if (!n) return false;
  return SENSITIVE_EXACT.has(n) || SENSITIVE_PARTS.some((part) => n.includes(part));
}

/** ¿El valor lleva algo escrito tal cual (no solo `{{variables}}` y un esquema)? */
export function isLiteral(value: string): boolean {
  const rest = value.trim().replace(/^(?:Bearer|Basic|Token)\s+/i, "").replace(/\{\{[^}]*\}\}/g, "").trim();
  return rest.length >= 6;
}

/** Nombre de variable propuesto para un secreto según dónde está. */
export function suggestSecretName(location: string): string {
  const auth = /^auth\.(.+)$/.exec(location)?.[1];
  if (auth) return auth === "value" ? "api_key" : auth;
  const name = location.replace(/^(header|parámetro|campo del formulario|variable)\s+/, "");
  const lower = name.toLowerCase().replace(/[^a-z0-9]/g, "");
  if (lower === "authorization" || lower === "proxyauthorization") return "token";
  const slugged = name.replace(/([a-z0-9])([A-Z])/g, "$1_$2").toLowerCase().replace(/[^a-z0-9]+/g, "_").replace(/^_+|_+$/g, "");
  return slugged || "secret";
}

/** Nombre válido para `{{variable}}`. */
export function isValidVarName(name: string): boolean {
  return /^[A-Za-z_][A-Za-z0-9_.-]*$/.test(name);
}

/** Carpeta de una solicitud ("users/list" → "users"; raíz → ""). */
export function folderOf(key: string | null | undefined): string {
  if (!key) return "";
  const i = key.lastIndexOf("/");
  return i < 0 ? "" : key.slice(0, i);
}

/** Los hallazgos de estas ubicaciones se pueden convertir automáticamente. */
export function isConvertibleLocation(location: string): boolean {
  return /^(header |parámetro |campo del formulario |auth\.|url \(usuario)/.test(location);
}
