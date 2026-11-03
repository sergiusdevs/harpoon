// Estado global de la app con runes de Svelte 5.

import { parseCurl } from "../domain/curl";
import { activateTab, emptyTabs, pruneTabs, remapTabs, type TabView } from "../domain/requestTabs";
import { validateGraphql } from "../domain/graphql";
import { requestFailure, type RequestFailure } from "../domain/requestFailure";
import { streams } from "./streams.svelte";
import { api, folderChain } from "../services/api";
import { resolveTheme, themes, type Theme } from "../domain/themes";
import { variableSuggestions, type VariableSuggestion } from "../domain/variables";
import { interfaceFonts, editorFonts, type InterfaceFont, type EditorFont } from "../domain/fonts";
import { convertRequestType, newTypedRequest, requestType, type RequestType, type Collection, type CollectionMeta, type Environment, type Exchange, type FolderConfig, type HistoryEntry, type Method, type MissingSecret, type Request, type SecretFinding, type SecretScope } from "../domain/types";
import { applySecret } from "../domain/secrets";
import { splitUrl } from "../domain/urlQuery";

export type Status = { kind: "idle" | "ok" | "err" | "busy"; text: string };

export type Dialog =
  | { kind: "request"; title: string; label: string; value: string; placeholder: string; method: Method; ok: string; folder: string; onOk: (value: string, method: Method, type: RequestType, url: string, file: string) => Promise<void> | void }
  | { kind: "prompt"; title: string; label?: string; value: string; placeholder?: string; ok: string; onOk: (value: string) => Promise<void> | void }
  | { kind: "confirm"; title: string; text: string; ok: string; danger?: boolean; onOk: () => Promise<void> | void };

class Store {
  collection = $state<Collection | null>(null);
  /** Carpetas bajo requests/, incluidas las vacías. */
  folders = $state<string[]>([]);
  selected = $state<string | null>(null);
  /** Solicitud y respuesta una encima de otra aunque haya ancho (botón de la barra de título). */
  forceStacked = $state(false);
  /** Barra lateral oculta (botón de la barra de título). */
  sidebarHidden = $state(false);
  tabs = $state(emptyTabs());
  env = $state<string | null>(null);
  /** Copia editable de la solicitud seleccionada. */
  draft = $state<Request | null>(null);
  dirty = $state(false);
  exchange = $state<Exchange | null>(null);
  error = $state<string | null>(null);
  sending = $state(false);
  cancelling = $state(false);
  requestFailure = $state<RequestFailure | null>(null);
  private sendId: string | null = null;
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
  modal = $state<"environments" | "settings" | "folder" | "themes" | "curl" | "variables" | "cookies" | "share" | "runner" | "search" | "stream" | "mock" | "secret-findings" | "missing-secrets" | null>(null);
  /** Posibles secretos detectados al guardar la solicitud abierta (modal "secret-findings"). */
  secretFindings = $state<{ key: string; findings: SecretFinding[] } | null>(null);
  /** Secretos declarados sin valor en esta máquina para el entorno activo. */
  missingSecrets = $state<MissingSecret[]>([]);
  /** Colección+entorno para los que el usuario ya cerró el aviso de secretos. */
  dismissedMissing = $state<string | null>(null);
  editingFolder = $state("");
  runFolder = $state("");

  /** Variables disponibles para una solicitud con esa clave (o en esa carpeta, con `key = "carpeta/x"`). */
  variablesFor(key: string): VariableSuggestion[] {
    const col = this.collection;
    if (!col) return [];
    const layers = [{ source: "Colección", names: Object.keys(col.meta.vars) }];
    for (const folder of folderChain(key)) {
      layers.push({ source: `Carpeta · ${folder}`, names: Object.keys(col.folder_configs[folder]?.vars ?? {}) });
    }
    const env = this.env ?? col.meta.default_environment;
    if (env) layers.push({ source: `Entorno · ${env}`, names: Object.keys(col.environments[env]?.vars ?? {}) });
    layers.push({ source: "Ejecución", names: Object.keys(this.runtimeVariables) });
    return variableSuggestions(layers);
  }

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

