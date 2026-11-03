import test from "node:test";
import assert from "node:assert/strict";
import { loadTypescript } from "./load-typescript.mjs";
const { StreamSessions } = await loadTypescript("../src/domain/streamSessions.ts");

function setup(start) {
  const events = new Map(), stops = [], sends = [];
  let sequence = 0;
  const bridge = {
    startStream: async (...args) => { events.set(args[5], args[6]); if (start) await start(); },
    stopStream: async (id) => { stops.push(id); },
    sendStream: async (id, message) => { sends.push([id, message]); },
  };
  const manager = new StreamSessions(Promise.resolve(bridge), () => {}, () => `session-${++sequence}`);
  const request = { name: "Socket", type: "websocket", method: "GET", url: "{{socket_url}}", headers: {} };
  return { manager, request, events, stops, sends };
}
const event = (kind, data = "") => ({ kind, data, event: "", id: "" });

test("navegar a otra solicitud conserva conexiones y mensajes", async () => {
  const { manager, request, events, stops } = setup();
  const first = manager.open("/one", "socket", "dev", request, "websocket");
  await manager.connect(first);
  events.get(first.id)(event("open"));
  manager.open("/two", "socket", "prod", request, "websocket");
  events.get(first.id)(event("message", "hola"));
  manager.select(first.identity);
  assert.equal(first.state, "open");
  assert.equal(first.entries[0].data, "hola");
  assert.equal(manager.sessions.size, 2);
  assert.deepEqual(stops, []);
});

test("desconectar durante la apertura cancela también tras registrar la sesión", async () => {
  let release;
  const pending = new Promise((resolve) => { release = resolve; });
  const { manager, request, stops, events } = setup(() => pending);
  const session = manager.open("/one", "socket", null, request, "websocket");
  const opening = manager.connect(session);
  await Promise.resolve();
  await manager.disconnect(session);
  assert.equal(session.state, "closing");
  release(); await opening;
  assert.equal(stops.length, 2);
  events.get(session.id)(event("closed"));
  assert.equal(session.state, "closed");
});

test("los eventos antiguos no cierran una conexión nueva", async () => {
  const { manager, request, events } = setup();
  const session = manager.open("/one", "socket", null, request, "websocket");
  await manager.connect(session);
  const old = events.get(session.id);
  old(event("closed"));
  await manager.connect(session);
  events.get(session.id)(event("open"));
  old(event("error", "error antiguo")); old(event("closed"));
  assert.equal(session.state, "open"); assert.equal(session.error, "");
});

test("conectar dos veces no duplica sesión y enviar usa su propio identificador", async () => {
  const { manager, request, events, sends } = setup();
  const session = manager.open("/one", "socket", null, request, "websocket");
  await Promise.all([manager.connect(session), manager.connect(session)]);
  assert.equal(events.size, 1);
  events.get(session.id)(event("open"));
  manager.update(session, { message: "ping" });
  await manager.send(session);
  assert.deepEqual(sends, [[session.id, "ping"]]); assert.equal(session.message, "");
});

test("reabrir una sesión activa conserva su configuración original", async () => {
  const { manager, request, events } = setup();
  const session = manager.open("/one", "socket", "dev", request, "websocket");
  await manager.connect(session); events.get(session.id)(event("open"));
  manager.open("/one", "socket", "prod", { ...request, url: "ws://otra-url" }, "websocket");
  assert.equal(session.env, "dev"); assert.equal(session.request.url, "{{socket_url}}");
});

test("los errores de apertura permiten volver a conectar", async () => {
  const manager = new StreamSessions(Promise.resolve({ startStream: async () => { throw new Error("fallo"); } }), () => {});
  const session = manager.open("/one", "socket", null, { url: "ws://localhost" }, "websocket");
  await manager.connect(session);
  assert.equal(session.state, "closed"); assert.equal(session.id, null); assert.match(session.error, /fallo/);
});

test("SSE reconecta, limita el historial y conserva el último evento", async () => {
  const { manager, request, events } = setup();
  const session = manager.open("/one", "events", null, { ...request, type: "sse" }, "sse");
  await manager.connect(session);
  const append = events.get(session.id);
  append(event("reconnecting", "reintentando"));
  assert.equal(session.notice, "reintentando");
  append(event("open")); assert.equal(session.notice, "");
  for (let i = 0; i < 205; i++) append(event("message", String(i)));
  assert.equal(session.entries.length, 200); assert.equal(session.entries.at(-1).data, "204");
});
