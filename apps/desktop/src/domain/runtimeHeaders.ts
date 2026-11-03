// Cabeceras que se envían aunque la solicitud no las declare: las que añade
// Harpoon a partir de Auth y del cuerpo, y las del cliente HTTP (reqwest/hyper).
import type { Auth, Body, Request } from "./types";

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

/** Igual que `harpoon_http::USER_AGENT`. */
export const USER_AGENT = "harpoon/0.1.0";

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

export function runtimeHeaders(request: Request, collectionAuth: Auth, resolvedUrl: string): RuntimeHeader[] {
  const disabled = request.disabled_headers ?? [];
  const rows: Omit<RuntimeHeader, "overridden">[] = [];
  const auth = authHeader(request.auth.type === "inherit" ? collectionAuth : request.auth);
  if (auth) rows.push({ ...auth, source: request.auth.type === "inherit" ? "Auth heredada de la colección" : "Pestaña Auth", toggleable: false });
  const type = contentType(request.body);
  if (type) rows.push({ name: "Content-Type", value: type, source: "Tipo de cuerpo", toggleable: false });
  rows.push(
    { name: "User-Agent", value: USER_AGENT, source: "Harpoon", toggleable: true },
    { name: "Accept", value: "*/*", source: "Cliente HTTP", toggleable: false },
    { name: "Accept-Encoding", value: "gzip", source: "Cliente HTTP", toggleable: false },
    { name: "Host", value: hostOf(resolvedUrl || request.url), source: "Cliente HTTP", toggleable: false },
  );
  return rows.map((row) => ({ ...row, overridden: has(request.headers, disabled, row.name) }));
}

/** Nombres desactivados que corresponden a cabeceras del runtime (no a filas de la tabla). */
export function disabledRuntime(request: Request): string[] {
  return (request.disabled_headers ?? []).filter((name) =>
    TOGGLEABLE.some((t) => t.toLowerCase() === name.toLowerCase())
    && !Object.keys(request.headers).some((key) => key.toLowerCase() === name.toLowerCase()));
}
