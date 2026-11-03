import test from "node:test";
import assert from "node:assert/strict";
import { loadTypescript } from "./load-typescript.mjs";

const { normalizeRequest, normalizeCollection, normalizeAuth, enabledHeaders, newTypedRequest, requestType } = await loadTypescript("../src/domain/types.ts");

test("completa solicitudes antiguas sin perder datos", () => {
  const request = normalizeRequest({ name: "Salud", url: "{{base_url}}/health", headers: { Accept: "application/json" } });
  assert.equal(request.method, "GET");
  assert.deepEqual(request.headers, { Accept: "application/json" });
  assert.deepEqual(request.query, {});
  assert.deepEqual(request.auth, { type: "inherit" });
  assert.deepEqual(request.scripts, { pre: "", post: "", test: "" });
  assert.deepEqual(request.expect, { headers: {}, body_contains: [] });
});

test("normaliza colecciones sin configuraciones opcionales", () => {
  const collection = normalizeCollection({ root: "/demo", meta: { name: "Demo" }, environments: { dev: {} }, requests: { health: { name: "Salud", url: "/health" } } });
  assert.deepEqual(collection.folder_configs, {});
  assert.deepEqual(collection.environments.dev, { vars: {}, secrets: [], descriptions: {} });
  assert.equal(collection.requests.health.method, "GET");
  assert.equal(collection.meta.settings.timeout_ms, 30000);
});

test("completa los campos opcionales de autenticación", () => {
  assert.deepEqual(normalizeAuth({ type: "api_key", key: "x", value: "y" }), { type: "api_key", key: "x", value: "y", in: "header" });
  assert.deepEqual(normalizeAuth({ type: "oauth2", client_id: "demo" }).extra, {});
});


test("conserva el tipo de las solicitudes creadas al normalizarlas", () => {
  for (const type of ["http", "graphql", "websocket", "sse"]) {
    const request = newTypedRequest("Ejemplo", type, "DELETE");
    const restored = normalizeRequest(JSON.parse(JSON.stringify(request)));
    assert.equal(requestType(restored), type);
    assert.equal(restored.method, type === "http" ? "DELETE" : type === "graphql" ? "POST" : "GET");
    if (type === "graphql") assert.equal(restored.body.type, "graphql");
    if (type === "websocket") assert.match(restored.url, /^wss?:\/\//);
    if (type === "sse") assert.equal(restored.headers.Accept, "text/event-stream");
  }
});

test("reconoce conexiones existentes por URL y cabeceras", () => {
  const request = { ...newTypedRequest("Ejemplo", "http"), type: undefined };
  assert.equal(requestType({ ...request, url: "ws://localhost:9000" }), "websocket");
  assert.equal(requestType({ ...request, headers: { accept: "application/json, text/event-stream; charset=utf-8" } }), "sse");
});


test("el tipo explícito prevalece sobre la URL y las cabeceras", () => {
  const request = newTypedRequest("Socket", "websocket");
  request.url = "{{socket_url}}";
  assert.equal(requestType(normalizeRequest(request)), "websocket");
  assert.equal(requestType({ ...request, type: "http", url: "ws://localhost" }), "http");
});


test("las cabeceras deshabilitadas conservan su valor sin pasar al envío", () => {
  const request = normalizeRequest({ name: "Headers", url: "http://localhost", headers: { "X-Debug": "{{missing}}", Accept: "application/json" }, disabled_headers: ["x-debug"] });
  assert.deepEqual(enabledHeaders(request), { Accept: "application/json" });
  assert.equal(request.headers["X-Debug"], "{{missing}}");
});
