// Puente con el backend. Dentro de Tauri usa `invoke`; en un navegador
// normal (desarrollo de la UI, capturas) usa un mock con la colección de ejemplo.

import type { Collection, Exchange, Request } from "./types";
import { emptyRequest, normalizeCollection } from "./types";

export interface Api {
  openCollection(path: string): Promise<Collection>;
  initCollection(path: string, name: string): Promise<Collection>;
  pickFolder(): Promise<string | null>;
  saveRequest(root: string, key: string, request: Request): Promise<void>;
  sendRequest(root: string, env: string | null, request: Request): Promise<Exchange>;
}

const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

async function tauriApi(): Promise<Api> {
  const { invoke } = await import("@tauri-apps/api/core");
  const { open } = await import("@tauri-apps/plugin-dialog");
  return {
    openCollection: async (path) => normalizeCollection(await invoke<Collection>("open_collection", { path })),
    initCollection: async (path, name) => normalizeCollection(await invoke<Collection>("init_collection", { path, name })),
    pickFolder: async () => {
      const r = await open({ directory: true, multiple: false, title: "Abrir colección" });
      return typeof r === "string" ? r : null;
    },
    saveRequest: (root, key, request) => invoke("save_request", { root, key, request }),
    sendRequest: (root, env, request) =>
      invoke<Exchange>("send_request", { args: { root, env, request } }),
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
    },
    environments: ["dev", "prod"],
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
  const vars: Record<string, string> = { base_url: "http://localhost:8080", api_version: "v2", pet_id: "42" };
  const interp = (s: string) => s.replace(/\{\{\s*(\w+)\s*\}\}/g, (_, k) => vars[k] ?? `{{${k}}}`);

  return {
    openCollection: async () => structuredClone(col),
    initCollection: async (path, name) => ({ ...structuredClone(col), root: path, meta: { ...col.meta, name } }),
    pickFolder: async () => col.root,
    saveRequest: async (_root, key, request) => {
      col.requests[key] = structuredClone(request);
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
        passed: assertions.every((a) => a.passed),
      };
    },
  };
}

export const api: Promise<Api> = inTauri ? tauriApi() : Promise.resolve(mockApi());
export const isMock = !inTauri;
