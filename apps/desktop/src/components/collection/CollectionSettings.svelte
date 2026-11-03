<script lang="ts">
  // Ajustes de la colección (harpoon.toml): nombre, auth, variables, conexión.
  import { store } from "../../state/store.svelte";
  import type { CollectionMeta } from "../../domain/types";
  import Modal from "../common/Modal.svelte";
  import AuthForm from "../request/AuthForm.svelte";
  import KeyValue from "../common/KeyValue.svelte";
  import CodeEditor from "../common/CodeEditor.svelte";

  let meta = $state<CollectionMeta>(structuredClone($state.snapshot(store.collection!.meta)));
  let tab = $state<"general" | "auth" | "vars" | "scripts" | "conn">("general");
  let dirty = $state(false);
  const touch = () => (dirty = true);

  async function save() {
    const next = structuredClone($state.snapshot(meta));
    next.secrets = (next.secrets ?? []).filter((name) => name in next.vars);
    await store.saveMeta(next);
    void store.refreshMissingSecrets();
    dirty = false;
    store.modal = null;
  }
</script>

<Modal title="Ajustes de la colección" width={680} onclose={() => (store.modal = null)}>
  <div class="tabs">
    <button class="tab" class:active={tab === "general"} onclick={() => (tab = "general")}>General</button>
    <button class="tab" class:active={tab === "auth"} onclick={() => (tab = "auth")}>Auth</button>
    <button class="tab" class:active={tab === "vars"} onclick={() => (tab = "vars")}>Variables</button>
    <button class="tab" class:active={tab === "scripts"} onclick={() => (tab = "scripts")}>Scripts</button>
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
      <KeyValue entries={meta.vars} onchange={(v) => { meta.vars = v; touch(); }} keyPlaceholder="variable" secretKeys={meta.secrets ?? []} onSecretsChange={(keys) => { if (keys.join() !== (meta.secrets ?? []).join()) { meta.secrets = keys; touch(); } }} />
      <p class="muted hint secret-hint">Las variables secretas no se guardan en Git: solo su nombre. El valor queda en este equipo (o en la variable de sistema <code>HARPOON_NOMBRE</code>).</p>
    {:else if tab === "scripts"}
      <p class="muted hint">Se ejecutan en todas las solicitudes, antes de los scripts propios de cada una.</p>
      {#each [["pre", "Pre-request"], ["post", "Post-response"], ["test", "Tests"]] as [k, label]}
        <div class="script">
          <div class="muted lbl">{label}</div>
          <div class="editor-wrap">
            <CodeEditor value={meta.scripts[k as "pre" | "post" | "test"]} language="javascript" onchange={(v) => { meta.scripts = { ...meta.scripts, [k]: v }; touch(); }} />
          </div>
        </div>
      {/each}
    {:else}
      <div class="form">
        <label><span>Timeout (ms)</span><input class="mono" type="number" bind:value={meta.settings.timeout_ms} oninput={touch} /></label>
        <label><span>Seguir redirecciones</span><input type="checkbox" bind:checked={meta.settings.follow_redirects} onchange={touch} /></label>
        <label><span>Proxy</span><input class="mono" value={meta.settings.proxy ?? ""} oninput={(e) => { meta.settings.proxy = e.currentTarget.value || undefined; touch(); }} placeholder="http://127.0.0.1:8888 (vacío = HTTP_PROXY del sistema)" /></label>
        <label><span>Ignorar TLS inválido</span><input type="checkbox" bind:checked={meta.settings.insecure} onchange={touch} /></label>
        <div class="sub">Certificados (rutas relativas a la colección, formato PEM)</div>
        <label><span>CA adicional</span><input class="mono" value={meta.settings.ca_cert ?? ""} oninput={(e) => { meta.settings.ca_cert = e.currentTarget.value || undefined; touch(); }} placeholder="certs/ca.pem" /></label>
        <label><span>Certificado de cliente</span><input class="mono" value={meta.settings.client_cert ?? ""} oninput={(e) => { meta.settings.client_cert = e.currentTarget.value || undefined; touch(); }} placeholder="certs/client.pem (mTLS)" /></label>
        <label><span>Clave privada</span><input class="mono" value={meta.settings.client_key ?? ""} oninput={(e) => { meta.settings.client_key = e.currentTarget.value || undefined; touch(); }} placeholder="certs/client-key.pem (si no va en el .pem anterior)" /></label>
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
  .secret-hint { margin-top: 10px; }
  code { font: 11px var(--mono); background: var(--bg-active); padding: 1px 5px; border-radius: 4px; }
  .sub {
    color: var(--text-muted);
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    margin-top: 8px;
  }
  .script {
    margin-bottom: 10px;
  }
  .editor-wrap {
    height: 110px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
    background: var(--bg-raised);
    margin-top: 4px;
  }
</style>
