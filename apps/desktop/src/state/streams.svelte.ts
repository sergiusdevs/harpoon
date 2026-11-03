import { api } from "../services/api";
import { StreamSessions, type StreamSession } from "../domain/streamSessions";

function reactiveSession(value: StreamSession) {
  let session = $state(value);
  return session;
}

class Streams {
  private revision = $state(0);
  readonly manager = new StreamSessions(api, () => { this.revision++; }, undefined, reactiveSession);
  get sessions() { void this.revision; return [...this.manager.sessions.values()]; }
  get current() { void this.revision; return this.manager.active ? this.manager.sessions.get(this.manager.active) ?? null : null; }
  get activeCount() { return this.sessions.filter((session) => session.state !== "closed").length; }
}

export const streams = new Streams();
