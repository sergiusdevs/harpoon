// Espejo de los tipos de `harpoon-core` tal y como los serializa serde.

export type Method = "GET" | "POST" | "PUT" | "PATCH" | "DELETE" | "HEAD" | "OPTIONS";
export const METHODS: Method[] = ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];

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

export type Body =
  | { type: "none" }
  | { type: "json"; json: string }
  | { type: "text"; text: string }
  | { type: "form"; fields: Record<string, string> };

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
  name: string;
  method: Method;
  url: string;
  headers: Record<string, string>;
  query: Record<string, string>;
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
  vars: Record<string, string>;
  secrets: string[];
}

export interface Collection {
  root: string;
  meta: CollectionMeta;
  environments: Record<string, Environment>;
  requests: Record<string, Request>;
}

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
  };
  response: {
    status: number;
    headers: Record<string, string>;
    body: string;
    is_json: boolean;
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
    method: r.method ?? "GET",
    url: r.url,
    headers: r.headers ?? {},
    query: r.query ?? {},
    auth: normalizeAuth(r.auth as Auth | undefined),
    body: r.body ?? { type: "none" },
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
    environments[k] = { vars: e?.vars ?? {}, secrets: e?.secrets ?? [] };
  const meta: CollectionMeta = {
    ...c.meta,
    vars: c.meta.vars ?? {},
    auth: normalizeAuth(c.meta.auth ?? { type: "none" }),
    settings: Object.assign({ timeout_ms: 30000, follow_redirects: true, insecure: false }, c.meta.settings ?? {}),
    scripts: Object.assign({ pre: "", post: "", test: "" }, c.meta.scripts ?? {}),
  };
  return { ...c, meta, environments, requests };
}
