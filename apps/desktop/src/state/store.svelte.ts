// Estado global de la app con runes de Svelte 5.

import { api, folderChain } from "../services/api";
import { themes, type Theme } from "../domain/themes";
import { interfaceFonts, editorFonts, type InterfaceFont, type EditorFont } from "../domain/fonts";
import { emptyRequest, type Collection, type CollectionMeta, type Environment, type Exchange, type HistoryEntry, type Method, type Request } from "../domain/types";

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
  theme = $state<Theme>("dark");
  interfaceFont = $state<InterfaceFont>("system");
  editorFont = $state<EditorFont>("default");
  editorFontSize = $state(12.5);
  /** Carpeta elegida que aún no tiene colección: se ofrece crearla. */
  pendingInit = $state<string | null>(null);
  dialog = $state<Dialog | null>(null);
  history = $state<HistoryEntry[]>([]);
  runtimeVariables = $state<Record<string, string>>({});
  /** Entrada del historial que se está viendo (null = respuesta en vivo). */
  viewingHistory = $state<HistoryEntry | null>(null);
  modal = $state<"environments" | "settings" | "folder" | "themes" | "curl" | "variables" | "cookies" | "share" | "runner" | "search" | null>(null);
  editingFolder = $state("");
  runFolder = $state("");

  openRunner(folder = "") {
    this.runFolder = folder;
    this.modal = "runner";
  }

  /** Pide confirmación antes de perder el borrador; devuelve false si queda pendiente. */
  confirmDiscard(action: () => void | Promise<void>): boolean {
    if (!this.dirty) return true;
    this.dialog = {
      kind: "confirm", title: "Cambios sin guardar",
      text: "La solicitud tiene cambios sin guardar. ¿Quieres descartarlos?",
      ok: "Descartar", danger: true,
      onOk: async () => {
        const saved = this.selected && this.collection?.requests[this.selected];
        if (saved) this.draft = structuredClone($state.snapshot(saved));
        this.dirty = false;
        await action();
      },
    };
    return false;
  }

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
    if (!this.confirmDiscard(() => this.pickAndOpen())) return;
    const path = await (await api).pickFolder();
    if (path) await this.open(path);
  }

  /** Sustituye la colección en memoria conservando la selección si sigue existiendo. */
  private async adopt(col: Collection, opts: { selectFirst?: boolean; select?: string } = {}) {
    this.collection = col;
    this.runtimeVariables = {};
    try { this.runtimeVariables = await (await api).runtimeVars(col.root); } catch { /* la colección sigue disponible */ }
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
    if (key === this.selected && this.dirty) return;
    if (key !== this.selected && !this.confirmDiscard(() => this.select(key, opts))) return;
    const req = this.collection.requests[key];
    if (!req) return;
    if (opts.keepDraft && this.selected === key && this.draft) return;
    this.selected = key;
    this.draft = structuredClone($state.snapshot(req));
    this.dirty = false;
    this.exchange = null;
    this.error = null;
    this.viewingHistory = null;
    void this.loadHistory();
  }

  async loadHistory() {
    if (!this.collection || !this.selected) {
      this.history = [];
      return;
    }
    try {
      this.history = await (await api).history(this.collection.root, this.selected);
    } catch {
      this.history = [];
    }
  }

  async clearHistory() {
    if (!this.collection || !this.selected) return;
    await (await api).clearHistory(this.collection.root, this.selected);
    this.history = [];
    this.viewingHistory = null;
  }

  /** Muestra una respuesta antigua en el panel de respuesta. */
  showHistory(entry: HistoryEntry | null) {
    this.viewingHistory = entry;
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
      const x = await (await api).sendRequest(this.collection.root, this.env, req, this.selected ?? "");
      this.exchange = x;
      this.runtimeVariables = x.vars;
      this.viewingHistory = null;
      void this.loadHistory();
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

  importCurl() {
    if (!this.confirmDiscard(() => this.importCurl())) return;
    this.modal = "curl";
  }

  async createImportedRequest(name: string, request: Request) {
    if (!this.collection) throw new Error("Abre una colección primero");
    const key = slug(name);
    if (!key) throw new Error("Introduce un nombre para la solicitud");
    if (this.collection.requests[key]) throw new Error(`Ya existe ${key}`);
    await (await api).saveRequest(this.collection.root, key, { ...request, name: name.trim() });
    const col = await (await api).openCollection(this.collection.root);
    await this.adopt(col, { select: key });
    this.status = { kind: "ok", text: `Importada requests/${key}.toml` };
  }

  newRequest(folder = "") {
    if (!this.confirmDiscard(() => this.newRequest(folder))) return;
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
    if (!this.confirmDiscard(() => this.duplicate(key))) return;
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
    if (this.selected === key && !this.confirmDiscard(() => this.renameRequest(key))) return;
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
    if (this.selected?.startsWith(folder + "/") && !this.confirmDiscard(() => this.renameFolder(folder))) return;
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

  /** Mueve una solicitud o carpeta a otra carpeta (arrastrar y soltar). */
  async moveInto(kind: "request" | "folder", path: string, targetFolder: string) {
    if (!this.collection) return;
    const name = path.split("/").pop()!;
    const target = targetFolder ? `${targetFolder}/${name}` : name;
    if (target === path) return;
    if ((this.selected === path || (kind === "folder" && this.selected?.startsWith(path + "/"))) && !this.confirmDiscard(() => this.moveInto(kind, path, targetFolder))) return;
    if (kind === "folder" && (targetFolder === path || targetFolder.startsWith(path + "/"))) {
      this.status = { kind: "err", text: "No se puede mover una carpeta dentro de sí misma" };
      return;
    }
    await this.run(async () => {
      const col =
        kind === "request"
          ? await (await api).renameRequest(this.collection!.root, path, target)
          : await (await api).renameFolder(this.collection!.root, path, target);
      const sel =
        kind === "request" && this.selected === path
          ? target
          : kind === "folder" && this.selected?.startsWith(path + "/")
            ? target + this.selected.slice(path.length)
            : undefined;
      await this.adopt(col, { select: sel });
      this.status = { kind: "ok", text: `Movido a ${targetFolder || "la raíz"}` };
    });
  }

  /** Crea un entorno vacío y lo selecciona (botón + junto al selector). */
  quickNewEnvironment() {
    this.dialog = {
      kind: "prompt",
      title: "Nuevo entorno",
      label: "Nombre (dev, staging, prod…)",
      value: "",
      placeholder: "staging",
      ok: "Crear",
      onOk: async (name) => {
        const n = slug(name);
        if (!n) throw "El nombre no puede estar vacío";
        if (this.collection?.environments[n]) throw `Ya existe el entorno ${n}`;
        const base = this.env ? this.collection?.environments[this.env]?.vars ?? {} : {};
        await this.saveEnvironment(n, { vars: { ...base }, secrets: [] });
        this.env = n;
        this.modal = "environments";
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

  async saveFolderVars(folder: string, vars: Record<string, string>) {
    const col = await (await api).saveFolderVars(this.collection!.root, folder, vars);
    await this.adopt(col);
    this.status = { kind: "ok", text: `Guardado ${folder}/folder.toml` };
  }

  async saveMeta(meta: CollectionMeta) {
    await this.run(async () => {
      const col = await (await api).saveMeta(this.collection!.root, meta);
      await this.adopt(col);
      this.status = { kind: "ok", text: "Guardado harpoon.toml" };
    });
  }

  // ---------- varios ----------

  setTheme(theme: Theme) {
    this.theme = theme;
    document.documentElement.dataset.theme = this.theme;
    try {
      localStorage.setItem("harpoon.theme", this.theme);
    } catch {
      /* sin almacenamiento */
    }
  }

  setInterfaceFont(font: InterfaceFont) {
    this.interfaceFont = font;
    document.documentElement.dataset.interfaceFont = font;
    try { localStorage.setItem("harpoon.interfaceFont", font); } catch { /* sin almacenamiento */ }
  }

  setEditorFont(font: EditorFont) {
    this.editorFont = font;
    document.documentElement.dataset.editorFont = font;
    try { localStorage.setItem("harpoon.editorFont", font); } catch { /* sin almacenamiento */ }
  }

  setEditorFontSize(size: number) {
    if (!Number.isFinite(size)) return;
    this.editorFontSize = Math.min(20, Math.max(10, size));
    document.documentElement.style.setProperty("--editor-font-size", `${this.editorFontSize}px`);
    try { localStorage.setItem("harpoon.editorFontSize", String(this.editorFontSize)); } catch { /* sin almacenamiento */ }
  }

  restore() {
    try {
      const t = localStorage.getItem("harpoon.theme");
      const theme = themes.find((item) => item.id === t);
      if (theme) {
        this.theme = theme.id;
        document.documentElement.dataset.theme = theme.id;
      }
      const interfaceFont = interfaceFonts.find((font) => font.id === localStorage.getItem("harpoon.interfaceFont"));
      const editorFont = editorFonts.find((font) => font.id === localStorage.getItem("harpoon.editorFont"));
      const size = localStorage.getItem("harpoon.editorFontSize");
      if (interfaceFont) this.setInterfaceFont(interfaceFont.id);
      if (editorFont) this.setEditorFont(editorFont.id);
      if (size !== null && size.trim() !== "") this.setEditorFontSize(Number(size));
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
