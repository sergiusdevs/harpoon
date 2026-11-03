import test, { before, after } from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, mkdir, writeFile, rm, readFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { execFile } from "node:child_process";
import { promisify } from "node:util";
import { startBackend } from "./test-backend.mjs";
const runFile = promisify(execFile);
const binary = process.env.HARPOON_BIN ?? fileURLToPath(new URL("../target/debug/harpoon", import.meta.url));
let backend;
const directories = [];
before(async () => { backend = await startBackend(); });
after(async () => { await backend?.close(); await Promise.all(directories.map((directory) => rm(directory, { recursive: true, force: true }))); });
async function collection() {
  const root = await mkdtemp(join(tmpdir(), "harpoon-integration-")); directories.push(root);
  await mkdir(join(root, "requests")); await mkdir(join(root, "environments"));
  await writeFile(join(root, "harpoon.toml"), 'name = "Local test"\nformat = 1\ndefault_environment = "dev"\n');
  await writeFile(join(root, "environments/dev.toml"), `[vars]\nbase_url = "${backend.address}"\nsocket_url = "${backend.address.replace("http", "ws")}/socket"\n`);
  return root;
}
async function request(root, key, path, extra = "", method = "GET", type = "http") {
  await writeFile(join(root, "requests", `${key}.toml`), `name = "${key}"\ntype = "${type}"\nmethod = "${method}"\nurl = "${path.startsWith("{{") ? path : "{{base_url}}" + path}"\n${extra}\n`);
}
async function cli(root, ...args) {
  try { const result = await runFile(binary, ["-C", root, ...args], { timeout: 15000 }); return { ...result, code: 0 }; }
  catch (error) { return { stdout: error.stdout ?? "", stderr: error.stderr ?? "", code: error.code }; }
}
async function send(root, key, status = 200) {
  const result = await cli(root, "send", key, "--json");
  assert.equal(result.code, 0, result.stderr);
  const json = JSON.parse(result.stdout); assert.equal(json.response.status, status); return json;
}

test("HTTP: GET, POST, PUT, PATCH, DELETE, HEAD y OPTIONS", async () => {
  const root = await collection();
  for (const method of ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"]) {
    await request(root, method.toLowerCase(), "/echo?source=harpoon", '[headers]\nX-Test = "local"', method);
    const result = await send(root, method.toLowerCase());
    if (method !== "HEAD") { assert.equal(result.response.body.method, method); assert.equal(result.response.body.query.source, "harpoon"); assert.equal(result.response.body.headers["x-test"], "local"); }
  }
});

test("HTTP: CRUD JSON, validación 400 y recurso inexistente 404", async () => {
  const root = await collection();
  await request(root, "create", "/users", '[body]\ntype = "json"\njson = \'{"name":"Grace"}\'\n[expect]\nstatus = 201', "POST");
  const created = await send(root, "create", 201); const id = created.response.body.id;
  await request(root, "update", `/users/${id}`, '[body]\ntype = "json"\njson = \'{"name":"Grace Hopper"}\'', "PUT");
  assert.equal((await send(root, "update")).response.body.name, "Grace Hopper");
  await request(root, "delete", `/users/${id}`, "", "DELETE"); await send(root, "delete", 204);
  await request(root, "missing", `/users/${id}`, '[expect]\nstatus = 404'); await send(root, "missing", 404);
  await request(root, "invalid", "/users", '[body]\ntype = "json"\njson = \'{"name":""}\'\n[expect]\nstatus = 400', "POST");
  assert.equal((await send(root, "invalid", 400)).response.body.errors[0].field, "name");
});

test("HTTP: login, variables de ejecución, Bearer y cookies entre procesos", async () => {
  const root = await collection();
  await request(root, "login", "/login", '[scripts]\npost = \'harpoon.vars.set("token", harpoon.res.json().token);\'', "POST");
  await send(root, "login");
  await request(root, "private", "/private", '[auth]\ntype = "bearer"\ntoken = "{{token}}"');
  assert.equal((await send(root, "private")).response.body.authenticated, true);
  await request(root, "session", "/session"); assert.equal((await send(root, "session")).response.body.authenticated, true);
});

test("HTTP: Basic y OAuth2 client credentials", async () => {
  const root = await collection();
  await request(root, "basic", "/basic", '[auth]\ntype = "basic"\nusername = "demo"\npassword = "secret"'); await send(root, "basic");
  await request(root, "oauth", "/private", '[auth]\ntype = "oauth2"\ngrant = "client_credentials"\ntoken_url = "{{base_url}}/oauth/token"\nclient_id = "demo"\nclient_secret = "secret"'); await send(root, "oauth");
});

test("HTTP: formulario y archivo multipart", async () => {
  const root = await collection();
  await request(root, "form", "/echo", '[body]\ntype = "form"\n[body.fields]\nname = "Ada Lovelace"', "POST");
  assert.match((await send(root, "form")).response.body.body, /name=Ada\+Lovelace/);
  await writeFile(join(root, "sample.txt"), "archivo local");
  await request(root, "multipart", "/echo", '[body]\ntype = "multipart"\n[[body.parts]]\ntype = "file"\nname = "upload"\npath = "sample.txt"', "POST");
  assert.match((await send(root, "multipart")).response.body.body, /archivo local/);
});

