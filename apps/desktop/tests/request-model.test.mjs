import test from "node:test";
import assert from "node:assert/strict";
import { loadTypescript } from "./load-typescript.mjs";

const { normalizeRequest, normalizeCollection, normalizeAuth } = await loadTypescript("../src/domain/types.ts");

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
  assert.deepEqual(collection.environments.dev, { vars: {}, secrets: [] });
  assert.equal(collection.requests.health.method, "GET");
  assert.equal(collection.meta.settings.timeout_ms, 30000);
});

test("completa los campos opcionales de autenticación", () => {
  assert.deepEqual(normalizeAuth({ type: "api_key", key: "x", value: "y" }), { type: "api_key", key: "x", value: "y", in: "header" });
  assert.deepEqual(normalizeAuth({ type: "oauth2", client_id: "demo" }).extra, {});
});
