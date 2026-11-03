// Espejo de los tipos de `harpoon-core` tal y como los serializa serde.

export type Method = "GET" | "POST" | "PUT" | "PATCH" | "DELETE" | "HEAD" | "OPTIONS";
export const METHODS: Method[] = ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];

export type RequestType = "http" | "graphql" | "websocket" | "sse";
export const REQUEST_TYPES: { value: RequestType; label: string }[] = [
  { value: "http", label: "HTTP / REST" },
  { value: "graphql", label: "GraphQL" },
  { value: "websocket", label: "WebSocket" },
  { value: "sse", label: "SSE" },
];

/** Reconoce el tipo a partir de los datos guardados en la colección. */
export function requestType(request: Request): RequestType {
  if (request.type) return request.type;
  if (/^wss?:\/\//i.test(request.url)) return "websocket";
  if (Object.entries(enabledHeaders(request)).some(([key, value]) =>
    key.toLowerCase() === "accept" && value.split(",").some((part) => part.split(";")[0].trim().toLowerCase() === "text/event-stream")
  )) return "sse";
  return request.body?.type === "graphql" ? "graphql" : "http";
}

export function newTypedRequest(name: string, type: RequestType, method: Method = "GET"): Request {
  const request = emptyRequest(name, type === "graphql" ? "POST" : type === "http" ? method : "GET");
  request.type = type;
  if (type === "graphql") {
    request.url = "{{base_url}}/graphql";
    request.body = { type: "graphql", query: "", variables: "{}", operation_name: "" };
  } else if (type === "websocket") {
    request.url = "wss://localhost/socket";
  } else if (type === "sse") {
    request.url = "{{base_url}}/events";
    request.headers.Accept = "text/event-stream";
  }
  return request;
}

export type Auth =
  | { type: "inherit" }
  | { type: "none" }
  | { type: "basic"; username: string; password: string }
  | { type: "bearer"; token: string }
  | { type: "api_key"; key: string; value: string; in: "header" | "query" }
  | { type: "digest"; username: string; password: string }
  | {
      type: "oauth2";
      grant: "client_credentials" | "password" | "refresh_token";
      token_url: string;
      client_id: string;
      client_secret: string;
      scope: string;
      username: string;
      password: string;
      refresh_token: string;
      client_auth: "basic" | "body";
      extra: Record<string, string>;
      header_prefix: string;
    };

export function emptyOAuth2(): Extract<Auth, { type: "oauth2" }> {
  return {
    type: "oauth2",
    grant: "client_credentials",
    token_url: "",
    client_id: "",
    client_secret: "",
    scope: "",
    username: "",
    password: "",
    refresh_token: "",
    client_auth: "basic",
    extra: {},
    header_prefix: "Bearer",
  };
}

export function normalizeAuth(a: Auth | undefined): Auth {
  if (!a) return { type: "inherit" };
  if (a.type === "api_key") return { ...a, in: a.in ?? "header" };
  if (a.type === "oauth2") return { ...emptyOAuth2(), ...a, extra: a.extra ?? {} };
  return a;
}

export type MultipartPart =
  | { type: "text"; name: string; value: string }
  | { type: "file"; name: string; path: string; content_type?: string };

export type Body =
  | { type: "none" }
  | { type: "json"; json: string }
  | { type: "graphql"; schema?: string; query: string; variables: string; operation_name: string }
  | { type: "text"; text: string }
  | { type: "xml"; xml: string }
  | { type: "binary"; path: string }
  | { type: "form"; fields: Record<string, string> }
  | { type: "multipart"; parts: MultipartPart[] };

export interface Expect {
  status?: number;
  headers: Record<string, string>;
  body_contains: string[];
  max_time_ms?: number;
}

export interface Scripts {
  pre: string;
  post: string;
  test: string;
}

export interface Request {
  type?: RequestType;
  name: string;
  method: Method;
  url: string;
  headers: Record<string, string>;
  header_descriptions?: Record<string, string>;
  query_descriptions?: Record<string, string>;
  query: Record<string, string>;
  disabled_query: string[];
  disabled_headers?: string[];
  auth: Auth;
  body: Body;
  scripts: Scripts;
  expect: Expect;
  docs: string;
}

export interface CollectionMeta {
  name: string;
  format: number;
  default_environment: string | null;
  vars: Record<string, string>;
  /** Variables de colección secretas: el valor vive en `.harpoon/secrets.toml` (fuera de Git). */
  secrets: string[];
  auth: Auth;
  settings: Settings;
  scripts: Scripts;
}

export interface Settings {
  timeout_ms: number;
  follow_redirects: boolean;
  proxy?: string;
  insecure: boolean;
  ca_cert?: string;
  client_cert?: string;
  client_key?: string;
}

export interface Environment {
  descriptions?: Record<string, string>;
  vars: Record<string, string>;
  secrets: string[];
}

export interface Collection {
  root: string;
  meta: CollectionMeta;
  environments: Record<string, Environment>;
  requests: Record<string, Request>;
  folder_configs: Record<string, FolderConfig>;
}

export interface FolderConfig {
  vars: Record<string, string>;
  /** Variables de carpeta secretas (valor en `.harpoon/secrets.toml`). */
  secrets: string[];
  /** Cabeceras que heredan las solicitudes de la carpeta y sus subcarpetas. */
  headers: Record<string, string>;
  /** `inherit` usa la auth de la carpeta superior o de la colección. */
  auth: Auth;
  scripts: Scripts;
}

export function emptyFolderConfig(): FolderConfig {
  return { vars: {}, secrets: [], headers: {}, auth: { type: "inherit" }, scripts: { pre: "", post: "", test: "" } };
}

/** Posible secreto en un archivo versionado (lo detecta `harpoon_core::scan`). */
export interface SecretFinding {
  file: string;
  location: string;
  kind: string;
  /** Valor recortado, nunca entero. */
  preview: string;
  /** Nombre de variable sugerido. */
  suggestion: string;
}

/** Secreto declarado sin valor en esta máquina. */
export interface MissingSecret {
  scope: "collection" | "folder" | "environment";
  /** Carpeta o entorno; vacío para la colección. */
  owner: string;
  name: string;
  /** Variable del sistema que también lo resuelve (`HARPOON_TOKEN`). */
  env_var: string;
}

export type SecretScope = { kind: "collection" } | { kind: "folder"; path: string } | { kind: "environment"; name: string };

export interface Outcome {
  name: string;
  passed: boolean;
  detail?: string;
}

export interface Exchange {
  sent: {
    method: Method;
    url: string;
    headers: Record<string, string>;
    query: Record<string, string>;
    body?: string;
    multipart?: MultipartPart[];
    binary_path?: string;
  };
  response: {
    status: number;
    headers: Record<string, string>;
    body: string;
    is_json: boolean;
    is_binary?: boolean;
    body_base64?: string;
    elapsed_ms: number;
    size: number;
  };
  assertions: Outcome[];
  tests: Outcome[];
  logs: string[];
  vars: Record<string, string>;
  passed: boolean;
}

export function emptyRequest(name: string, method: Method = "GET"): Request {
  return {
    name,
    method,
    url: "{{base_url}}/",
    headers: {},
    query: {},
    disabled_query: [],
    disabled_headers: [],
    auth: { type: "inherit" },
    body: { type: "none" },
    scripts: { pre: "", post: "", test: "" },
    expect: { headers: {}, body_contains: [] },
    docs: "",
  };
}

/** Rellena los campos que serde omite cuando están vacíos. */
export function normalizeRequest(r: Partial<Request> & { name: string; url: string }): Request {
  return {
    name: r.name,
    type: r.type ?? requestType(r as Request),
    method: r.method ?? "GET",
    url: r.url,
    headers: r.headers ?? {},
    header_descriptions: r.header_descriptions ?? {},
    query_descriptions: r.query_descriptions ?? {},
    query: r.query ?? {},
    disabled_query: r.disabled_query ?? [],
    disabled_headers: r.disabled_headers ?? [],
    auth: normalizeAuth(r.auth as Auth | undefined),
    body: r.body?.type === "graphql" ? { ...r.body, variables: r.body.variables ?? "", operation_name: r.body.operation_name ?? "" } : r.body ?? { type: "none" },
    scripts: { pre: "", post: "", test: "", ...(r.scripts ?? {}) },
    expect: { headers: {}, body_contains: [], ...(r.expect ?? {}) },
    docs: r.docs ?? "",
  };
}

export function normalizeCollection(c: Collection): Collection {
  const requests: Record<string, Request> = {};
  for (const [k, r] of Object.entries(c.requests)) requests[k] = normalizeRequest(r);
  const environments: Record<string, Environment> = {};
  for (const [k, e] of Object.entries(c.environments ?? {}))
    environments[k] = { vars: e?.vars ?? {}, secrets: e?.secrets ?? [], descriptions: e?.descriptions ?? {} };
  const meta: CollectionMeta = {
    ...c.meta,
    vars: c.meta.vars ?? {},
    secrets: c.meta.secrets ?? [],
    auth: normalizeAuth(c.meta.auth ?? { type: "none" }),
    settings: Object.assign({ timeout_ms: 30000, follow_redirects: true, insecure: false }, c.meta.settings ?? {}),
    scripts: Object.assign({ pre: "", post: "", test: "" }, c.meta.scripts ?? {}),
  };
  const folder_configs: Record<string, FolderConfig> = {};
  for (const [k, f] of Object.entries(c.folder_configs ?? {})) folder_configs[k] = { ...f, vars: f?.vars ?? {}, secrets: f?.secrets ?? [], headers: f?.headers ?? {}, auth: normalizeAuth(f?.auth), scripts: { pre: f?.scripts?.pre ?? "", post: f?.scripts?.post ?? "", test: f?.scripts?.test ?? "" } };
  return { ...c, meta, environments, requests, folder_configs };
}

export interface StoredCookie {
  name: string;
  value: string;
  domain: string;
  path: string;
  secure: boolean;
  http_only: boolean;
  session: boolean;
}

export interface HistoryEntry {
  at: string;
  env?: string;
  method: Method;
  url: string;
  status: number;
  elapsed_ms: number;
  size: number;
  headers: Record<string, string>;
  body: string;
  truncated: boolean;
  checks_passed: number;
  checks_total: number;
}

/** Conversión explícita: conserva lo que puede reutilizar el protocolo de destino. */
export function convertRequestType(source: Request, type: RequestType): { request: Request; warnings: string[] } {
  const request: Request = JSON.parse(JSON.stringify(source));
  const warnings: string[] = [];
  const previous = requestType(source);
  if (previous === type) return { request, warnings };
  request.type = type;
  if (type === "websocket" || type === "sse") {
    if (request.body.type !== "none") warnings.push("Se quitará el cuerpo de la solicitud.");
    if (request.scripts.pre || request.scripts.post || request.scripts.test) warnings.push("Se conservarán los scripts, pero no se ejecutarán en la conexión.");
    if (request.expect.status !== undefined || request.expect.max_time_ms !== undefined || request.expect.body_contains.length || Object.keys(request.expect.headers).length) warnings.push("Se conservarán las aserciones, pero no se evaluarán en la conexión.");
    request.body = { type: "none" }; request.method = "GET";
  } else if (type === "graphql") {
    const body = request.body;
    let values: { query?: unknown; variables?: unknown; operationName?: unknown } = {};
    if (body.type === "json") { try { values = JSON.parse(body.json); } catch { /* Se avisará antes de sustituir el cuerpo. */ } }
    if (body.type !== "graphql") {
      const imported = values && typeof values === "object" && typeof values.query === "string";
      if (body.type !== "none" && !imported) warnings.push("Se sustituirá el cuerpo actual por una consulta GraphQL vacía.");
      request.body = { type: "graphql", query: imported ? values.query as string : "", variables: JSON.stringify(imported ? values.variables ?? {} : {}, null, 2), operation_name: imported && typeof values.operationName === "string" ? values.operationName : "" };
    }
    if (request.method !== "POST") warnings.push("El método cambiará a POST.");
    request.method = "POST";
  } else if (request.body.type === "graphql") {
    const body = request.body;
    let variables: unknown;
    try { variables = JSON.parse(body.variables || "{}"); } catch { throw new Error("Corrige las variables JSON de GraphQL antes de convertir a HTTP."); }
    request.body = { type: "json", json: JSON.stringify({ query: body.query, variables, ...(body.operation_name ? { operationName: body.operation_name } : {}) }, null, 2) };
    warnings.push("La consulta GraphQL se convertirá en un cuerpo JSON.");
  }
  if (type === "websocket") request.url = request.url.replace(/^https:/i, "wss:").replace(/^http:/i, "ws:");
  else request.url = request.url.replace(/^wss:/i, "https:").replace(/^ws:/i, "http:");
  for (const key of Object.keys(request.headers)) {
    if (key.toLowerCase() !== "accept") continue;
    if (previous === "sse" && type !== "sse") {
      const remaining = request.headers[key].split(",").filter((part) => part.split(";")[0].trim().toLowerCase() !== "text/event-stream").join(",").trim();
      if (remaining) request.headers[key] = remaining; else delete request.headers[key];
    } else if (type === "sse") {
      if (request.headers[key].toLowerCase() !== "text/event-stream") warnings.push("La cabecera Accept cambiará a text/event-stream.");
      delete request.headers[key];
    }
  }
  if (type === "sse") request.headers.Accept = "text/event-stream";
  return { request, warnings };
}


export function enabledHeaders(request: Request): Record<string, string> {
  const disabled = new Set((request.disabled_headers ?? []).map((key) => key.toLowerCase()));
  return Object.fromEntries(Object.entries(request.headers ?? {}).filter(([key]) => !disabled.has(key.toLowerCase())));
}
