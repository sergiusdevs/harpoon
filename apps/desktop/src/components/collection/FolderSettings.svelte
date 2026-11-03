<script lang="ts">
  import { store, folderChain } from "../../state/store.svelte";
  import Modal from "../common/Modal.svelte";
  import KeyValue from "../common/KeyValue.svelte";
  import Icon from "../common/Icon.svelte";

  const folder = store.editingFolder;
  let vars = $state<Record<string, string>>(structuredClone($state.snapshot(store.collection!.folder_configs[folder]?.vars ?? {})));
  let secrets = $state<string[]>([...(store.collection!.folder_configs[folder]?.secrets ?? [])]);
  let dirty = $state(false);
  let revealed = $state<string[]>([]);
  let error = $state("");
  const inherited = $derived.by(() => {
    const values: Record<string, { value: string; source: string; secret: boolean }> = {};
    const colSecrets = store.collection!.meta.secrets ?? [];
    for (const [key, value] of Object.entries(store.collection!.meta.vars)) values[key] = { value, source: "Colección", secret: colSecrets.includes(key) };
    for (const parent of folderChain(folder)) {
      const cfg = store.collection!.folder_configs[parent];
      for (const [key, value] of Object.entries(cfg?.vars ?? {})) values[key] = { value, source: parent, secret: (cfg?.secrets ?? []).includes(key) };
    }
    return values;
  });
  const overridden = $derived(Object.keys(vars).filter((key) => key in inherited));
  const envOverrides = $derived(Object.keys(vars).filter((key) => key in (store.collection?.environments[store.env ?? ""]?.vars ?? {})));

  async function save() {
    try {
      const next = structuredClone($state.snapshot(vars));
      await store.saveFolderVars(folder, next, secrets.filter((name) => name in next));
      void store.refreshMissingSecrets();
      store.modal = null;
    } catch (e) { error = String(e); }
  }
</script>

<Modal title={`Variables · ${folder}`} width={680} onclose={() => (store.modal = null)}>
  <p class="muted hint">Estas variables se aplican a las solicitudes de esta carpeta y sus subcarpetas. El entorno y las variables de ejecución tienen prioridad.</p>
  <KeyValue entries={vars} onchange={(value) => { vars = value; dirty = true; }} keyPlaceholder="variable" addLabel="Añadir variable" secretKeys={secrets} onSecretsChange={(keys) => { if (keys.join() !== secrets.join()) { secrets = keys; dirty = true; } }} />
  <p class="muted hint">Las variables secretas no se guardan en Git: solo su nombre. El valor queda en este equipo (o en la variable de sistema <code>HARPOON_NOMBRE</code>).</p>
  {#if overridden.length}<p class="hint">Sustituyen valores heredados: <span class="mono">{overridden.join(", ")}</span>.</p>{/if}
  {#if envOverrides.length}<p class="hint">El entorno activo sustituye: <span class="mono">{envOverrides.join(", ")}</span>.</p>{/if}
  {#if Object.keys(inherited).length}
    <h3>Variables heredadas</h3>
    <div class="inherited">
      {#each Object.entries(inherited) as [key, item]}
        <div>
          <span class="mono val">{key} = {item.secret && !revealed.includes(key) ? (item.value ? "••••••••" : "(sin valor)") : item.value}{#if item.secret}<button class="eye" aria-label={`${revealed.includes(key) ? "Ocultar" : "Mostrar"} ${key}`} aria-pressed={revealed.includes(key)} onclick={() => (revealed = revealed.includes(key) ? revealed.filter((k) => k !== key) : [...revealed, key])}><Icon name={revealed.includes(key) ? "eye-off" : "eye"} size={13} /></button>{/if}</span>
          <span class="muted">{item.secret ? "secreto · " : ""}{item.source}{key in vars ? " · sustituida" : ""}</span>
        </div>
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
  .val { display: inline-flex; align-items: center; gap: 4px; min-width: 0; }
  .eye { display: inline-flex; align-items: center; justify-content: center; width: 20px; height: 20px; border-radius: 4px; color: var(--text-muted); }
  .eye:hover { background: var(--bg-hover); color: var(--text); }
  code { font: 11px var(--mono); background: var(--bg-active); padding: 1px 5px; border-radius: 4px; }
</style>