  get hasUnsavedTabs() { return this.dirty || Object.values(this.tabs.views).some((view) => view.dirty); }
  private currentView(): TabView | null {
    return this.draft ? { draft: this.draft, dirty: this.dirty, exchange: this.exchange, error: this.error, failure: this.requestFailure } : null;
  }
  tabRequest(key: string) { return key === this.selected ? this.draft : this.tabs.views[key]?.draft ?? this.collection?.requests[key]; }
  tabDirty(key: string) { return key === this.selected ? this.dirty : this.tabs.views[key]?.dirty ?? false; }
  confirmDiscardAll(action: () => void | Promise<void>): boolean {
    if (!this.hasUnsavedTabs) return true;
    this.dialog = { kind: "confirm", title: "Cambios sin guardar", text: "Hay solicitudes abiertas con cambios sin guardar. ¿Quieres descartarlos y continuar?", ok: "Descartar cambios", danger: true, onOk: async () => {
      this.tabs.views = {};
      if (this.selected && this.collection?.requests[this.selected]) this.draft = structuredClone($state.snapshot(this.collection.requests[this.selected]));
      this.dirty = false; await action();
    } };
    return false;
  }
  closeTab(key: string, discard = false) {
    if (this.tabDirty(key) && !discard) {
      this.dialog = { kind: "confirm", title: "Cerrar solicitud", text: "Esta solicitud tiene cambios sin guardar. ¿Quieres descartarlos y cerrar la pestaña?", ok: "Descartar y cerrar", danger: true, onOk: () => this.closeTab(key, true) }; return;
    }
    const index = this.tabs.opened.indexOf(key);
    this.tabs.opened = this.tabs.opened.filter((item) => item !== key); delete this.tabs.views[key];
    if (this.selected === key) {
      this.selected = null; this.draft = null; this.dirty = false; this.exchange = null; this.error = null; this.requestFailure = null; this.history = [];
      const next = this.tabs.opened[Math.min(index, this.tabs.opened.length - 1)]; if (next) this.select(next);
    }
  }
  private remapOpenTabs(from: string, to: string, folder = false) {
    const view = this.currentView();
    if (this.selected && view) this.tabs.views[this.selected] = JSON.parse(JSON.stringify($state.snapshot(view)));
    const rename = remapTabs(this.tabs, from, to, folder);
    if (this.selected) this.selected = rename(this.selected);
  }

  // ---------- colección ----------

  async open(path: string) {
    if (this.collection && path !== this.collection.root && !this.confirmDiscardAll(() => this.open(path))) return;
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
    if (!this.confirmDiscardAll(() => this.pickAndOpen())) return;
    const path = await (await api).pickFolder();
    if (path) await this.open(path);
  }

