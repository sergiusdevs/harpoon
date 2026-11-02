<script lang="ts">
  import { store, folderChain } from "../../state/store.svelte";
  import Modal from "../common/Modal.svelte";
  import KeyValue from "../common/KeyValue.svelte";

  const folder = store.editingFolder;
  let vars = $state<Record<string, string>>(structuredClone($state.snapshot(store.collection!.folder_configs[folder]?.vars ?? {})));
  let dirty = $state(false);
  let error = $state("");
  const inherited = $derived.by(() => {
    const values: Record<string, { value: string; source: string }> = {};
    for (const [key, value] of Object.entries(store.collection!.meta.vars)) values[key] = { value, source: "Colección" };
    for (const parent of folderChain(folder)) {
      for (const [key, value] of Object.entries(store.collection!.folder_configs[parent]?.vars ?? {})) values[key] = { value, source: parent };
    }
    return values;
  });
  const overridden = $derived(Object.keys(vars).filter((key) => key in inherited));
  const envOverrides = $derived(Object.keys(vars).filter((key) => key in (store.collection?.environments[store.env ?? ""]?.vars ?? {})));

  async function save() {
    try {
      await store.saveFolderVars(folder, structuredClone($state.snapshot(vars)));
      store.modal = null;
    } catch (e) { error = String(e); }
  }
</script>

<Modal title={`Variables · ${folder}`} width={680} onclose={() => (store.modal = null)}>
  <p class="muted hint">Estas variables se aplican a las solicitudes de esta carpeta y sus subcarpetas. El entorno y las variables de ejecución tienen prioridad.</p>
  <KeyValue entries={vars} onchange={(value) => { vars = value; dirty = true; }} keyPlaceholder="variable" addLabel="Añadir variable" />
  {#if overridden.length}<p class="hint">Sustituyen valores heredados: <span class="mono">{overridden.join(", ")}</span>.</p>{/if}
  {#if envOverrides.length}<p class="hint">El entorno activo sustituye: <span class="mono">{envOverrides.join(", ")}</span>.</p>{/if}
  {#if Object.keys(inherited).length}
    <h3>Variables heredadas</h3>
    <div class="inherited">
      {#each Object.entries(inherited) as [key, item]}
        <div><span class="mono">{key} = {item.value}</span><span class="muted">{item.source}{key in vars ? " · sustituida" : ""}</span></div>
      {/each}
    </div>
  {/if}
  {#if error}<p class="error" role="alert">{error}</p>{/if}
  {#snippet footer()}
    <button class="btn" onclick={() => (store.modal = null)}>Cancelar</button>
    <button class="btn primary" onclick={save} disabled={!dirty}>Guardar</button>
  {/snippet}
</Modal>

<style>
  .hint { font-size: 12px; }
  h3 { margin: 20px 0 8px; font-size: 12px; }
  .inherited { max-height: 180px; overflow: auto; border-top: 1px solid var(--border); }
  .inherited div { display: flex; justify-content: space-between; gap: 12px; padding: 7px 4px; border-bottom: 1px solid var(--border); font-size: 12px; }
  .error { color: var(--err); }
</style>
