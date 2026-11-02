// Estado global de la app con runes de Svelte 5.

import { api, folderChain } from "./api";
import { emptyRequest, type Collection, type CollectionMeta, type Environment, type Exchange, type Method, type Request } from "./types";

export type Status = { kind: "idle" | "ok" | "err" | "busy"; text: string };

export type Dialog =
  | { kind: "prompt"; title: string; label?: string; value: string; placeholder?: string; ok: string; onOk: (value: string) => Promise<void> | void }
  | { kind: "confirm"; title: string; text: string; ok: string; danger?: boolean; onOk: () => Promise<void> | void };

class Store {
  collection = $state<Collection | null>(null);
  /** Carpetas bajo requests/, incluidas las vacías. */
  folders = $state<string[]>([]);
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
  dialog = $state<Dialog | null>(null);
  modal = $state<"environments" | "settings" | null>(null);

  // ---------- colección ----------

  async open(path: string) {
    try {
      const col = await (await api).openCollection(path);
      await this.adopt(col, { selectFirst: true });
      this.status = { kind: "ok", text: `${col.meta.name} · ${col.root}` };
      this.remember(path);
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
    await this.run(async () => {
      const col = await (await api).initCollection(path, name.trim() || basename(path));
      this.pendingInit = null;
      await this.adopt(col, { selectFirst: true });
      this.status = { kind: "ok", text: `Colección creada en ${col.root}` };
      this.remember(path);
    });
  }

  async pickAndOpen() {
    const path = await (await api).pickFolder();
    if (path) await this.open(path);
  }

  /** Sustituye la colección en memoria conservando la selección si sigue existiendo. */
  private async adopt(col: Collection, opts: { selectFirst?: boolean; select?: string } = {}) {
    this.collection = col;
    if (this.env && !(this.env in col.environments)) this.env = null;
    if (!this.env) this.env = col.meta.default_environment;
    this.folders = await (await api).listFolders(col.root);
    const target = opts.select ?? (this.selected && this.selected in col.requests ? this.selected : null);
    if (target) this.select(target, { keepDraft: target === this.selected });
    else if (opts.selectFirst) {
      const first = Object.keys(col.requests)[0];
      if (first) this.select(first);
      else {
        this.selected = null;
        this.draft = null;
      }
    } else {
      this.selected = null;
      this.draft = null;
    }
  }

  private remember(path: string) {
    try {
      localStorage.setItem("harpoon.lastCollection", path);
    } catch {
      /* sin almacenamiento */
    }
  }

  /** Ejecuta una operación mostrando el error en la barra de estado. */
  private async run(fn: () => Promise<void>) {
    try {
      await fn();
    } catch (e) {
      this.status = { kind: "err", text: String(e) };
    }
  }

  // ---------- solicitud seleccionada ----------

  select(key: string, opts: { keepDraft?: boolean } = {}) {
    if (!this.collection) return;
    const req = this.collection.requests[key];
    if (!req) return;
    if (opts.keepDraft && this.selected === key && this.draft) return;
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
    await this.run(async () => {
      const req = structuredClone($state.snapshot(this.draft!));
      await (await api).saveRequest(this.collection!.root, this.selected!, req);
      this.collection!.requests[this.selected!] = req;
      this.dirty = false;
      this.status = { kind: "ok", text: `Guardado requests/${this.selected}.toml` };
    });
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

  // ---------- crear / renombrar / borrar ----------

  newRequest(folder = "") {
    this.dialog = {
      kind: "prompt",
      title: folder ? `Nueva solicitud en ${folder}/` : "Nueva solicitud",
      label: "Nombre (identificador de archivo)",
      value: "",
      placeholder: "listar-usuarios",
      ok: "Crear",
      onOk: async (name) => {
        const key = (folder ? folder + "/" : "") + slug(name);
        if (!slug(name)) throw "El nombre no puede estar vacío";
        if (this.collection!.requests[key]) throw `Ya existe ${key}`;
        const req = emptyRequest(name.trim());
        await (await api).saveRequest(this.collection!.root, key, req);
        const col = await (await api).openCollection(this.collection!.root);
        await this.adopt(col, { select: key });
        this.status = { kind: "ok", text: `Creada requests/${key}.toml` };
      },
    };
  }

  duplicate(key: string) {
    const src = this.collection?.requests[key];
    if (!src) return;
    this.dialog = {
      kind: "prompt",
      title: "Duplicar solicitud",
      label: "Nuevo identificador",
      value: key + "-copia",
      ok: "Duplicar",
      onOk: async (to) => {
        const target = normalizeKey(to);
        if (this.collection!.requests[target]) throw `Ya existe ${target}`;
        const req = structuredClone($state.snapshot(src));
        req.name = req.name + " (copia)";
        await (await api).saveRequest(this.collection!.root, target, req);
        const col = await (await api).openCollection(this.collection!.root);
        await this.adopt(col, { select: target });
      },
    };
  }

  renameRequest(key: string) {
    this.dialog = {
      kind: "prompt",
      title: "Renombrar solicitud",
      label: "Identificador (puede incluir carpeta: users/list)",
      value: key,
      ok: "Renombrar",
      onOk: async (to) => {
        const target = normalizeKey(to);
        if (target === key) return;
        const col = await (await api).renameRequest(this.collection!.root, key, target);
        await this.adopt(col, { select: this.selected === key ? target : undefined });
      },
    };
  }

  deleteRequest(key: string) {
    this.dialog = {
      kind: "confirm",
      title: "Borrar solicitud",
      text: `Se eliminará requests/${key}.toml. Esta acción no se puede deshacer.`,
      ok: "Borrar",
      danger: true,
      onOk: async () => {
        const col = await (await api).deleteRequest(this.collection!.root, key);
        if (this.selected === key) this.selected = null;
        await this.adopt(col, { selectFirst: true });
      },
    };
  }

  newFolder(parent = "") {
    this.dialog = {
      kind: "prompt",
      title: parent ? `Nueva carpeta en ${parent}/` : "Nueva carpeta",
      label: "Nombre",
      value: "",
      placeholder: "usuarios",
      ok: "Crear",
      onOk: async (name) => {
        const folder = (parent ? parent + "/" : "") + slug(name);
        if (!slug(name)) throw "El nombre no puede estar vacío";
        const col = await (await api).createFolder(this.collection!.root, folder);
        await this.adopt(col);
      },
    };
  }

  renameFolder(folder: string) {
    this.dialog = {
      kind: "prompt",
      title: "Renombrar carpeta",
      label: "Ruta de la carpeta",
      value: folder,
      ok: "Renombrar",
      onOk: async (to) => {
        const target = normalizeKey(to);
        if (target === folder) return;
        const col = await (await api).renameFolder(this.collection!.root, folder, target);
        const sel = this.selected?.startsWith(folder + "/") ? target + this.selected.slice(folder.length) : undefined;
        await this.adopt(col, { select: sel });
      },
    };
  }

  deleteFolder(folder: string) {
    const n = Object.keys(this.collection?.requests ?? {}).filter((k) => k.startsWith(folder + "/")).length;
    this.dialog = {
      kind: "confirm",
      title: "Borrar carpeta",
      text: n ? `Se eliminará la carpeta ${folder}/ y sus ${n} solicitud(es). No se puede deshacer.` : `Se eliminará la carpeta vacía ${folder}/.`,
      ok: "Borrar",
      danger: true,
      onOk: async () => {
        const col = await (await api).deleteFolder(this.collection!.root, folder);
        if (this.selected?.startsWith(folder + "/")) this.selected = null;
        await this.adopt(col, { selectFirst: true });
      },
    };
  }

  // ---------- entornos y ajustes ----------

  async saveEnvironment(name: string, env: Environment) {
    await this.run(async () => {
      const col = await (await api).saveEnvironment(this.collection!.root, name, env);
      await this.adopt(col);
      this.status = { kind: "ok", text: `Guardado environments/${name}.toml` };
    });
  }

  async deleteEnvironment(name: string) {
    await this.run(async () => {
      const col = await (await api).deleteEnvironment(this.collection!.root, name);
      await this.adopt(col);
    });
  }

  async saveMeta(meta: CollectionMeta) {
    await this.run(async () => {
      const col = await (await api).saveMeta(this.collection!.root, meta);
      await this.adopt(col);
      this.status = { kind: "ok", text: "Guardado harpoon.toml" };
    });
  }

  // ---------- varios ----------

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

/** Convierte un nombre libre en un segmento de archivo razonable. */
export function slug(name: string): string {
  return name
    .trim()
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .replace(/[^A-Za-z0-9._ -]+/g, "-")
    .replace(/\s+/g, "-")
    .replace(/-+/g, "-")
    .replace(/^-|-$/g, "")
    .toLowerCase();
}

function normalizeKey(key: string): string {
  return key
    .split("/")
    .map((p) => p.trim())
    .filter(Boolean)
    .join("/");
}

export function fmtSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}

export { folderChain, type Method };