  /** Sustituye la colección en memoria conservando la selección si sigue existiendo. */
  private async adopt(col: Collection, opts: { selectFirst?: boolean; select?: string } = {}) {
    if (this.collection?.root !== col.root) {
      this.tabs = emptyTabs(); this.selected = null; this.draft = null; this.dirty = false;
    }
    pruneTabs(this.tabs, col.requests);
    // Los parámetros escritos dentro de la URL pasan a Params (se envía lo mismo).
    for (const request of Object.values(col.requests)) {
      const split = splitUrl(request.url);
      if (split.pairs.length) { request.url = split.base; request.query = { ...Object.fromEntries(split.pairs), ...request.query }; }
    }
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
        this.draft = null; this.dirty = false;
      }
    } else {
      this.selected = null;
      this.draft = null; this.dirty = false;
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
    if (opts.keepDraft && this.selected === key && this.draft) return;
    const view = activateTab(this.tabs, this.selected, this.currentView(), key, this.collection.requests);
    if (!view) return;
    this.selected = key; this.draft = view.draft; this.dirty = view.dirty;
    this.exchange = view.exchange; this.error = view.error; this.requestFailure = view.failure;
    this.viewingHistory = null; this.history = [];
    void this.loadHistory();
  }

  async loadHistory() {
    if (!this.collection || !this.selected) {
      this.history = [];
      return;
    }
    try {
      const root = this.collection.root, key = this.selected;
      const history = await (await api).history(root, key);
      if (this.collection?.root === root && this.selected === key) this.history = history;
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
    if (this.selected && this.tabs.views[this.selected]) this.tabs.views[this.selected].dirty = true;
  }

  /**
   * Guarda la solicitud abierta. Antes revisa si lleva algo que parezca un
   * secreto escrito tal cual (tokens, passwords…): si es así abre el modal
   * "secret-findings" y no guarda hasta que el usuario elija.
   */
  async save(opts: { force?: boolean } = {}) {
    if (!this.collection || !this.selected || !this.draft) return;
    const root = this.collection.root, key = this.selected;
    const request = structuredClone($state.snapshot(this.draft));
    if (!opts.force) {
      try {
        const findings = await (await api).scanRequest(key, request);
        if (findings.length) {
          this.secretFindings = { key, findings };
          this.modal = "secret-findings";
          return;
        }
      } catch { /* si el detector falla, se guarda igual */ }
    }
    this.secretFindings = null;
    await this.run(async () => {
      await (await api).saveRequest(root, key, request);
      if (this.collection?.root !== root) return;
      this.collection.requests[key] = request;
      const cached = this.tabs.views[key];
      if (cached && JSON.stringify(cached.draft) === JSON.stringify(request)) cached.dirty = false;
      if (this.selected === key && JSON.stringify($state.snapshot(this.draft)) === JSON.stringify(request)) {
        this.dirty = false; if (cached) { cached.dirty = false; cached.draft = request; }
      }
      this.status = { kind: "ok", text: `Guardado requests/${key}.toml` };
    });
  }

  changeRequestType(type: RequestType) {
    if (!this.draft || !this.collection || requestType(this.draft) === type || this.sending) return;
    const root = this.collection.root, key = this.selected;
    const active = streams.sessions.find((session) => session.root === root && session.key === key && session.state !== "closed");
    if (active) {
      this.dialog = { kind: "confirm", title: "Conexión activa", text: "Desconecta esta sesión antes de cambiar el tipo de solicitud.", ok: "Ver conexión", onOk: () => { streams.manager.select(active.identity); this.modal = "stream"; } };
      return;
    }
    try {
      const { request, warnings } = convertRequestType($state.snapshot(this.draft), type);
      const apply = () => {
        if (this.collection?.root !== root || this.selected !== key) return;
        this.draft = request; this.touch(); this.exchange = null;
        this.error = null; this.requestFailure = null;
      };
      if (warnings.length) this.dialog = { kind: "confirm", title: "Cambiar tipo de solicitud", text: warnings.join(" ") + " Los cambios quedarán pendientes de guardar.", ok: "Cambiar tipo", onOk: apply };
      else apply();
    } catch (cause) { this.status = { kind: "err", text: requestFailure(cause).detail }; }
  }

  openStream() {
    if (!this.collection || !this.selected || !this.draft) return;
    const type = requestType(this.draft);
    if (type !== "websocket" && type !== "sse") return;
    streams.manager.open(this.collection.root, this.selected, this.env, $state.snapshot(this.draft), type);
    this.modal = "stream";
  }

  async cancelSend() {
    if (!this.sendId || this.cancelling) return;
    this.cancelling = true;
    try { await (await api).cancelRequest(this.sendId); }
    catch (cause) { this.status = { kind: "err", text: requestFailure(cause).message }; this.cancelling = false; }
  }

  async send() {
    if (!this.collection || !this.draft || this.sending) return;
    if (["websocket", "sse"].includes(requestType(this.draft))) {
      this.openStream();
      return;
    }
    if (this.draft.body.type === "graphql" && !this.draft.scripts.pre && !this.collection.meta.scripts.pre) {
      const body = this.draft.body;
      const issues = validateGraphql(body.query, body.variables, body.operation_name, body.schema);
      if (issues.length) {
        const detail = issues.map((issue) => issue.message).join("\n");
        this.requestFailure = { code: "graphql", title: "Revisa la consulta GraphQL", message: "Corrige la consulta o las variables antes de enviarla.", detail };
        this.error = detail; return;
      }
    }
    const root = this.collection.root;
    const key = this.selected ?? "";
    const id = crypto.randomUUID();
    this.sendId = id;
    this.sending = true; this.cancelling = false;
    this.error = null; this.requestFailure = null;
    this.status = { kind: "busy", text: `${this.draft.method} ${this.draft.url}` };
    try {
      const req = structuredClone($state.snapshot(this.draft));
      const x = await (await api).sendRequest(root, this.env, req, key, id);
      if (this.collection?.root === root) this.runtimeVariables = x.vars;
      if (this.collection?.root === root && this.selected === key) {
        this.exchange = x;
        this.viewingHistory = null;
        void this.loadHistory();
      }
      if (this.collection?.root === root && this.tabs.opened.includes(key) && this.tabs.views[key]) {
        this.tabs.views[key].exchange = x; this.tabs.views[key].error = null; this.tabs.views[key].failure = null;
      }
      this.status = { kind: x.passed ? "ok" : "err", text: `${x.response.status} · ${x.response.elapsed_ms} ms · ${fmtSize(x.response.size)}` };
    } catch (cause) {
      const failure = requestFailure(cause);
      if (this.collection?.root === root && this.selected === key) {
        this.exchange = null; this.requestFailure = failure;
        this.error = failure.code === "cancelled" ? null : failure.detail || failure.message;
      }
      if (this.collection?.root === root && this.tabs.opened.includes(key) && this.tabs.views[key]) {
        this.tabs.views[key].exchange = null; this.tabs.views[key].error = failure.detail; this.tabs.views[key].failure = failure;
      }
      this.status = { kind: failure.code === "cancelled" ? "idle" : "err", text: failure.title };
    } finally {
      if (this.sendId === id) { this.sendId = null; this.sending = false; this.cancelling = false; }
    }
  }

  // ---------- crear / renombrar / borrar ----------

  importCurl() {
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
    this.dialog = {
      kind: "request",
      title: "Nueva solicitud",
      label: "Nombre de la solicitud",
      value: "",
      placeholder: "listar-usuarios",
      method: "GET",
      ok: "Crear",
      folder,
      onOk: async (name, method, type, url, file) => {
        if (!name.trim()) throw "El nombre no puede estar vacío";
        const base = slug(file) || slug(name);
        if (!base) throw "El nombre de archivo no puede estar vacío";
        const key = (folder ? folder + "/" : "") + base;
        if (this.collection!.requests[key]) throw `Ya existe ${key}`;
        // Si se pega un cURL en la URL, se importa entero (como en la barra de la solicitud).
        const fromCurl = /^\s*curl(?:\s|$)/.test(url);
        const req = fromCurl ? parseCurl(url) : newTypedRequest(name.trim(), type, method);
        req.name = name.trim(); req.type = requestType(req);
        if (!fromCurl && url.trim()) req.url = url.trim();
        await (await api).saveRequest(this.collection!.root, key, req);
        const col = await (await api).openCollection(this.collection!.root);
        await this.adopt(col, { select: key });
        this.status = { kind: "ok", text: `Creada requests/${key}.toml` };
      },
    };
  }

  toggleStacked() {
    this.forceStacked = !this.forceStacked;
    try { localStorage.setItem("harpoon.forceStacked", this.forceStacked ? "1" : "0"); } catch { /* sin almacenamiento */ }
  }

  toggleSidebar() {
    this.sidebarHidden = !this.sidebarHidden;
    try { localStorage.setItem("harpoon.sidebarHidden", this.sidebarHidden ? "1" : "0"); } catch { /* sin almacenamiento */ }
  }

  /** Copia una carpeta con sus subcarpetas, solicitudes y variables. */
  cloneFolder(folder: string) {
    this.dialog = {
      kind: "prompt",
      title: "Clonar carpeta",
      label: "Ruta de la copia",
      value: folder + "-copia",
      ok: "Clonar",
      onOk: async (to) => {
        const target = normalizeKey(to);
        if (!target) throw "La ruta no puede estar vacía";
        if (target === folder || target.startsWith(folder + "/")) throw "La copia no puede estar dentro de la carpeta original";
        const col = this.collection!;
        if (this.folders.includes(target) || Object.keys(col.requests).some((k) => k.startsWith(target + "/"))) throw `Ya existe ${target}/`;
        const a = await api;
        const remap = (path: string) => target + path.slice(folder.length);
        const inside = (path: string) => path === folder || path.startsWith(folder + "/");
        for (const f of [folder, ...this.folders.filter((f) => f.startsWith(folder + "/"))]) await a.createFolder(col.root, remap(f));
        for (const [key, req] of Object.entries(col.requests)) {
          if (key.startsWith(folder + "/")) await a.saveRequest(col.root, remap(key), structuredClone($state.snapshot(req)));
        }
        for (const [f, cfg] of Object.entries(col.folder_configs ?? {})) {
          if (!inside(f)) continue;
          if (Object.keys(cfg.vars ?? {}).length) await a.saveFolderVars(col.root, remap(f), { ...cfg.vars });
          await a.saveFolderSettings(col.root, remap(f), structuredClone($state.snapshot({ headers: cfg.headers, auth: cfg.auth, scripts: cfg.scripts })));
        }
        await this.adopt(await a.openCollection(col.root), { select: this.selected ?? undefined });
        this.status = { kind: "ok", text: `Clonada en ${target}/` };
      },
    };
  }

  async revealFolder(folder: string) {
    if (!this.collection) return;
    await this.run(async () => { await (await api).revealFolder(this.collection!.root, folder); });
  }

  async openTerminal(folder: string) {
    if (!this.collection) return;
    await this.run(async () => { await (await api).openTerminal(this.collection!.root, folder); });
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
        this.remapOpenTabs(key, target);
        await this.adopt(col, { select: this.selected ?? undefined });
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
        this.remapOpenTabs(folder, target, true);
        await this.adopt(col, { select: this.selected ?? undefined });
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
    if (kind === "folder" && (targetFolder === path || targetFolder.startsWith(path + "/"))) {
      this.status = { kind: "err", text: "No se puede mover una carpeta dentro de sí misma" };
      return;
    }
    await this.run(async () => {
      const col =
        kind === "request"
          ? await (await api).renameRequest(this.collection!.root, path, target)
          : await (await api).renameFolder(this.collection!.root, path, target);
      this.remapOpenTabs(path, target, kind === "folder");
      await this.adopt(col, { select: this.selected ?? undefined });
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
    if (!this.collection) throw new Error("Abre una colección primero");
    const col = await (await api).saveEnvironment(this.collection.root, name, env);
    await this.adopt(col);
    this.status = { kind: "ok", text: `Guardado environments/${name}.toml` };
  }

  async deleteEnvironment(name: string) {
    if (!this.collection) return;
    const col = await (await api).deleteEnvironment(this.collection.root, name);
    await this.adopt(col);
  }

  async saveFolderSettings(folder: string, settings: Pick<FolderConfig, "headers" | "auth" | "scripts">) {
    const col = await (await api).saveFolderSettings(this.collection!.root, folder, settings);
    await this.adopt(col);
  }

  async saveFolderVars(folder: string, vars: Record<string, string>, secrets?: string[]) {
    const col = await (await api).saveFolderVars(this.collection!.root, folder, vars, secrets);
    await this.adopt(col);
    this.status = { kind: "ok", text: `Guardado ${folder}/folder.toml` };
  }

  /** Exporta un entorno; los secretos salen con valor vacío. */
  async exportEnvironment(name: string, format: "postman" | "harpoon") {
    if (!this.collection) return;
    await this.run(async () => {
      const path = await (await api).exportEnvironment(this.collection!.root, name, format);
      if (path) this.status = { kind: "ok", text: `Entorno ${name} exportado (secretos sin valor)` };
    });
  }

  /** Elige un archivo de entorno y devuelve su vista previa (o null si se cancela). */
  async pickEnvironmentFile(): Promise<{ path: string; preview: import("../services/api").EnvironmentPreview } | null> {
    const a = await api;
    const path = await a.pickFile();
    if (!path) return null;
    return { path, preview: await a.previewEnvironment(path) };
  }

  /** Importa el entorno del archivo con ese nombre y lo deja seleccionado. */
  async importEnvironment(path: string, name: string) {
    if (!this.collection) throw new Error("Abre una colección primero");
    const col = await (await api).importEnvironment(this.collection.root, path, name);
    await this.adopt(col, { select: this.selected ?? undefined });
    await this.refreshMissingSecrets();
    this.status = { kind: "ok", text: `Entorno ${name} importado` };
  }

  /** Recalcula los secretos sin valor para la colección y el entorno activos. */
  async refreshMissingSecrets() {
    const col = this.collection;
    if (!col) { this.missingSecrets = []; return; }
    try {
      const missing = await (await api).missingSecrets(col.root, this.env);
      if (this.collection?.root === col.root) this.missingSecrets = missing;
    } catch { this.missingSecrets = []; }
  }

  /** Clave para recordar que el aviso de secretos ya se cerró. */
  get missingKey() { return `${this.collection?.root ?? ""}::${this.env ?? ""}`; }

  /** Guarda una variable secreta (valor local; en Git solo el nombre). */
  async setSecret(scope: SecretScope, name: string, value: string) {
    if (!this.collection) throw new Error("Abre una colección primero");
    const col = await (await api).setSecret(this.collection.root, scope, name, value);
    await this.adopt(col, { select: this.selected ?? undefined });
    await this.refreshMissingSecrets();
    const where = scope.kind === "collection" ? "la colección" : scope.kind === "folder" ? `la carpeta ${scope.path}` : `el entorno ${scope.name}`;
    this.status = { kind: "ok", text: `Secreto {{${name}}} guardado en ${where} (fuera de Git)` };
  }

  /**
   * Convierte un hallazgo del detector en secreto: guarda el valor como
   * variable secreta y en la solicitud abierta deja `{{nombre}}`.
   * Devuelve false si ese hallazgo no se puede convertir solo (cuerpo, scripts).
   */
  async convertFindingToSecret(finding: SecretFinding, name: string, scope: SecretScope): Promise<boolean> {
    if (!this.draft) return false;
    const draft = structuredClone($state.snapshot(this.draft));
    const result = applySecret(draft, finding.location, name);
    if (!result) return false;
    const keepDraft = result.request;
    await this.setSecret(scope, name, result.value);
    this.draft = keepDraft; this.touch();
    if (this.secretFindings) {
      this.secretFindings.findings = this.secretFindings.findings.filter((f) => f !== finding && f.location !== finding.location);
    }
    return true;
  }

  async saveMeta(meta: CollectionMeta) {
    await this.run(async () => {
      const col = await (await api).saveMeta(this.collection!.root, meta);
      await this.adopt(col);
      this.status = { kind: "ok", text: "Guardado harpoon.toml" };
    });
  }

  // ---------- varios ----------

  /** Aplica el tema elegido; "system" sigue el modo claro/oscuro del sistema en vivo. */
  private applyTheme() {
    const media = typeof matchMedia === "function" ? matchMedia("(prefers-color-scheme: dark)") : null;
    document.documentElement.dataset.theme = resolveTheme(this.theme, media?.matches ?? true);
    if (media && !this.followingSystem) {
      this.followingSystem = true;
      media.addEventListener("change", () => { if (this.theme === "system") this.applyTheme(); });
    }
  }
  private followingSystem = false;

  setTheme(theme: Theme) {
    this.theme = theme;
    this.applyTheme();
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
        this.applyTheme();
      }
      const interfaceFont = interfaceFonts.find((font) => font.id === localStorage.getItem("harpoon.interfaceFont"));
      const editorFont = editorFonts.find((font) => font.id === localStorage.getItem("harpoon.editorFont"));
      const size = localStorage.getItem("harpoon.editorFontSize");
      if (interfaceFont) this.setInterfaceFont(interfaceFont.id);
      if (editorFont) this.setEditorFont(editorFont.id);
      if (size !== null && size.trim() !== "") this.setEditorFontSize(Number(size));
      this.forceStacked = localStorage.getItem("harpoon.forceStacked") === "1";
      this.sidebarHidden = localStorage.getItem("harpoon.sidebarHidden") === "1";
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
