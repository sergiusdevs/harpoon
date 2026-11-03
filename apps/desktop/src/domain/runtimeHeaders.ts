// Cabeceras que se envían aunque la solicitud no las declare: las que añade
// Harpoon a partir de Auth y del cuerpo, y las del cliente HTTP (reqwest/hyper).
import type { Auth, Body, Request } from "./types";
import type { InheritedHeader } from "./folderInheritance";

export interface RuntimeHeader {
  name: string;
  value: string;
  /** De dónde sale, para el tooltip. */
  source: string;
  /** Se puede desactivar desde la solicitud (`disabled_headers`). */
  toggleable: boolean;
  /** La solicitud define una cabecera con el mismo nombre que la sustituye. */
  overridden: boolean;
}

declare const __APP_VERSION__: string | undefined;

/** Igual que `harpoon_http::USER_AGENT` (la versión la inyecta Vite desde package.json). */
export const USER_AGENT = `harpoon/${typeof __APP_VERSION__ === "string" ? __APP_VERSION__ : "dev"}`;

/** Cabeceras del runtime que la solicitud puede desactivar. */
export const TOGGLEABLE = ["User-Agent"];

const has = (headers: Record<string, string>, disabled: string[], name: string) =>
  Object.keys(headers).some((key) => key.toLowerCase() === name.toLowerCase() && !disabled.some((d) => d.toLowerCase() === key.toLowerCase()));

function authHeader(auth: Auth): { name: string; value: string } | null {
  switch (auth.type) {
    case "basic": return { name: "Authorization", value: "Basic …" };
    case "bearer": return { name: "Authorization", value: "Bearer …" };
    case "digest": return { name: "Authorization", value: "Digest … (tras el reto 401)" };
    case "oauth2": return { name: "Authorization", value: `${auth.header_prefix || "Bearer"} …` };
    case "api_key": return auth.in === "header" && auth.key ? { name: auth.key, value: "…" } : null;
    default: return null;
  }
}

function contentType(body: Body): string | null {
  switch (body.type) {
    case "json": case "graphql": return "application/json";
    case "xml": return "application/xml";
    case "binary": return "application/octet-stream";
    case "form": return "application/x-www-form-urlencoded";
    case "multipart": return "multipart/form-data; boundary=…";
    default: return null;
  }
}

function hostOf(url: string): string {
  try {
    return new URL(url).host;
  } catch {
    return "según la URL";
  }
}

/**
 * `inherited`: auth que se usa con "heredar" y de dónde sale (`inheritedAuth`).
 * `folderHeaders`: cabeceras de las carpetas de la solicitud (`inheritedHeaders`).
 */
export function runtimeHeaders(request: Request, inherited: { auth: Auth; source: string }, resolvedUrl: string, folderHeaders: InheritedHeader[] = []): RuntimeHeader[] {
  const disabled = request.disabled_headers ?? [];
  const rows: Omit<RuntimeHeader, "overridden">[] = folderHeaders.map((h) => ({ ...h, toggleable: true }));
  const auth = authHeader(request.auth.type === "inherit" ? inherited.auth : request.auth);
  if (auth) rows.push({ ...auth, source: request.auth.type === "inherit" ? `Auth heredada de ${inherited.source}` : "Pestaña Auth", toggleable: false });
  const type = contentType(request.body);
  if (type) rows.push({ name: "Content-Type", value: type, source: "Tipo de cuerpo", toggleable: false });
  rows.push(
    { name: "User-Agent", value: USER_AGENT, source: "Harpoon", toggleable: true },
    { name: "Accept", value: "*/*", source: "Cliente HTTP", toggleable: false },
    { name: "Accept-Encoding", value: "gzip, deflate, br", source: "Cliente HTTP", toggleable: false },
    { name: "Host", value: hostOf(resolvedUrl || request.url), source: "Cliente HTTP", toggleable: false },
  );
  return rows.map((row) => ({ ...row, overridden: has(request.headers, disabled, row.name) }));
}

/** Nombres desactivados que corresponden a cabeceras heredadas o del runtime (no a filas de la tabla). */
export function disabledRuntime(request: Request, folderHeaders: InheritedHeader[] = []): string[] {
  const toggleable = [...TOGGLEABLE, ...folderHeaders.map((h) => h.name)];
  return (request.disabled_headers ?? []).filter((name) =>
    toggleable.some((t) => t.toLowerCase() === name.toLowerCase())
    && !Object.keys(request.headers).some((key) => key.toLowerCase() === name.toLowerCase()));
}
