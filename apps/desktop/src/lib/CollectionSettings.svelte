<script lang="ts">
  // Ajustes de la colección (harpoon.toml): nombre, auth, variables, conexión.
  import { store } from "./store.svelte";
  import type { CollectionMeta } from "./types";
  import Modal from "./Modal.svelte";
  import AuthForm from "./AuthForm.svelte";
  import KeyValue from "./KeyValue.svelte";

  let meta = $state<CollectionMeta>(structuredClone($state.snapshot(store.collection!.meta)));
  let tab = $state<"general" | "auth" | "vars" | "conn">("general");
  let dirty = $state(false);
  const touch = () => (dirty = true);

  async function save() {
    await store.saveMeta(structuredClone($state.snapshot(meta)));
    dirty = false;
    store.modal = null;
  }
</script>

<Modal title="Ajustes de la colección" width={680} onclose={() => (store.modal = null)}>
  <div class="tabs">
    <button class="tab" class:active={tab === "general"} onclick={() => (tab = "general")}>General</button>
    <button class="tab" class:active={tab === "auth"} onclick={() => (tab = "auth")}>Auth</button>
    <button class="tab" class:active={tab === "vars"} onclick={() => (tab = "vars")}>Variables</button>
    <button class="tab" class:active={tab === "conn"} onclick={() => (tab = "conn")}>Conexión</button>
  </div>
  <div class="content">
    {#if tab === "general"}
      <div class="form">
        <label><span>Nombre</span><input bind:value={meta.name} oninput={touch} /></label>
        <label>
          <span>Entorno por defecto</span>
          <select value={meta.default_environment ?? ""} onchange={(e) => { meta.default_environment = e.currentTarget.value || null; touch(); }}>
            <option value="">— ninguno —</option>
            {#each Object.keys(store.collection?.environments ?? {}) as n}<option value={n}>{n}</option>{/each}
          </select>
        </label>
        <div class="rowinfo"><span class="muted lbl">Carpeta</span><span class="mono muted path">{store.collection?.root}</span></div>
      </div>
    {:else if tab === "auth"}
      <AuthForm auth={meta.auth} allowInherit={false} onchange={(a) => { meta.auth = a; touch(); }} />
    {:else if tab === "vars"}
      <p class="muted hint">Variables comunes a todos los entornos (los entornos las pueden sobrescribir).</p>
      <KeyValue entries={meta.vars} onchange={(v) => { meta.vars = v; touch(); }} keyPlaceholder="variable" />
    {:else}
      <div class="form">
        <label><span>Timeout (ms)</span><input class="mono" type="number" bind:value={meta.settings.timeout_ms} oninput={touch} /></label>
        <label><span>Seguir redirecciones</span><input type="checkbox" bind:checked={meta.settings.follow_redirects} onchange={touch} /></label>
        <label><span>Proxy</span><input class="mono" value={meta.settings.proxy ?? ""} oninput={(e) => { meta.settings.proxy = e.currentTarget.value || undefined; touch(); }} placeholder="http://127.0.0.1:8888 (vacío = HTTP_PROXY del sistema)" /></label>
        <label><span>Ignorar TLS inválido</span><input type="checkbox" bind:checked={meta.settings.insecure} onchange={touch} /></label>
      </div>
    {/if}
  </div>
  {#snippet footer()}
    <button class="btn" onclick={() => (store.modal = null)}>Cancelar</button>
    <button class="btn primary" onclick={save} disabled={!dirty}>Guardar</button>
  {/snippet}
</Modal>

<style>
  .tabs {
    margin: -14px -16px 12px;
  }
  .content {
    min-height: 260px;
  }
  .form {
    display: flex;
    flex-direction: column;
    gap: 10px;
    max-width: 560px;
  }
  label {
    display: grid;
    grid-template-columns: 160px 1fr;
    align-items: center;
    gap: 10px;
  }
  label > span:first-child {
    color: var(--text-muted);
    font-size: 12px;
  }
  input:not([type="checkbox"]),
  select {
    font-size: 12px;
  }
  input[type="checkbox"] {
    justify-self: start;
  }
  .rowinfo {
    display: grid;
    grid-template-columns: 160px 1fr;
    gap: 10px;
    align-items: center;
  }
  .lbl {
    font-size: 12px;
  }
  .path {
    font-size: 11.5px;
    word-break: break-all;
  }
  .hint {
    font-size: 12px;
    margin: 0 0 8px;
  }
</style>
