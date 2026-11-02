// Estado global de la app con runes de Svelte 5.

import { api } from "./api";
import type { Collection, Exchange, Request } from "./types";

export type Status = { kind: "idle" | "ok" | "err" | "busy"; text: string };

class Store {
  collection = $state<Collection | null>(null);
  selected = $state<string | null>(null);
  env = $state<string | null>(null);
  /** Copia editable de la solicitud seleccionada. */
  draft = $state<Request | null>(null);
  dirty = $state(false);
  exchange = $state<Exchange | null>(null);
  error = $state<string | null>(null);
  sending = $state(false);
  status = $state<Status>({ kind: "idle", text: "Abre una colección" });
  theme = $state<"dark" | "light">("dark");
  /** Carpeta elegida que aún no tiene colección: se ofrece crearla. */
  pendingInit = $state<string | null>(null);

  async open(path: string) {
    try {
      const col = await (await api).openCollection(path);
      this.collection = col;
      this.env = col.meta.default_environment;
      this.status = { kind: "ok", text: `${col.meta.name} · ${col.root}` };
      const first = Object.keys(col.requests)[0];
      if (first) this.select(first);
      try {
        localStorage.setItem("harpoon.lastCollection", path);
      } catch {
        /* sin almacenamiento */
      }
    } catch (e) {
      const msg = String(e);
      if (msg.includes("harpoon.toml")) {
        this.pendingInit = path;
        this.status = { kind: "idle", text: "Esa carpeta no tiene colección todavía" };
      } else {
        this.status = { kind: "err", text: msg };
      }
    }
  }

  async createHere(name: string) {
    const path = this.pendingInit;
    if (!path) return;
    try {
      const col = await (await api).initCollection(path, name.trim() || basename(path));
      this.pendingInit = null;
      this.collection = col;
      this.env = col.meta.default_environment;
      this.status = { kind: "ok", text: `Colección creada en ${col.root}` };
      const first = Object.keys(col.requests)[0];
      if (first) this.select(first);
      try {
        localStorage.setItem("harpoon.lastCollection", path);
      } catch {
        /* sin almacenamiento */
      }
    } catch (e) {
      this.status = { kind: "err", text: String(e) };
    }
  }

  async pickAndOpen() {
    const path = await (await api).pickFolder();
    if (path) await this.open(path);
  }

  select(key: string) {
    if (!this.collection) return;
    const req = this.collection.requests[key];
    if (!req) return;
    this.selected = key;
    this.draft = structuredClone($state.snapshot(req));
    this.dirty = false;
    this.exchange = null;
    this.error = null;
  }

  touch() {
    this.dirty = true;
  }

  async save() {
    if (!this.collection || !this.selected || !this.draft) return;
    const req = structuredClone($state.snapshot(this.draft));
    await (await api).saveRequest(this.collection.root, this.selected, req);
    this.collection.requests[this.selected] = req;
    this.dirty = false;
    this.status = { kind: "ok", text: `Guardado requests/${this.selected}.toml` };
  }

  async send() {
    if (!this.collection || !this.draft || this.sending) return;
    this.sending = true;
    this.error = null;
    this.status = { kind: "busy", text: `${this.draft.method} ${this.draft.url}` };
    try {
      const req = structuredClone($state.snapshot(this.draft));
      const x = await (await api).sendRequest(this.collection.root, this.env, req);
      this.exchange = x;
      this.status = {
        kind: x.passed ? "ok" : "err",
        text: `${x.response.status} · ${x.response.elapsed_ms} ms · ${fmtSize(x.response.size)}`,
      };
    } catch (e) {
      this.exchange = null;
      this.error = String(e);
      this.status = { kind: "err", text: String(e) };
    } finally {
      this.sending = false;
    }
  }

  toggleTheme() {
    this.theme = this.theme === "dark" ? "light" : "dark";
    document.documentElement.dataset.theme = this.theme;
    try {
      localStorage.setItem("harpoon.theme", this.theme);
    } catch {
      /* sin almacenamiento */
    }
  }

  restore() {
    try {
      const t = localStorage.getItem("harpoon.theme");
      if (t === "light" || t === "dark") {
        this.theme = t;
        document.documentElement.dataset.theme = t;
      }
      const last = localStorage.getItem("harpoon.lastCollection");
      if (last) void this.open(last);
    } catch {
      /* sin almacenamiento */
    }
  }
}

export const store = new Store();

export function basename(path: string): string {
  return path.replace(/[\\/]+$/, "").split(/[\\/]/).pop() ?? path;
}

export function fmtSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}
