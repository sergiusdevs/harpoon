import test from "node:test";
import assert from "node:assert/strict";
import { loadTypescript } from "./load-typescript.mjs";

const { extractLiteral, applySecret } = await loadTypescript("../src/domain/secrets.ts");

test("respeta el esquema y otras variables", () => {
  assert.deepEqual(extractLiteral("Bearer abc123xyz", "token"), { next: "Bearer {{token}}", secret: "abc123xyz" });
  assert.deepEqual(extractLiteral("{{prefix}}-abcdef123", "k"), { next: "{{prefix}}{{k}}", secret: "-abcdef123" });
  assert.equal(extractLiteral("{{token}}", "t"), null);
});

test("convierte headers, parámetros, auth y la contraseña de la URL", () => {
  const base = { name: "x", method: "GET", url: "https://ana:hunter2@api.x.com/a?b=1", headers: { Authorization: "Bearer s3cr3t-value", Accept: "application/json" }, query: { api_key: "zzzzzzzz" }, body: { type: "none" }, auth: { type: "bearer", token: "tok-123456" } };
  const h = applySecret(base, "header Authorization", "auth_header");
  assert.equal(h.value, "s3cr3t-value");
  assert.equal(h.request.headers.Authorization, "Bearer {{auth_header}}");
  assert.equal(base.headers.Authorization, "Bearer s3cr3t-value", "no muta el original");
  assert.equal(applySecret(base, "parámetro api_key", "api_key").request.query.api_key, "{{api_key}}");
  const a = applySecret(base, "auth.token", "token");
  assert.deepEqual([a.value, a.request.auth.token], ["tok-123456", "{{token}}"]);
  const u = applySecret(base, "url (usuario:contraseña)", "password");
  assert.deepEqual([u.value, u.request.url], ["hunter2", "https://ana:{{password}}@api.x.com/a?b=1"]);
  assert.equal(applySecret(base, "cuerpo", "x"), null);
});

const helpers = await loadTypescript("../src/domain/secrets.ts");

test("isSensitiveName replica el criterio de Rust", () => {
  for (const n of ["Authorization", "Proxy-Authorization", "Cookie", "auth", "pwd", "pass", "X-API-Key", "api_key", "access_token", "client-secret", "X-Session-Id", "privateKey", "AWS-Signature", "db_password"]) assert.equal(helpers.isSensitiveName(n), true, n);
  for (const n of ["Accept", "Content-Type", "page", "author", "passport", "user", ""]) assert.equal(helpers.isSensitiveName(n), false, n);
});

test("isLiteral ignora variables y el esquema", () => {
  assert.equal(helpers.isLiteral("Bearer {{token}}"), false);
  assert.equal(helpers.isLiteral("{{api_key}}"), false);
  assert.equal(helpers.isLiteral("Bearer abc"), false);
  assert.equal(helpers.isLiteral("Bearer abcdef123456"), true);
  assert.equal(helpers.isLiteral("Basic {{creds}}"), false);
  assert.equal(helpers.isLiteral("hunter2"), true);
  assert.equal(helpers.isLiteral(""), false);
});

test("suggestSecretName, folderOf e isConvertibleLocation", () => {
  assert.equal(helpers.suggestSecretName("header Authorization"), "token");
  assert.equal(helpers.suggestSecretName("header X-API-Key"), "x_api_key");
  assert.equal(helpers.suggestSecretName("parámetro apiKey"), "api_key");
  assert.equal(helpers.suggestSecretName("auth.value"), "api_key");
  assert.equal(helpers.suggestSecretName("auth.client_secret"), "client_secret");
  assert.equal(helpers.folderOf("users/list"), "users");
  assert.equal(helpers.folderOf("a/b/c"), "a/b");
  assert.equal(helpers.folderOf("root"), "");
  assert.equal(helpers.isConvertibleLocation("header Authorization"), true);
  assert.equal(helpers.isConvertibleLocation("url (usuario:contraseña)"), true);
  assert.equal(helpers.isConvertibleLocation("url"), false);
  assert.equal(helpers.isConvertibleLocation("cuerpo"), false);
  assert.equal(helpers.isConvertibleLocation("script pre"), false);
  assert.equal(helpers.isValidVarName("api_key"), true);
  assert.equal(helpers.isValidVarName("a b"), false);
});
