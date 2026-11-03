// Puente de la consola/red. El registro vive en Rust (src-tauri/src/devtools.rs):
// aquí solo se pide el estado inicial y se escuchan las entradas nuevas.
// En modo demo (navegador) se simula envolviendo `sendRequest` del mock.

import { api, isMock } from "./api";
import type { Exchange } from "../domain/types";

export type LogLevel = "log" | "info" | "warn" | "error" | "debug";
export interface LogEntry { id: number; at: number; level: LogLevel; message: string; source: string }
export interface NetEntry {
  id: number; at: number; key: string; method: string; domain: string; path: string;
  status: number | null; duration_ms: number | null; size: number | null; error: string | null;
}
export interface Batch { logs: LogEntry[]; net: NetEntry[] }

type Listener = (batch: Batch) => void;

export interface DevtoolsBridge {
  snapshot(): Promise<Batch>;
  clear(kind: "logs" | "net" | "all"): Promise<void>;
  listen(fn: Listener): Promise<() => void>;
}

async function tauriBridge(): Promise<DevtoolsBridge> {
  const { invoke } = await import("@tauri-apps/api/core");
  const { listen } = await import("@tauri-apps/api/event");
  return {
    snapshot: () => invoke<Batch>("devtools_snapshot"),
    clear: (kind) => invoke("devtools_clear", { kind }),
    listen: (fn) => listen<Batch>("devtools://entries", (event) => fn(event.payload)),
  };
}

/** Misma lógica que `split_level`/`split_url` de Rust, solo para el modo demo. */
function splitLevel(line: string): [LogLevel, string] {
  const m = /^\[(info|warn|error|debug)\] /.exec(line);
  return m ? [m[1] as LogLevel, line.slice(m[0].length)] : ["log", line];
}
function splitUrl(url: string, query: Record<string, string> = {}): [string, string] {
  const rest = url.includes("://") ? url.slice(url.indexOf("://") + 3) : url;
  const i = rest.search(/[/?#]/);
  const domain = i < 0 ? rest : rest.slice(0, i);
  let path = i < 0 ? "/" : rest.slice(i);
  if (path.startsWith("?")) path = "/" + path;
  const q = Object.entries(query).map(([k, v]) => (v ? `${k}=${v}` : k)).join("&");
  if (q) path += (path.includes("?") ? "&" : "?") + q;
  return [domain.split("@").pop() ?? domain, path];
}

async function mockBridge(): Promise<DevtoolsBridge> {
  const listeners = new Set<Listener>();
  let seq = 0;
  const state: Batch = { logs: [], net: [] };
  const emit = (batch: Batch) => {
    state.logs.push(...batch.logs); state.net.push(...batch.net);
    for (const fn of listeners) fn(batch);
  };
  const bridge = await api;
  const send = bridge.sendRequest.bind(bridge);
  bridge.sendRequest = async (root, env, request, key, id) => {
    const at = Date.now();
    try {
      const x: Exchange = await send(root, env, request, key, id);
      const [domain, path] = splitUrl(x.sent.url, x.sent.query);
      emit({
        logs: x.logs.map((line) => { const [level, message] = splitLevel(line); return { id: ++seq, at, level, message, source: key }; }),
        net: [{ id: ++seq, at, key, method: x.sent.method, domain, path, status: x.response.status, duration_ms: x.response.elapsed_ms, size: x.response.size, error: null }],
      });
      return x;
    } catch (cause) {
      const [domain, path] = splitUrl(request.url);
      const error = String((cause as { detail?: string })?.detail ?? cause);
      emit({ logs: [{ id: ++seq, at, level: "error", message: error, source: key }], net: [{ id: ++seq, at, key, method: request.method, domain, path, status: null, duration_ms: null, size: null, error }] });
      throw cause;
    }
  };
  return {
    snapshot: async () => ({ logs: [...state.logs], net: [...state.net] }),
    clear: async (kind) => { if (kind !== "net") state.logs = []; if (kind !== "logs") state.net = []; },
    listen: async (fn) => { listeners.add(fn); return () => listeners.delete(fn); },
  };
}

export const devtoolsBridge: Promise<DevtoolsBridge> = isMock ? mockBridge() : tauriBridge();
