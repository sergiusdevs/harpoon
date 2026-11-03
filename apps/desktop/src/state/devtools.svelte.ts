// Estado del panel inferior (consola + red). Los datos llegan de Rust en
// lotes; aquí solo se guardan con `$state.raw` (sin proxys profundos) y se
// recortan al mismo máximo que el backend.

import { devtoolsBridge, type LogEntry, type LogLevel, type NetEntry, type Batch } from "../services/devtools";

export type { LogEntry, NetEntry, LogLevel };
export type DevtoolsTab = "console" | "network";
export type LogFilter = "all" | LogLevel;
export type NetFilter = "all" | "2xx" | "3xx" | "4xx" | "5xx" | "error";

const MAX = 2000;
const read = (key: string) => { try { return localStorage.getItem(key); } catch { return null; } };
const write = (key: string, value: string) => { try { localStorage.setItem(key, value); } catch { /* sin almacenamiento */ } };

class Devtools {
  open = $state(read("harpoon.devtools.open") === "1");
  tab = $state<DevtoolsTab>(read("harpoon.devtools.tab") === "network" ? "network" : "console");
  height = $state(Number(read("harpoon.devtools.height")) || 260);
  logs = $state.raw<LogEntry[]>([]);
  net = $state.raw<NetEntry[]>([]);
  logFilter = $state<LogFilter>("all");
  netFilter = $state<NetFilter>("all");
  /** Errores llegados con el panel cerrado (para el punto del botón). */
  unseenErrors = $state(0);
  #started = false;

  async start() {
    if (this.#started) return;
    this.#started = true;
    const bridge = await devtoolsBridge;
    await bridge.listen((batch) => this.#append(batch));
    const snap = await bridge.snapshot();
    this.logs = snap.logs.slice(-MAX);
    this.net = snap.net.slice(-MAX);
  }

  #append(batch: Batch) {
    if (batch.logs.length) this.logs = this.logs.length + batch.logs.length > MAX ? [...this.logs, ...batch.logs].slice(-MAX) : [...this.logs, ...batch.logs];
    if (batch.net.length) this.net = this.net.length + batch.net.length > MAX ? [...this.net, ...batch.net].slice(-MAX) : [...this.net, ...batch.net];
    if (!this.open) this.unseenErrors += batch.logs.filter((l) => l.level === "error").length;
  }

  toggle(tab?: DevtoolsTab) {
    if (tab && this.open && this.tab !== tab) { this.setTab(tab); return; }
    this.open = !this.open;
    if (tab) this.setTab(tab);
    if (this.open) this.unseenErrors = 0;
    write("harpoon.devtools.open", this.open ? "1" : "0");
  }
  close() { this.open = false; write("harpoon.devtools.open", "0"); }
  setTab(tab: DevtoolsTab) { this.tab = tab; write("harpoon.devtools.tab", tab); }
  resize(delta: number) {
    this.height = Math.round(Math.min(window.innerHeight - 160, Math.max(120, this.height - delta)));
    write("harpoon.devtools.height", String(this.height));
  }
  async clear() {
    const kind = this.tab === "console" ? "logs" : "net";
    if (kind === "logs") this.logs = []; else this.net = [];
    await (await devtoolsBridge).clear(kind);
  }
}

export const devtools = new Devtools();
