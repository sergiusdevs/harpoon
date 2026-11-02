// Puente con el backend. Dentro de Tauri usa `invoke`; en un navegador
// normal (desarrollo de la UI, capturas) usa un mock con la colección de ejemplo.

import type { Collection, CollectionMeta, Environment, Exchange, HistoryEntry, Outcome, Request, StoredCookie } from "../domain/types";
import type { CurlPrepared } from "../domain/curlExport";
import { folderChain } from "../domain/paths";
export { folderChain } from "../domain/paths";
import { emptyRequest, normalizeCollection } from "../domain/types";

export interface SharePreview { name: string; warnings: string[] }
export interface StreamEvent { kind: "open" | "closed" | "error" | "message" | "sent"; data: string; event: string; id: string }

export interface Api {
  openCollection(path: string): Promise<Collection>;
  initCollection(path: string, name: string): Promise<Collection>;
  pickFolder(): Promise<string | null>;
  pickFile(): Promise<string | null>;
  saveRequest(root: string, key: string, request: Request): Promise<void>;
  deleteRequest(root: string, key: string): Promise<Collection>;
  renameRequest(root: string, from: string, to: string): Promise<Collection>;
  createFolder(root: string, folder: string): Promise<Collection>;
  renameFolder(root: string, from: string, to: string): Promise<Collection>;
  deleteFolder(root: string, folder: string): Promise<Collection>;
  listFolders(root: string): Promise<string[]>;
  saveFolderVars(root: string, folder: string, vars: Record<string, string>): Promise<Collection>;
  resolveUrl(root: string, key: string, env: string | null, url: string): Promise<string>;
  saveEnvironment(root: string, name: string, env: Environment): Promise<Collection>;
  deleteEnvironment(root: string, name: string): Promise<Collection>;
  saveMeta(root: string, meta: CollectionMeta): Promise<Collection>;
  sendRequest(root: string, env: string | null, request: Request, key: string): Promise<Exchange>;
  exportHarpoon(root: string, name: string): Promise<string[] | null>;
  previewHarpoon(path: string): Promise<SharePreview>;
  importHarpoon(path: string, parent: string, name: string): Promise<Collection>;
  exportPostman(root: string, name: string): Promise<string[] | null>;
  previewPostman(path: string): Promise<SharePreview>;
  importPostman(path: string, parent: string, name: string): Promise<Collection>;
  previewOpenapi(path: string): Promise<SharePreview>;
  importOpenapi(path: string, parent: string, name: string): Promise<Collection>;
  startStream(root: string, key: string, env: string | null, request: Request, protocol: "websocket" | "sse", id: string, onEvent: (event: StreamEvent) => void): Promise<void>;
  sendStream(id: string, data: string): Promise<void>;
  stopStream(id: string): Promise<void>;
  listCookies(root: string): Promise<StoredCookie[]>;
  addCookie(root: string, url: string, cookie: string): Promise<void>;
  removeCookie(root: string, domain: string, path: string, name: string): Promise<void>;
  history(root: string, key: string): Promise<HistoryEntry[]>;
  clearHistory(root: string, key: string): Promise<void>;
  prepareCurl(root: string, key: string, env: string | null, request: Request): Promise<CurlPrepared>;
  saveResponse(filename: string, bytes: Uint8Array): Promise<void>;
  runtimeVars(root: string): Promise<Record<string, string>>;
  saveRuntimeVars(root: string, vars: Record<string, string>): Promise<void>;
}

const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

