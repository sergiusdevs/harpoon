<script lang="ts">
  import { onMount } from "svelte";
  import { store } from "../../state/store.svelte";
  import { api, isMock, type MockConfig, type MockRoute, type MockStatus } from "../../services/api";
  import { METHODS, type Method } from "../../domain/types";
  import Modal from "../common/Modal.svelte";
  import KeyValue from "../common/KeyValue.svelte";

  const root = store.collection!.root;
  let config = $state<MockConfig>({ routes: [] });
  let server = $state<MockStatus | null>(null);
  let selected = $state<number | null>(null);
  let port = $state(8787);
  let dirty = $state(false);
  let busy = $state(false);
  let loading = $state(true);
  let error = $state("");
  const activeHere = $derived(server?.root === root);
  const route = $derived(selected === null ? null : config.routes[selected] ?? null);
  onMount(() => { if (isMock) { loading = false; return; } void (async () => {
    try { const bridge = await api; [config, server] = await Promise.all([bridge.loadMock(root), bridge.mockStatus()]); if (config.routes.length) selected = 0; }
    catch (cause) { error = String(cause); } finally { loading = false; }
  })(); });
  function close() {
    if (!dirty) { store.modal = null; return; }
    store.dialog = { kind: "confirm", title: "Cambios sin guardar", text: "Se descartarán los cambios de las respuestas mock.", ok: "Descartar", danger: true, onOk: () => { store.modal = null; } };
  }
  function add() {
    config.routes.push({ enabled: true, method: "GET", path: "/nueva-ruta", status: 200, headers: { "Content-Type": "application/json" }, body: "{}" });
    selected = config.routes.length - 1; dirty = true;
  }
  function patch(value: Partial<MockRoute>) { if (selected === null || !config.routes[selected]) return; Object.assign(config.routes[selected], value); dirty = true; }
  function remove() { if (selected === null) return; config.routes.splice(selected, 1); selected = config.routes.length ? Math.min(selected, config.routes.length - 1) : null; dirty = true; }
  async function save(): Promise<boolean> {
    try { await (await api).saveMock(root, structuredClone($state.snapshot(config))); dirty = false; return true; }
    catch (cause) { error = String(cause); return false; }
  }
  async function saveChanges() { busy = true; error = ""; try { await save(); } finally { busy = false; } }
  async function start() {
    if (!Number.isInteger(port) || port < 0 || port > 65535) { error = "El puerto debe estar entre 0 y 65535."; return; }
    busy = true; error = "";
    try { if (dirty && !(await save())) return; server = await (await api).startMock(root, port); }
    catch (cause) { error = String(cause); } finally { busy = false; }
  }
  async function stop() { busy = true; error = ""; try { await (await api).stopMock(); server = null; } catch (cause) { error = String(cause); } finally { busy = false; } }
