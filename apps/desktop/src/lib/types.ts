// Espejo de los tipos de `harpoon-core` tal y como los serializa serde.

export type Method = "GET" | "POST" | "PUT" | "PATCH" | "DELETE" | "HEAD" | "OPTIONS";
export const METHODS: Method[] = ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];

export type Auth =
  | { type: "inherit" }
  | { type: "none" }
  | { type: "basic"; username: string; password: string }
  | { type: "bearer"; token: string }
  | { type: "api_key"; key: string; value: string; in: "header" | "query" };

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
  settings: { timeout_ms: number; follow_redirects: boolean; proxy?: string; insecure: boolean };
  scripts: Scripts;
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
  const auth = (r.auth ?? { type: "inherit" }) as Auth;
  return {
    name: r.name,
    method: r.method ?? "GET",
    url: r.url,
    headers: r.headers ?? {},
    query: r.query ?? {},
    auth: auth.type === "api_key" ? { ...auth, in: auth.in ?? "header" } : auth,
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
    auth: c.meta.auth ?? { type: "none" },
    settings: Object.assign({ timeout_ms: 30000, follow_redirects: true, insecure: false }, c.meta.settings ?? {}),
    scripts: Object.assign({ pre: "", post: "", test: "" }, c.meta.scripts ?? {}),
  };
  return { ...c, meta, environments, requests };
}
