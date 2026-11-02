<script lang="ts">
  import { store, slug } from "../../state/store.svelte";
  import { api, isMock, type SharePreview } from "../../services/api";
  import Modal from "../common/Modal.svelte";
  let mode = $state<"export" | "import">(store.collection ? "export" : "import");
  let source = $state("");
  let parent = $state("");
  let name = $state("");
  let preview = $state<SharePreview | null>(null);
  let warnings = $state<string[]>([]);
  let message = $state("");
  let error = $state("");
  let busy = $state(false);
  async function chooseSource() {
    try {
      error = "";
      const path = await (await api).pickFile();
      if (!path) return;
      const data = await (await api).previewHarpoon(path);
      source = path; preview = data; name = slug(data.name) || "coleccion";
    } catch (e) { source = ""; preview = null; error = String(e); }
  }
  async function chooseParent() { const path = await (await api).pickFolder(); if (path) parent = path; }
  async function exportCollection() {
    if (!store.collection) return;
    if (store.dirty) { error = "Guarda la solicitud antes de exportar la colección."; return; }
    try {
      busy = true; error = ""; message = "";
      const result = await (await api).exportHarpoon(store.collection.root, slug(store.collection.meta.name) || "coleccion");
      if (result) { warnings = result; message = "Colección exportada."; }
    } catch (e) { error = String(e); } finally { busy = false; }
  }
  async function importCollection() {
    const folder = slug(name);
    if (!folder) { error = "Introduce un nombre de carpeta válido."; return; }
    if (store.dirty) { error = "Guarda la solicitud antes de cambiar de colección."; return; }
    try {
      busy = true; error = "";
      const col = await (await api).importHarpoon(source, parent, folder);
      store.modal = null;
      await store.open(col.root);
    } catch (e) { error = String(e); } finally { busy = false; }
  }
</script>
<Modal title="Compartir colección" width={700} onclose={() => (store.modal = null)}>
  <div class="segmented"><button class:active={mode === "export"} aria-pressed={mode === "export"} disabled={!store.collection} onclick={() => { mode = "export"; error = ""; }}>Exportar</button><button class:active={mode === "import"} aria-pressed={mode === "import"} onclick={() => { mode = "import"; error = ""; }}>Importar</button></div>
  {#if isMock}<p class="muted hint">Disponible en la aplicación de escritorio.</p>{/if}
  {#if mode === "export"}
    <p class="muted hint">El archivo incluye la colección y sus entornos públicos. No incluye secretos de entorno, cookies ni historial. Las credenciales escritas directamente en las solicitudes sí se incluyen.</p>
    <button class="btn" disabled={busy || isMock || !store.collection} onclick={exportCollection}>Exportar colección…</button>
  {:else}
    <p class="muted hint">La colección se importará en una carpeta nueva, sin sobrescribir archivos existentes.</p>
    <div class="form"><div class="row"><button class="btn" disabled={isMock} onclick={chooseSource}>Elegir archivo…</button><span class="muted path">{source || "Ningún archivo seleccionado"}</span></div>
      {#if preview}<p class="hint">Colección: <b>{preview.name}</b></p>{#each preview.warnings as warning}<p class="warning">{warning}</p>{/each}
        <div class="row"><button class="btn" onclick={chooseParent}>Elegir destino…</button><span class="muted path">{parent || "Ninguna carpeta seleccionada"}</span></div>
        <label>Nombre de la carpeta<input bind:value={name} /></label>
        <button class="btn primary" disabled={busy || !parent || !name.trim()} onclick={importCollection}>Importar colección</button>
      {/if}</div>
  {/if}
  {#if message}<p class="success">{message}</p>{/if}{#each warnings as warning}<p class="warning">{warning}</p>{/each}{#if error}<p class="error" role="alert">{error}</p>{/if}
  {#snippet footer()}<button class="btn" onclick={() => (store.modal = null)}>Cerrar</button>{/snippet}
</Modal>
<style>
  .hint { font-size: 12px; line-height: 1.5; } .form { display: flex; flex-direction: column; gap: 10px; align-items: flex-start; }
  .row { display: flex; align-items: center; gap: 10px; width: 100%; } .path { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 11px; }
  label { display: flex; flex-direction: column; gap: 5px; font-size: 12px; color: var(--text-muted); width: min(100%, 300px); }
  .error, .warning, .success { font-size: 12px; } .error { color: var(--err); } .warning { color: var(--warn); } .success { color: var(--ok); }
</style>