test("GraphQL: consulta, variables y nombre de operación", async () => {
  const root = await collection();
  await request(root, "graphql", "/graphql", '[body]\ntype = "graphql"\nquery = "query FindUser($id: ID!) { user(id: $id) { id name } }"\nvariables = \'{"id":1}\'\noperation_name = "FindUser"', "POST", "graphql");
  const result = await send(root, "graphql"); assert.equal(result.response.body.data.user.name, "Ada"); assert.equal(result.response.body.operationName, "FindUser");
});

test("HTTP: redirección, timeout y aserciones fallidas", async () => {
  const root = await collection();
  await request(root, "redirect", "/redirect"); assert.equal((await send(root, "redirect")).response.body.status, "UP");
  await request(root, "fail", "/health", '[expect]\nstatus = 201');
  assert.equal((await cli(root, "send", "fail", "--json")).code, 1);
  await writeFile(join(root, "harpoon.toml"), 'name = "Local test"\nformat = 1\ndefault_environment = "dev"\n[settings]\ntimeout_ms = 50\n');
  await request(root, "slow", "/slow");
  const timeout = await cli(root, "send", "slow"); assert.notEqual(timeout.code, 0); assert.match(timeout.stderr, /tiempo|timeout|agotado/i);
});

test("runner: omite WS/SSE explícitos y antiguos sin enviar tráfico", async () => {
  const root = await collection();
  await request(root, "health", "/health", '[expect]\nstatus = 200');
  await request(root, "ws", "{{socket_url}}", "", "GET", "websocket");
  await request(root, "sse", "/events", "", "GET", "sse");
  await writeFile(join(root, "requests/legacy.toml"), `name = "Antigua"\nurl = "${backend.address.replace("http", "ws")}/socket"\n`);
  const listed = await cli(root, "list");
  assert.equal(listed.code, 0, listed.stderr);
  assert.match(listed.stdout, /ws\s+WS\s+/); assert.match(listed.stdout, /sse\s+SSE\s+/);
  const previous = backend.hits.get("/events") ?? 0;
  const result = await cli(root, "run", "--json", "--junit", join(root, "report.xml"));
  assert.equal(result.code, 0, result.stderr);
  const report = JSON.parse(result.stdout); assert.equal(report.total, 1); assert.equal(report.failed, 0);
  assert.match(result.stderr, /Omitida ws/); assert.match(result.stderr, /Omitida sse/); assert.match(result.stderr, /Omitida legacy/);
  assert.equal(backend.hits.get("/events") ?? 0, previous);
  assert.match(await readFile(join(root, "report.xml"), "utf8"), /tests="1"/);
  for (const key of ["ws", "sse", "legacy"]) {
    const direct = await cli(root, "send", key); assert.notEqual(direct.code, 0); assert.match(direct.stderr, /interactiva/);
  }
  const onlyStreams = await cli(root, "run", "ws", "--json"); assert.notEqual(onlyStreams.code, 0); assert.match(onlyStreams.stderr, /no hay solicitudes HTTP/);
});

test("backend: WebSocket real hace eco de mensajes", async () => {
  const socket = new WebSocket(backend.address.replace("http", "ws") + "/socket");
  await new Promise((resolve, reject) => { socket.addEventListener("open", resolve, { once: true }); socket.addEventListener("error", reject, { once: true }); });
  const received = new Promise((resolve) => socket.addEventListener("message", (event) => resolve(event.data), { once: true }));
  socket.send("hola Harpoon"); assert.equal(await received, "hola Harpoon");
  const closed = new Promise((resolve) => socket.addEventListener("close", resolve, { once: true })); socket.close(); await closed;
});

test("backend: SSE entrega eventos e interpreta Last-Event-ID", async () => {
  const response = await fetch(backend.address + "/events?once", { headers: { "Last-Event-ID": "7" } });
  assert.match(response.headers.get("content-type"), /text\/event-stream/);
  const data = await response.text(); assert.match(data, /id: 8/); assert.match(data, /event: tick/); assert.match(data, /"tick":8/);
});


test("HTTP: cabeceras deshabilitadas no se resuelven ni se envían", async () => {
  const root = await collection();
  await request(root, "headers", "/echo", 'disabled_headers = ["x-debug"]\n[headers]\nX-Debug = "{{undefined_variable}}"\nX-Enabled = "on"\n[header_descriptions]\nX-Debug = "Temporalmente desactivada"');
  const body = (await send(root, "headers")).response.body;
  assert.equal(body.headers["x-debug"], undefined); assert.equal(body.headers["x-enabled"], "on");
  assert.match(await readFile(join(root, "requests/headers.toml"), "utf8"), /undefined_variable/);
});