async function tauriApi(): Promise<Api> {
  const { invoke, Channel } = await import("@tauri-apps/api/core");
  const { open } = await import("@tauri-apps/plugin-dialog");
  const col = async (cmd: string, args: Record<string, unknown>) =>
    normalizeCollection(await invoke<Collection>(cmd, args));
  return {
    openCollection: (path) => col("open_collection", { path }),
    initCollection: (path, name) => col("init_collection", { path, name }),
    pickFolder: async () => {
      const r = await open({ directory: true, multiple: false, title: "Abrir colección" });
      return typeof r === "string" ? r : null;
    },
    pickFile: async () => {
      const result = await open({ directory: false, multiple: false, title: "Seleccionar archivo" });
      return typeof result === "string" ? result : null;
    },
    saveRequest: (root, key, request) => invoke("save_request", { root, key, request }),
    deleteRequest: (root, key) => col("delete_request", { root, key }),
    renameRequest: (root, from, to) => col("rename_request", { root, from, to }),
    createFolder: (root, folder) => col("create_folder", { root, folder }),
    renameFolder: (root, from, to) => col("rename_folder", { root, from, to }),
    deleteFolder: (root, folder) => col("delete_folder", { root, folder }),
    listFolders: (root) => invoke<string[]>("list_folders", { root }),
    saveFolderVars: (root, folder, vars) => col("save_folder_vars", { root, folder, vars }),
    resolveUrl: (root, key, env, url) => invoke<string>("resolve_url", { root, key, env, url }),
    saveEnvironment: (root, name, env) => col("save_environment", { root, name, env }),
    deleteEnvironment: (root, name) => col("delete_environment", { root, name }),
    saveMeta: (root, meta) => col("save_meta", { root, meta }),
    sendRequest: (root, env, request, key) =>
      invoke<Exchange>("send_request", { args: { root, env, key, request } }),
    prepareCurl: (root, key, env, request) => invoke<CurlPrepared>("prepare_curl", { root, key, env, request }),
    saveResponse: async (filename, bytes) => {
      const { save } = await import("@tauri-apps/plugin-dialog");
      const path = await save({ defaultPath: filename, title: "Guardar respuesta" });
      if (path) await invoke("save_response_file", { path, bytes: Array.from(bytes) });
    },
    exportHarpoon: async (root, name) => {
      const { save } = await import("@tauri-apps/plugin-dialog");
      const path = await save({ title: "Exportar colección Harpoon", defaultPath: `${name}.harpoon.json`, filters: [{ name: "Colección Harpoon", extensions: ["json"] }] });
      return path ? invoke<string[]>("export_harpoon", { root, path }) : null;
    },
    previewHarpoon: (path) => invoke<SharePreview>("preview_harpoon", { path }),
    importHarpoon: (path, parent, name) => col("import_harpoon", { path, parent, name }),
    exportPostman: async (root, name) => {
      const { save } = await import("@tauri-apps/plugin-dialog");
      const path = await save({ title: "Exportar colección Postman", defaultPath: `${name}.postman_collection.json`, filters: [{ name: "Colección Postman", extensions: ["json"] }] });
      return path ? invoke<string[]>("export_postman", { root, path }) : null;
    },
    previewPostman: (path) => invoke<SharePreview>("preview_postman", { path }),
    importPostman: (path, parent, name) => col("import_postman", { path, parent, name }),
    previewOpenapi: (path) => invoke<SharePreview>("preview_openapi", { path }),
    importOpenapi: (path, parent, name) => col("import_openapi", { path, parent, name }),
    startStream: (root, key, env, request, protocol, id, onEvent) => {
      const channel = new Channel<StreamEvent>();
      channel.onmessage = onEvent;
      return invoke("start_stream", { root, key, env, request, protocol, id, channel });
    },
    sendStream: (id, data) => invoke("stream_send", { id, data }),
    stopStream: (id) => invoke("stop_stream", { id }),
    listCookies: (root) => invoke<StoredCookie[]>("list_cookies", { root }),
    addCookie: (root, url, cookie) => invoke("add_cookie", { root, url, cookie }),
    removeCookie: (root, domain, path, name) => invoke("remove_cookie", { root, domain, path, name }),
    history: (root, key) => invoke<HistoryEntry[]>("history", { root, key }),
    clearHistory: (root, key) => invoke("clear_history", { root, key }),
    runtimeVars: (root) => invoke<Record<string, string>>("runtime_vars", { root }),
    saveRuntimeVars: (root, vars) => invoke("save_runtime_vars", { root, vars }),
  };
}