</script>
<Modal title="Servidor mock local" width={900} onclose={close}>
  <div class="server-bar"><span class:online={!!server} class="status">{server ? "● En ejecución" : "○ Detenido"}</span>{#if server}<span class="mono address">{server.address}</span>{#if !activeHere}<span class="muted hint">Otra colección</span>{/if}<button class="btn" disabled={busy} onclick={stop}>Detener</button>{:else}<label class="port">Puerto <input type="number" min="0" max="65535" bind:value={port} disabled={busy || loading || isMock} /></label><button class="btn primary" disabled={busy || loading || isMock} onclick={start}>Iniciar</button>{/if}</div>
  <p class="muted hint">Solo accesible desde este equipo. Las rutas se evalúan en orden; usa <code>/usuarios/&#123;id&#125;</code> para un segmento variable. Los cambios se guardan en <code>mocks.toml</code>.</p>
  {#if isMock}<p class="muted hint">Disponible en la aplicación de escritorio.</p>{/if}
  {#if activeHere}<p class="muted hint">Detén el servidor para modificar sus respuestas. Sigue funcionando al cerrar esta ventana.</p>{/if}
  {#if error}<p class="error" role="alert">{error}</p>{/if}
  <div class="layout">
    <div class="routes"><div class="route-title"><span>Rutas</span><button class="btn small" onclick={add} disabled={busy || loading || isMock || activeHere}>＋ Añadir</button></div>
      {#each config.routes as item, index}<button class="route" class:chosen={selected === index} onclick={() => (selected = index)}><span class="method">{item.method}</span><span class="route-path">{item.path}</span>{#if !item.enabled}<span class="muted">inactiva</span>{/if}</button>{/each}
      {#if !config.routes.length}<p class="muted hint empty">Añade una ruta para configurar su respuesta.</p>{/if}
    </div>
    <div class="editor">
      {#if route}
        <fieldset disabled={busy || loading || isMock || activeHere}>
          <div class="row"><label class="enabled"><input type="checkbox" checked={route.enabled} onchange={(e) => patch({ enabled: e.currentTarget.checked })} /> Activa</label><label>Método<select value={route.method} onchange={(e) => patch({ method: e.currentTarget.value as Method })}>{#each METHODS as method}<option value={method}>{method}</option>{/each}</select></label><label class="path">Ruta<input class="mono" value={route.path} oninput={(e) => patch({ path: e.currentTarget.value })} placeholder={"/usuarios/{id}"} /></label><label>Estado<input type="number" min="200" max="599" value={route.status} oninput={(e) => patch({ status: Number(e.currentTarget.value) })} /></label></div>
          <div class="section-title">Cabeceras de respuesta</div>
          <KeyValue entries={route.headers} onchange={(headers) => patch({ headers })} keyPlaceholder="Cabecera" valuePlaceholder="valor" addLabel="Añadir cabecera" />
          <label class="body-label">Cuerpo<textarea class="mono" value={route.body} oninput={(e) => patch({ body: e.currentTarget.value })} spellcheck="false"></textarea></label>
          <button class="btn small delete" onclick={remove}>Eliminar ruta</button>
        </fieldset>
      {:else}<p class="muted hint">Selecciona una ruta para editar su respuesta.</p>{/if}
    </div>
  </div>
  {#snippet footer()}<span class="muted hint unsaved">{dirty ? "Cambios sin guardar" : ""}</span><button class="btn" onclick={close}>Cerrar</button><button class="btn primary" disabled={!dirty || busy || loading || isMock || activeHere} onclick={saveChanges}>Guardar</button>{/snippet}
</Modal>
<style>
  .server-bar, .row, .route-title { display: flex; align-items: end; gap: 10px; } .server-bar { align-items: center; flex-wrap: wrap; } .status { font-size: 12px; color: var(--text-muted); } .status.online { color: var(--ok); } .address { font-size: 12px; user-select: text; } .port { display: flex; flex-direction: row; align-items: center; gap: 6px; font-size: 12px; } .port input { width: 90px; }
  .hint, .error { font-size: 12px; line-height: 1.5; } .error { color: var(--err); overflow-wrap: anywhere; }
  .layout { display: grid; grid-template-columns: minmax(150px, 220px) minmax(0, 1fr); gap: 16px; min-height: 300px; } .routes { border-right: 1px solid var(--border); padding-right: 12px; } .route-title { justify-content: space-between; align-items: center; font-size: 12px; margin-bottom: 8px; } .route { display: flex; gap: 7px; width: 100%; padding: 8px; border-radius: 5px; text-align: left; font-size: 11px; } .route:hover, .route.chosen { background: var(--bg-hover); } .method { color: var(--accent); } .route-path { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; } .empty { padding: 6px; }
  fieldset { border: 0; padding: 0; margin: 0; min-width: 0; } .row { flex-wrap: wrap; } label { display: flex; flex-direction: column; gap: 5px; font-size: 12px; color: var(--text-muted); } label input[type="number"] { width: 75px; } .path { flex: 1; min-width: 120px; } .enabled { flex-direction: row; align-items: center; padding-bottom: 8px; } .section-title { color: var(--text-muted); font-size: 12px; margin: 14px 0 6px; } .body-label { margin-top: 14px; } textarea { min-height: 125px; resize: vertical; } .delete { margin-top: 12px; color: var(--err); } .unsaved { margin-right: auto; }
  @media (max-width: 600px) { .layout { grid-template-columns: 1fr; } .routes { border-right: 0; border-bottom: 1px solid var(--border); padding-bottom: 8px; } }
</style>
