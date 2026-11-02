// Puente con el backend. Dentro de Tauri usa `invoke`; en un navegador
// normal (desarrollo de la UI, capturas) usa un mock con la colección de ejemplo.

import type { Collection, CollectionMeta, Environment, Exchange, Request } from "./types";
import { emptyRequest, normalizeCollection } from "./types";

export interface Api {
  openCollection(path: string): Promise<Collection>;
  initCollection(path: string, name: string): Promise<Collection>;
  pickFolder(): Promise<string | null>;
  saveRequest(root: string, key: string, request: Request): Promise<void>;
  deleteRequest(root: string, key: string): Promise<Collection>;
  renameRequest(root: string, from: string, to: string): Promise<Collection>;
  createFolder(root: string, folder: string): Promise<Collection>;
  renameFolder(root: string, from: string, to: string): Promise<Collection>;
  deleteFolder(root: string, folder: string): Promise<Collection>;
  listFolders(root: string): Promise<string[]>;
  saveEnvironment(root: string, name: string, env: Environment): Promise<Collection>;
  deleteEnvironment(root: string, name: string): Promise<Collection>;
  saveMeta(root: string, meta: CollectionMeta): Promise<Collection>;
  sendRequest(root: string, env: string | null, request: Request): Promise<Exchange>;
  runtimeVars(root: string): Promise<Record<string, string>>;
  saveRuntimeVars(root: string, vars: Record<string, string>): Promise<void>;
}

const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

async function tauriApi(): Promise<Api> {
  const { invoke } = await import("@tauri-apps/api/core");
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
    saveRequest: (root, key, request) => invoke("save_request", { root, key, request }),
    deleteRequest: (root, key) => col("delete_request", { root, key }),
    renameRequest: (root, from, to) => col("rename_request", { root, from, to }),
    createFolder: (root, folder) => col("create_folder", { root, folder }),
    renameFolder: (root, from, to) => col("rename_folder", { root, from, to }),
    deleteFolder: (root, folder) => col("delete_folder", { root, folder }),
    listFolders: (root) => invoke<string[]>("list_folders", { root }),
    saveEnvironment: (root, name, env) => col("save_environment", { root, name, env }),
    deleteEnvironment: (root, name) => col("delete_environment", { root, name }),
    saveMeta: (root, meta) => col("save_meta", { root, meta }),
    sendRequest: (root, env, request) =>
      invoke<Exchange>("send_request", { args: { root, env, request } }),
    runtimeVars: (root) => invoke<Record<string, string>>("runtime_vars", { root }),
    saveRuntimeVars: (root, vars) => invoke("save_runtime_vars", { root, vars }),
  };
}

function mockApi(): Api {
  const col: Collection = {
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
      "user/login": { ...emptyRequest("Login"), url: "{{base_url}}/{{api_version}}/user/login", query: { username: "ada", password: "{{password}}" } },
    },
  };
  const folders = new Set<string>(["pets", "store", "user", "admin"]);
  let runtime: Record<string, string> = { token: "demo-token" };
  const snapshot = async () => structuredClone(col);
  const vars: Record<string, string> = { base_url: "http://localhost:8080", api_version: "v2", pet_id: "42" };
  const interp = (s: string) => s.replace(/\{\{\s*(\w+)\s*\}\}/g, (_, k) => vars[k] ?? `{{${k}}}`);
  const sortRequests = () => {
    const entries = Object.entries(col.requests).sort(([a], [b]) => a.localeCompare(b));
    col.requests = Object.fromEntries(entries);
  };

  return {
    openCollection: snapshot,
    initCollection: async (path, name) => ({ ...structuredClone(col), root: path, meta: { ...col.meta, name } }),
    pickFolder: async () => col.root,
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
      sortRequests();
      return snapshot();
    },
    deleteFolder: async (_r, folder) => {
      for (const k of Object.keys(col.requests)) if (k.startsWith(folder + "/")) delete col.requests[k];
      for (const f of [...folders]) if (f === folder || f.startsWith(folder + "/")) folders.delete(f);
      return snapshot();
    },
    listFolders: async () => [...folders].sort(),
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
    sendRequest: async (_root, _env, request) => {
      await new Promise((r) => setTimeout(r, 180 + Math.random() * 120));
      const url = interp(request.url);
      const status = request.method === "POST" ? 201 : url.includes("inventory") ? 500 : 200;
      const body =
        status === 500
          ? { error: "boom", trace_id: "c1f9e0" }
          : request.method === "POST"
            ? { id: 42, ...JSON.parse(request.body.type === "json" ? request.body.json : "{}") }
            : { pets: [{ id: 1, name: "Rex", status: "available", tags: ["dog", "friendly"] }, { id: 2, name: "Misu", status: "available", tags: ["cat"] }], total: 2 };
      const text = JSON.stringify(body, null, 2);
      const elapsed = Math.round(14 + Math.random() * 80);
      const assertions = [];
      if (request.expect.status !== undefined)
        assertions.push({ name: `status == ${request.expect.status}`, passed: status === request.expect.status, detail: status === request.expect.status ? undefined : `recibido ${status}` });
      for (const [k, v] of Object.entries(request.expect.headers))
        assertions.push({ name: `header ${k} == "${v}"`, passed: k.toLowerCase() === "content-type" && v === "application/json" });
      for (const f of request.expect.body_contains)
        assertions.push({ name: `body contains ${JSON.stringify(f)}`, passed: text.includes(f), detail: text.includes(f) ? undefined : "no aparece en el cuerpo" });
      if (request.expect.max_time_ms !== undefined)
        assertions.push({ name: `time <= ${request.expect.max_time_ms}ms`, passed: elapsed <= request.expect.max_time_ms });
      // Simulación de scripts: en la app real los ejecuta QuickJS en Rust.
      const tests = [];
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
        sent: { method: request.method, url, headers: { ...request.headers, api_key: "dev-key" }, query: request.query, body: request.body.type === "json" ? request.body.json : undefined },
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
    },
    runtimeVars: async () => ({ ...runtime }),
    saveRuntimeVars: async (_r, vars) => {
      runtime = { ...vars };
    },
  };
}

/** `a/b/c` → `["a", "a/b"]` (las carpetas que contienen la clave). */
export function folderChain(key: string): string[] {
  const parts = key.split("/");
  const out: string[] = [];
  for (let i = 1; i < parts.length; i++) out.push(parts.slice(0, i).join("/"));
  return out;
}

export const api: Promise<Api> = inTauri ? tauriApi() : Promise.resolve(mockApi());
export const isMock = !inTauri;