function mockApi(): Api {
  const col: Collection = {
    folder_configs: {},
    root: "/Users/sergius/workspace/projects/dev-api/examples/petstore",
    meta: {
      name: "Petstore",
      format: 1,
      default_environment: "dev",
      vars: { api_version: "v2" },
      auth: { type: "api_key", key: "api_key", value: "{{api_key}}", in: "header" },
      settings: { timeout_ms: 30000, follow_redirects: true, insecure: false },
      scripts: { pre: "", post: "", test: "test('responde JSON', () => expect(harpoon.res.header('content-type')).to.include('json'));" },
    },
    environments: {
      dev: { vars: { base_url: "http://localhost:8080", api_key: "dev-key-no-secreta" }, secrets: ["api_key"] },
      prod: { vars: { base_url: "https://petstore.example.com" }, secrets: ["api_key"] },
    },
    requests: {
      "pets/list": {
        ...emptyRequest("Listar mascotas"),
        url: "{{base_url}}/{{api_version}}/pet/findByStatus",
        query: { status: "available" },
        headers: { Accept: "application/json" },
        expect: { status: 200, headers: { "Content-Type": "application/json" }, body_contains: [] },
        docs: "Devuelve las mascotas con el estado indicado.",
      },
      "pets/create": {
        ...emptyRequest("Crear mascota", "POST"),
        url: "{{base_url}}/{{api_version}}/pet",
        headers: { "Content-Type": "application/json" },
        body: { type: "json", json: '{\n  "name": "Firulais",\n  "status": "available"\n}' },
        expect: { status: 201, headers: {}, body_contains: ['"id"'], max_time_ms: 500 },
        scripts: {
          pre: 'harpoon.req.headers["X-Request-Id"] = "req-" + Date.now();',
          post: 'harpoon.vars.set("pet_id", harpoon.res.json().id);\nconsole.log("pet_id =", harpoon.vars.get("pet_id"));',
          test: 'test("devuelve id numérico", () => expect(harpoon.res.json().id).to.be.a("number"));\ntest("eco del nombre", () => expect(harpoon.res.json().name).to.equal("Firulais"));\ntest("rápido", () => expect(harpoon.res.elapsed_ms).to.be.below(50));',
        },
      },
      "pets/update": { ...emptyRequest("Actualizar mascota", "PUT"), url: "{{base_url}}/{{api_version}}/pet/{{pet_id}}" },
      "pets/delete": { ...emptyRequest("Borrar mascota", "DELETE"), url: "{{base_url}}/{{api_version}}/pet/{{pet_id}}" },
      "store/inventory": {
        ...emptyRequest("Inventario"),
        url: "{{base_url}}/{{api_version}}/store/inventory",
        auth: { type: "none" },
        expect: { status: 200, headers: {}, body_contains: [], max_time_ms: 2000 },
      },
      "docs/home": { ...emptyRequest("Página de docs"), url: "{{base_url}}/docs", docs: "Devuelve HTML: prueba la pestaña Vista." },
      "docs/logo": { ...emptyRequest("Logo (PNG)"), url: "{{base_url}}/logo", docs: "Devuelve una imagen y cookies." },
      "user/login": { ...emptyRequest("Login"), url: "{{base_url}}/{{api_version}}/user/login", query: { username: "ada", password: "{{password}}" } },
    },
  };
  const folders = new Set<string>(["pets", "store", "user", "admin", "docs"]);
  let runtime: Record<string, string> = { token: "demo-token" };
  let mockCookies: StoredCookie[] = [];
  const hist: Record<string, HistoryEntry[]> = {};
  const snapshot = async () => structuredClone(col);
  const resolveMockUrl = (key: string, env: string | null, url: string) => {
    const values = { ...col.meta.vars };
    for (const folder of folderChain(key)) Object.assign(values, col.folder_configs[folder]?.vars ?? {});
    Object.assign(values, col.environments[env ?? col.meta.default_environment ?? ""]?.vars ?? {}, runtime);
    return url.replace(/\{\{\s*([^{}]+?)\s*\}\}/g, (match, name: string) => values[name] ?? match);
  };
  const sortRequests = () => {
    const entries = Object.entries(col.requests).sort(([a], [b]) => a.localeCompare(b));
    col.requests = Object.fromEntries(entries);
  };

  return {
    openCollection: snapshot,
    initCollection: async (path, name) => ({ ...structuredClone(col), root: path, meta: { ...col.meta, name } }),
    pickFolder: async () => col.root,
    pickFile: async () => null,
    saveRequest: async (_root, key, request) => {
      col.requests[key] = structuredClone(request);
      for (const f of folderChain(key)) folders.add(f);
      sortRequests();
    },
    deleteRequest: async (_r, key) => {
      delete col.requests[key];
      return snapshot();
    },
    renameRequest: async (_r, from, to) => {
      if (col.requests[to]) throw `ya existe: ${to}`;
      col.requests[to] = col.requests[from];
      delete col.requests[from];
      for (const f of folderChain(to)) folders.add(f);
      sortRequests();
      return snapshot();
    },
    createFolder: async (_r, folder) => {
      for (const f of folderChain(folder + "/x")) folders.add(f);
      return snapshot();
    },
    renameFolder: async (_r, from, to) => {
      for (const k of Object.keys(col.requests))
        if (k.startsWith(from + "/")) {
          col.requests[to + k.slice(from.length)] = col.requests[k];
          delete col.requests[k];
        }
      for (const f of [...folders])
        if (f === from || f.startsWith(from + "/")) {
          folders.delete(f);
          folders.add(to + f.slice(from.length));
        }
      for (const f of Object.keys(col.folder_configs)) if (f === from || f.startsWith(from + "/")) {
        col.folder_configs[to + f.slice(from.length)] = col.folder_configs[f];
        delete col.folder_configs[f];
      }
      sortRequests();
      return snapshot();
    },
    deleteFolder: async (_r, folder) => {
      for (const k of Object.keys(col.requests)) if (k.startsWith(folder + "/")) delete col.requests[k];
      for (const f of [...folders]) if (f === folder || f.startsWith(folder + "/")) folders.delete(f);
      for (const f of Object.keys(col.folder_configs)) if (f === folder || f.startsWith(folder + "/")) delete col.folder_configs[f];
      return snapshot();
    },
    listFolders: async () => [...folders].sort(),
    saveFolderVars: async (_root, folder, vars) => {
      col.folder_configs[folder] = { vars: structuredClone(vars) };
      return snapshot();
    },
    resolveUrl: async (_root, key, env, url) => {
      return resolveMockUrl(key, env, url);
    },
    saveEnvironment: async (_r, name, env) => {
      col.environments[name] = structuredClone(env);
      return snapshot();
    },
    deleteEnvironment: async (_r, name) => {
      delete col.environments[name];
      if (col.meta.default_environment === name) col.meta.default_environment = null;
      return snapshot();
    },
    saveMeta: async (_r, meta) => {
      col.meta = structuredClone(meta);
      return snapshot();
    },
    sendRequest: async (_root, env, request, key): Promise<Exchange> => {
      const x = await mockSend(request, key, env);
      (hist[key] ??= []).unshift({
        at: new Date().toISOString().slice(0, 19) + "Z",
        env: env ?? undefined,
        method: request.method,
        url: x.sent.url,
        status: x.response.status,
        elapsed_ms: x.response.elapsed_ms,
        size: x.response.size,
        headers: x.response.headers,
        body: x.response.body,
        truncated: false,
        checks_passed: [...x.assertions, ...x.tests].filter((c) => c.passed).length,
        checks_total: x.assertions.length + x.tests.length,
      });
      return x;
    },
    prepareCurl: async (_root, key, env, request) => {
      if (col.meta.scripts.pre || request.scripts.pre) throw new Error("La exportación con scripts previos requiere la aplicación de escritorio");
      const values = { ...col.meta.vars };
      for (const folder of folderChain(key)) Object.assign(values, col.folder_configs[folder]?.vars ?? {});
      Object.assign(values, col.environments[env ?? col.meta.default_environment ?? ""]?.vars ?? {}, runtime);
      const resolve = (value: string) => value.replace(/\{\{\s*([^{}]+?)\s*\}\}/g, (_, name: string) => {
        if (!(name in values)) throw new Error(`Variable sin definir: ${name}`);
        return values[name];
      });
      const headers: Record<string, string> = {}, query: Record<string, string> = {};
      for (const [name, value] of Object.entries(request.headers)) headers[resolve(name)] = resolve(value);
      for (const [name, value] of Object.entries(request.query)) if (!request.disabled_query.includes(name)) query[resolve(name)] = resolve(value);
      const auth = request.auth.type === "inherit" ? col.meta.auth : request.auth;
      let digest: CurlPrepared["digest"] = null;
      if (auth.type === "oauth2") throw new Error("OAuth2 necesita obtener un token al enviar y no se puede exportar automáticamente a cURL.");
      if (auth.type === "digest") digest = { username: resolve(auth.username), password: resolve(auth.password) };
      if (auth.type === "bearer") headers.Authorization = `Bearer ${resolve(auth.token)}`;
      if (auth.type === "basic") {
        const bytes = new TextEncoder().encode(`${resolve(auth.username)}:${resolve(auth.password)}`);
        headers.Authorization = `Basic ${btoa(Array.from(bytes, (byte) => String.fromCharCode(byte)).join(""))}`;
      }
      if (auth.type === "api_key") (auth.in === "query" ? query : headers)[resolve(auth.key)] = resolve(auth.value);
      let body: string | undefined;
      if (request.body.type === "json") { body = resolve(request.body.json); headers["Content-Type"] ??= "application/json"; }
      else if (request.body.type === "text") body = resolve(request.body.text);
      else if (request.body.type === "xml") { body = resolve(request.body.xml); if (!Object.keys(headers).some((name) => name.toLowerCase() === "content-type")) headers["Content-Type"] = "application/xml"; }
      if (request.body.type === "binary" && !Object.keys(headers).some((name) => name.toLowerCase() === "content-type")) headers["Content-Type"] = "application/octet-stream";
      else if (request.body.type === "form") {
        body = new URLSearchParams(Object.entries(request.body.fields).map(([k, v]) => [k, resolve(v)])).toString();
        headers["Content-Type"] ??= "application/x-www-form-urlencoded";
      }
      const multipart = request.body.type === "multipart" ? request.body.parts.map((part) => part.type === "text"
        ? { type: "text" as const, name: resolve(part.name), value: resolve(part.value) }
        : { type: "file" as const, name: resolve(part.name), path: resolve(part.path), content_type: part.content_type ? resolve(part.content_type) : undefined }) : undefined;
      if (multipart && Object.keys(headers).some((name) => name.toLowerCase() === "content-type")) throw new Error("Elimina Content-Type para generar el límite multipart automáticamente");
      const binary_path = request.body.type === "binary" ? resolve(request.body.path) : undefined;
      return { request: { method: request.method, url: resolve(request.url), headers, query, body, multipart, binary_path }, digest, settings: col.meta.settings };
    },
    saveResponse: async (filename, bytes) => {
      const url = URL.createObjectURL(new Blob([new Uint8Array(bytes)]));
      const link = document.createElement("a");
      link.href = url; link.download = filename; link.click();
      setTimeout(() => URL.revokeObjectURL(url), 1000);
    },
    exportHarpoon: async () => { throw new Error("Disponible en la aplicación de escritorio"); },
    previewHarpoon: async () => { throw new Error("Disponible en la aplicación de escritorio"); },
    importHarpoon: async () => { throw new Error("Disponible en la aplicación de escritorio"); },
    exportPostman: async () => { throw new Error("Disponible en la aplicación de escritorio"); },
    previewPostman: async () => { throw new Error("Disponible en la aplicación de escritorio"); },
    importPostman: async () => { throw new Error("Disponible en la aplicación de escritorio"); },
    previewOpenapi: async () => { throw new Error("Disponible en la aplicación de escritorio"); },
    importOpenapi: async () => { throw new Error("Disponible en la aplicación de escritorio"); },
    startStream: async () => { throw new Error("Disponible en la aplicación de escritorio"); },
    sendStream: async () => { throw new Error("Disponible en la aplicación de escritorio"); },
    stopStream: async () => { throw new Error("Disponible en la aplicación de escritorio"); },
    listCookies: async () => structuredClone(mockCookies),
    addCookie: async (_root, url, cookie) => {
      const parsed = new URL(url), pair = cookie.split(";", 1)[0], at = pair.indexOf("=");
      if (at <= 0 || !["http:", "https:"].includes(parsed.protocol)) throw new Error("Cookie o URL inválida");
      const entry: StoredCookie = { name: pair.slice(0, at).trim(), value: pair.slice(at + 1).trim(), domain: parsed.hostname, path: "/", secure: /;\s*secure(?:;|$)/i.test(cookie), http_only: /;\s*httponly(?:;|$)/i.test(cookie), session: !/;\s*(expires|max-age)=/i.test(cookie) };
      mockCookies = [...mockCookies.filter((c) => c.name !== entry.name || c.domain !== entry.domain || c.path !== entry.path), entry];
    },
    removeCookie: async (_root, domain, path, name) => { mockCookies = mockCookies.filter((c) => c.domain !== domain || c.path !== path || c.name !== name); },
    history: async (_r, key) => structuredClone(hist[key] ?? []),
    clearHistory: async (_r, key) => {
      delete hist[key];
    },
    runtimeVars: async () => ({ ...runtime }),
    saveRuntimeVars: async (_r, vars) => {
      runtime = { ...vars };
    },
  };

  async function mockSend(request: Request, key: string, env: string | null): Promise<Exchange> {
    {
      await new Promise((r) => setTimeout(r, 180 + Math.random() * 120));
      const url = resolveMockUrl(key, env, request.url);
      if (url.endsWith("/logo")) {
        const png = "iVBORw0KGgoAAAANSUhEUgAAAEAAAABACAYAAACqaXHeAAAAUklEQVR4nO3PsQ0AIAwDwcD+O8MGoEhBuC8tnZ3kAgAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAB4bgEWmwABNMI7oAAAAABJRU5ErkJggg==";
        return { sent: { method: request.method, url, headers: request.headers, query: Object.fromEntries(Object.entries(request.query).filter(([key]) => !request.disabled_query.includes(key))) }, response: { status: 200, headers: { "content-type": "image/png", "content-length": "150", "set-cookie": "session=abc123; Path=/; HttpOnly\ntheme=dark; Path=/; Max-Age=3600" }, body: "", is_json: false, is_binary: true, body_base64: png, elapsed_ms: 7, size: 150 }, assertions: [], tests: [], logs: [], vars: { ...runtime }, passed: true };
      }
      if (url.endsWith("/docs")) {
        const html = "<!doctype html><html><body style=\"font-family:sans-serif;padding:24px\"><h1>Petstore API</h1><p>Documentación <b>HTML</b> servida por la API.</p><ul><li>GET /pet</li><li>POST /pet</li></ul></body></html>";
        return { sent: { method: request.method, url, headers: request.headers, query: Object.fromEntries(Object.entries(request.query).filter(([key]) => !request.disabled_query.includes(key))) }, response: { status: 200, headers: { "content-type": "text/html; charset=utf-8", "content-length": String(html.length) }, body: html, is_json: false, elapsed_ms: 9, size: html.length }, assertions: [], tests: [], logs: [], vars: { ...runtime }, passed: true };
      }
      const status = request.method === "POST" ? 201 : url.includes("inventory") ? 500 : 200;
      const body =
        status === 500
          ? { error: "boom", trace_id: "c1f9e0" }
          : request.method === "POST"
            ? { id: 42, ...JSON.parse(request.body.type === "json" ? request.body.json : "{}") }
            : { pets: [{ id: 1, name: "Rex", status: "available", tags: ["dog", "friendly"] }, { id: 2, name: "Misu", status: "available", tags: ["cat"] }], total: 2 };
      const text = JSON.stringify(body, null, 2);
      const elapsed = Math.round(14 + Math.random() * 80);
      const assertions: Outcome[] = [];
      if (request.expect.status !== undefined)
        assertions.push({ name: `status == ${request.expect.status}`, passed: status === request.expect.status, detail: status === request.expect.status ? undefined : `recibido ${status}` });
      for (const [k, v] of Object.entries(request.expect.headers))
        assertions.push({ name: `header ${k} == "${v}"`, passed: k.toLowerCase() === "content-type" && v === "application/json" });
      for (const f of request.expect.body_contains)
        assertions.push({ name: `body contains ${JSON.stringify(f)}`, passed: text.includes(f), detail: text.includes(f) ? undefined : "no aparece en el cuerpo" });
      if (request.expect.max_time_ms !== undefined)
        assertions.push({ name: `time <= ${request.expect.max_time_ms}ms`, passed: elapsed <= request.expect.max_time_ms });
      // Simulación de scripts: en la app real los ejecuta QuickJS en Rust.
      const tests: Outcome[] = [];
      const logs: string[] = [];
      if (col.meta.scripts.test) tests.push({ name: "responde JSON", passed: true });
      if (request.scripts.post.includes("pet_id")) {
        runtime = { ...runtime, pet_id: String((body as { id?: number }).id ?? 42) };
        logs.push(`pet_id = ${runtime.pet_id}`);
      }
      if (request.scripts.test) {
        tests.push({ name: "devuelve id numérico", passed: true });
        tests.push({ name: "eco del nombre", passed: true });
        tests.push({ name: "rápido", passed: elapsed < 50, detail: elapsed < 50 ? undefined : `esperaba < 50, recibido ${elapsed}` });
      }
      return {
        sent: { method: request.method, url, headers: { ...request.headers, api_key: "dev-key" }, query: Object.fromEntries(Object.entries(request.query).filter(([key]) => !request.disabled_query.includes(key))), body: request.body.type === "json" ? request.body.json : undefined },
        response: {
          status,
          headers: { "content-type": "application/json", "content-length": String(text.length), server: "petstore/1.4.2", date: new Date().toUTCString() },
          body: text,
          is_json: true,
          elapsed_ms: elapsed,
          size: text.length,
        },
        assertions,
        tests,
        logs,
        vars: { ...runtime },
        passed: assertions.every((a) => a.passed) && tests.every((t) => t.passed),
      };
    }
  }
}

export const api: Promise<Api> = inTauri ? tauriApi() : Promise.resolve(mockApi());
export const isMock = !inTauri;
