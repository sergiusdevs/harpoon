import type { Request } from "./types";

export type Protocol = "websocket" | "sse";
export type StreamEvent = { kind: "open" | "closed" | "error" | "message" | "sent" | "reconnecting"; data: string; event: string; id: string };
export type StreamEntry = StreamEvent & { key: number; time: string };
export type StreamSession = {
  identity: string; root: string; key: string; env: string | null; request: Request; protocol: Protocol;
  state: "closed" | "connecting" | "open" | "closing"; id: string | null;
  urlEdited: boolean; message: string; error: string; notice: string; entries: StreamEntry[]; sequence: number;
};
type Bridge = {
  startStream(root: string, key: string, env: string | null, request: Request, protocol: Protocol, id: string, onEvent: (event: StreamEvent) => void): Promise<void>;
  stopStream(id: string): Promise<void>;
  sendStream(id: string, data: string): Promise<void>;
};

/** Las sesiones pertenecen a la app, no al componente que muestra los mensajes. */
export class StreamSessions {
  readonly sessions = new Map<string, StreamSession>();
  active: string | null = null;
  constructor(private bridge: Promise<Bridge>, private changed: () => void, private uuid: () => string = () => crypto.randomUUID(), private reactive: (session: StreamSession) => StreamSession = (session) => session) {}

  open(root: string, key: string, env: string | null, request: Request, protocol: Protocol) {
    const identity = JSON.stringify([root, key]);
    let session = this.sessions.get(identity);
    if (!session) {
      session = this.reactive({ identity, root, key, env, request: JSON.parse(JSON.stringify(request)) as Request, protocol, state: "closed", id: null, urlEdited: false, message: "", error: "", notice: "", entries: [], sequence: 0 });
      this.sessions.set(identity, session);
    } else if (session.state === "closed") {
      session.request = JSON.parse(JSON.stringify(request)) as Request;
      session.urlEdited = false;
      session.env = env;
      session.protocol = protocol;
    }
    this.active = identity;
    this.changed();
    return session;
  }

  select(identity: string) {
    if (this.sessions.has(identity)) { this.active = identity; this.changed(); }
  }

  update(session: StreamSession, patch: Partial<Pick<StreamSession, "message" | "request" | "entries" | "urlEdited">>) {
    Object.assign(session, patch);
    this.changed();
  }

  private append(session: StreamSession, id: string, event: StreamEvent) {
    if (session.id !== id) return; // Los eventos tardíos no afectan a una reconexión posterior.
    if (event.kind === "closed") { session.state = "closed"; session.id = null; }
    else if (event.kind === "error") session.error = event.data;
    else if (event.kind === "open" && session.state !== "closing") { session.state = "open"; session.notice = ""; }
    else if (event.kind === "reconnecting" && session.state !== "closing") { session.state = "connecting"; session.notice = event.data; }
    else if (event.kind === "message" || event.kind === "sent") {
      const data = event.data.length > 20000 ? event.data.slice(0, 20000) + "\n… contenido recortado en la vista" : event.data;
      session.entries = [...session.entries.slice(-199), { ...event, data, key: session.sequence++, time: new Date().toLocaleTimeString() }];
    }
    this.changed();
  }

  async connect(session: StreamSession) {
    if (session.state !== "closed") return;
    const id = this.uuid();
    session.id = id; session.state = "connecting"; session.error = ""; session.notice = "";
    const request = JSON.parse(JSON.stringify(session.request)) as Request;
    this.changed();
    try {
      const bridge = await this.bridge;
      await bridge.startStream(session.root, session.key, session.env, request, session.protocol, id, (event) => this.append(session, id, event));
      // Puede pedirse desconectar mientras startStream todavía prepara la conexión.
      if (session.id === id && this.sessions.get(session.identity)?.state === "closing") await bridge.stopStream(id);
    } catch (cause) {
      if (session.id === id) { session.error = String(cause); session.state = "closed"; session.id = null; this.changed(); }
    }
  }

  async disconnect(session: StreamSession) {
    if (!session.id || session.state === "closing") return;
    const previous = session.state;
    session.state = "closing";
    this.changed();
    try { await (await this.bridge).stopStream(session.id); }
    catch (cause) { session.error = String(cause); if (session.id) session.state = previous; this.changed(); }
  }

  async send(session: StreamSession) {
    if (!session.id || session.state !== "open" || !session.message) return;
    const message = session.message;
    try {
      await (await this.bridge).sendStream(session.id, message);
      if (session.message === message) session.message = "";
    } catch (cause) { session.error = String(cause); }
    this.changed();
  }
}
