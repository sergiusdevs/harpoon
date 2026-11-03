import { createServer } from "node:http";
import { createHash } from "node:crypto";
import { pathToFileURL } from "node:url";

// Backend de pruebas sin dependencias. Solo escucha en loopback.
export async function startBackend(port = 0) {
  const users = new Map([[1, { id: 1, name: "Ada" }]]);
  const hits = new Map();
  let nextId = 2;
  const server = createServer(async (req, res) => {
    const url = new URL(req.url, "http://localhost");
    hits.set(url.pathname, (hits.get(url.pathname) ?? 0) + 1);
    const json = (status, value, headers = {}) => { res.writeHead(status, { "Content-Type": "application/json", ...headers }); res.end(JSON.stringify(value)); };
    try {
      const chunks = [];
      for await (const chunk of req) {
        chunks.push(chunk);
        if (chunks.reduce((size, item) => size + item.length, 0) > 1024 * 1024) { json(413, { error: "body too large" }); return; }
      }
      const text = Buffer.concat(chunks).toString();
      const body = text && req.headers["content-type"]?.includes("application/json") ? JSON.parse(text) : text;
      if (url.pathname === "/health") return json(200, { status: "UP" });
      if (url.pathname === "/echo") return json(200, { method: req.method, query: Object.fromEntries(url.searchParams), headers: req.headers, body });
      if (url.pathname === "/login") return json(200, { token: "local-test-token" }, { "Set-Cookie": "SESSION=local-test-session; HttpOnly; Path=/; SameSite=Lax" });
      if (url.pathname === "/private") return req.headers.authorization === "Bearer local-test-token" ? json(200, { authenticated: true }) : json(401, { error: "Unauthorized" });
      if (url.pathname === "/session") return req.headers.cookie?.includes("SESSION=local-test-session") ? json(200, { authenticated: true }) : json(401, { error: "No session" });
      if (url.pathname === "/basic") return req.headers.authorization === `Basic ${Buffer.from("demo:secret").toString("base64")}` ? json(200, { authenticated: true }) : json(401, { error: "Unauthorized" });
      if (url.pathname === "/oauth/token") return json(200, { access_token: "local-test-token", token_type: "Bearer", expires_in: 3600 });
      if (url.pathname === "/redirect") { res.writeHead(302, { Location: "/health" }); res.end(); return; }
      if (url.pathname === "/slow") { const timer = setTimeout(() => json(200, { slow: true }), 400); res.on("close", () => clearTimeout(timer)); return; }
      if (url.pathname === "/users" && req.method === "GET") return json(200, [...users.values()]);
      if (url.pathname === "/users" && req.method === "POST") {
        if (typeof body !== "object" || !body.name?.trim()) return json(400, { status: 400, title: "Validation failed", errors: [{ field: "name", message: "must not be blank" }] });
        const user = { id: nextId++, name: body.name }; users.set(user.id, user); return json(201, user, { Location: `/users/${user.id}` });
      }
      if (/^\/users\/\d+$/.test(url.pathname)) {
        const id = Number(url.pathname.split("/").pop());
        if (!users.has(id)) return json(404, { error: "Not found" });
        if (req.method === "DELETE") { users.delete(id); res.writeHead(204); res.end(); return; }
        if (["PUT", "PATCH"].includes(req.method)) users.set(id, { id, name: body.name ?? users.get(id).name });
        return json(200, users.get(id));
      }
      // Mock acotado para comprobar el transporte GraphQL (sin motor de esquemas).
      if (url.pathname === "/graphql") {
        if (req.method !== "POST" || typeof body !== "object" || !body.query) return json(400, { errors: [{ message: "Missing query" }] });
        if (body.query.includes("unknownField")) return json(200, { errors: [{ message: "Unknown field" }] });
        return json(200, { data: { user: users.get(Number(body.variables?.id ?? 1)) ?? null }, operationName: body.operationName ?? null });
      }
      if (url.pathname === "/events") {
        res.writeHead(200, { "Content-Type": "text/event-stream", "Cache-Control": "no-cache" });
        let id = Number(req.headers["last-event-id"] ?? 0);
        const emit = () => { id++; res.write(`id: ${id}\nevent: tick\ndata: ${JSON.stringify({ tick: id })}\n\n`); if (url.searchParams.has("once")) res.end(); };
        emit();
        if (!url.searchParams.has("once")) { const timer = setInterval(emit, 50); res.on("close", () => clearInterval(timer)); }
        return;
      }
      return json(404, { error: "Not found" });
    } catch { json(400, { error: "Malformed request" }); }
  });
  server.on("upgrade", (req, socket, head) => {
    if (req.url !== "/socket" || !req.headers["sec-websocket-key"]) { socket.end("HTTP/1.1 400 Bad Request\r\n\r\n"); return; }
    const accept = createHash("sha1").update(req.headers["sec-websocket-key"] + "258EAFA5-E914-47DA-95CA-C5AB0DC85B11").digest("base64");
    socket.write(`HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: ${accept}\r\n\r\n`);
    let pending = head;
    const consume = () => {
      while (pending.length >= 2) {
        const opcode = pending[0] & 15, masked = Boolean(pending[1] & 128);
        let size = pending[1] & 127, offset = 2;
        if (size === 126) { if (pending.length < 4) return; size = pending.readUInt16BE(2); offset = 4; }
        if (size === 127 || !masked) { socket.destroy(); return; }
        if (pending.length < offset + 4 + size) return;
        const mask = pending.subarray(offset, offset + 4); offset += 4;
        const payload = Buffer.from(pending.subarray(offset, offset + size));
        for (let i = 0; i < size; i++) payload[i] ^= mask[i % 4];
        pending = pending.subarray(offset + size);
        if (opcode === 8) { socket.end(Buffer.from([0x88, 0])); return; }
        if (opcode !== 1 && opcode !== 9) continue;
        const header = size < 126 ? Buffer.from([opcode === 9 ? 0x8a : 0x81, size]) : Buffer.from([opcode === 9 ? 0x8a : 0x81, 126, size >> 8, size & 255]);
        socket.write(Buffer.concat([header, payload]));
      }
    };
    socket.on("error", () => {});
    socket.on("data", (chunk) => { pending = Buffer.concat([pending, chunk]); consume(); });
    consume();
  });
  const sockets = new Set();
  server.on("connection", (socket) => { sockets.add(socket); socket.on("close", () => sockets.delete(socket)); });
  await new Promise((resolve) => server.listen(port, "127.0.0.1", resolve));
  const address = `http://127.0.0.1:${server.address().port}`;
  return { address, hits, close: async () => { for (const socket of sockets) socket.destroy(); await new Promise((resolve) => server.close(resolve)); } };
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const backend = await startBackend(Number(process.env.PORT ?? 8788));
  console.log(`Backend de pruebas: ${backend.address}\nWebSocket: ${backend.address.replace("http", "ws")}/socket\nSSE: ${backend.address}/events`);
  for (const signal of ["SIGINT", "SIGTERM"]) process.on(signal, () => { void backend.close().then(() => process.exit(0)); });
}
