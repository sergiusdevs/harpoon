<script lang="ts">
  // Configuración de una carpeta (folder.toml): variables, cabeceras, auth y
  // scripts que heredan sus solicitudes y subcarpetas.
  import { store, folderChain } from "../../state/store.svelte";
  import { emptyFolderConfig, type Auth, type Scripts } from "../../domain/types";
  import { inheritedAuth, inheritedHeaders } from "../../domain/folderInheritance";
  import Modal from "../common/Modal.svelte";
  import KeyValue from "../common/KeyValue.svelte";
  import Icon from "../common/Icon.svelte";
  import AuthForm from "../request/AuthForm.svelte";
  import CodeEditor from "../common/CodeEditor.svelte";

  const folder = store.editingFolder;
  const initial = structuredClone($state.snapshot({ ...emptyFolderConfig(), ...store.collection!.folder_configs[folder] }));
  let tab = $state<"vars" | "headers" | "auth" | "scripts">("vars");
  let vars = $state<Record<string, string>>(initial.vars);
  let secrets = $state<string[]>([...initial.secrets]);
  let headers = $state<Record<string, string>>(initial.headers);
  let auth = $state<Auth>(initial.auth);
  let scripts = $state<Scripts>(initial.scripts);
  let varsDirty = $state(false);
  let settingsDirty = $state(false);
  let revealed = $state<string[]>([]);
  let error = $state("");
  const parents = folderChain(folder);

  const inherited = $derived.by(() => {
    const values: Record<string, { value: string; source: string; secret: boolean }> = {};
    const colSecrets = store.collection!.meta.secrets ?? [];
    for (const [key, value] of Object.entries(store.collection!.meta.vars)) values[key] = { value, source: "Colección", secret: colSecrets.includes(key) };
    for (const parent of parents) {
      const cfg = store.collection!.folder_configs[parent];
      for (const [key, value] of Object.entries(cfg?.vars ?? {})) values[key] = { value, source: parent, secret: (cfg?.secrets ?? []).includes(key) };
    }
    return values;
  });
  const overridden = $derived(Object.keys(vars).filter((key) => key in inherited));
  const envOverrides = $derived(Object.keys(vars).filter((key) => key in (store.collection?.environments[store.env ?? ""]?.vars ?? {})));
  const parentHeaders = $derived(inheritedHeaders(store.collection!, parents));
  const parentAuth = $derived(inheritedAuth(store.collection!, parents));
  const counts = $derived({
    vars: Object.keys(vars).length,
    headers: Object.keys(headers).length,
    auth: auth.type !== "inherit" ? 1 : 0,
    scripts: (["pre", "post", "test"] as const).filter((k) => scripts[k].trim()).length,
  });

  async function save() {
    try {
      if (varsDirty) {
        const next = structuredClone($state.snapshot(vars));
        await store.saveFolderVars(folder, next, secrets.filter((name) => name in next));
      }
      if (settingsDirty) await store.saveFolderSettings(folder, structuredClone($state.snapshot({ headers, auth, scripts })));
      store.status = { kind: "ok", text: `Guardado ${folder}/folder.toml` };
      void store.refreshMissingSecrets();
      store.modal = null;
    } catch (e) { error = String(e); }
  }
</script>

<Modal title={`Carpeta · ${folder}`} width={720} onclose={() => (store.modal = null)}>
  <div class="tabs" role="tablist">
    {#each [["vars", "Variables"], ["headers", "Headers"], ["auth", "Auth"], ["scripts", "Scripts"]] as [id, label]}
      <button class="tab" role="tab" aria-selected={tab === id} class:active={tab === id} onclick={() => (tab = id as typeof tab)}>
        {label}{#if counts[id as keyof typeof counts]}<span class="count">{counts[id as keyof typeof counts]}</span>{/if}
      </button>
    {/each}
  </div>
  <p class="muted hint lead">Se aplica a las solicitudes de <span class="mono">{folder}/</span> y sus subcarpetas. Lo que defina cada solicitud tiene prioridad.</p>

  <div class="content">
    {#if tab === "vars"}
      <KeyValue entries={vars} onchange={(value) => { vars = value; varsDirty = true; }} keyPlaceholder="variable" addLabel="Añadir variable" secretKeys={secrets} onSecretsChange={(keys) => { if (keys.join() !== secrets.join()) { secrets = keys; varsDirty = true; } }} />
      <p class="muted hint">Las variables secretas no se guardan en Git: solo su nombre. El valor queda en este equipo (o en la variable de sistema <code>HARPOON_NOMBRE</code>). El entorno y las variables de ejecución tienen prioridad.</p>
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
    {:else if tab === "headers"}
      <KeyValue entries={headers} onchange={(value) => { headers = value; settingsDirty = true; }} caseInsensitive keyPlaceholder="Cabecera" addLabel="Añadir cabecera" secretLocation={(name) => `header ${name}`} />
      <p class="muted hint">Una solicitud puede sustituir una cabecera definiendo la suya o desactivarla en su pestaña Headers.</p>
      {#if parentHeaders.length}
        <h3>Heredadas de carpetas superiores</h3>
        <div class="inherited">
          {#each parentHeaders as h (h.name)}
            <div><span class="mono val">{h.name}: {h.value}</span><span class="muted">{h.source}{Object.keys(headers).some((k) => k.toLowerCase() === h.name.toLowerCase()) ? " · sustituida" : ""}</span></div>
          {/each}
        </div>
      {/if}
    {:else if tab === "auth"}
      <AuthForm {auth} onchange={(value) => { auth = value; settingsDirty = true; }} inheritHint={`Usa la auth de ${parentAuth.source} (${parentAuth.auth.type}).`} />
    {:else}
      <p class="muted hint">Orden de ejecución: colección → carpetas (de fuera hacia dentro) → solicitud.</p>
      {#each [["pre", "Pre-request"], ["post", "Post-response"], ["test", "Tests"]] as [k, label]}
        <div class="script">
          <div class="muted lbl">{label}</div>
          <div class="editor-wrap">
            <CodeEditor value={scripts[k as keyof Scripts]} language="javascript" label={`Script ${label}`} onchange={(v) => { scripts = { ...scripts, [k]: v }; settingsDirty = true; }} />
          </div>
        </div>
      {/each}
    {/if}
  </div>
  {#if error}<p class="error" role="alert">{error}</p>{/if}
  {#snippet footer()}
    <button class="btn" onclick={() => (store.modal = null)}>Cancelar</button>
    <button class="btn primary" onclick={save} disabled={!varsDirty && !settingsDirty}>Guardar</button>
  {/snippet}
</Modal>

<style>
  .tabs { margin: -14px -16px 12px; }
  .count { font-size: 10px; background: var(--bg-active); color: var(--text-muted); border-radius: 10px; padding: 0 6px; }
  .content { min-height: 280px; }
  .hint { font-size: 12px; }
  .lead { margin: 0 0 12px; }
  h3 { margin: 20px 0 8px; font-size: 12px; }
  .inherited { max-height: 180px; overflow: auto; border-top: 1px solid var(--border); }
  .inherited div { display: flex; justify-content: space-between; gap: 12px; padding: 7px 4px; border-bottom: 1px solid var(--border); font-size: 12px; }
  .error { color: var(--err); }
  .val { display: inline-flex; align-items: center; gap: 4px; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .eye { display: inline-flex; align-items: center; justify-content: center; width: 20px; height: 20px; border-radius: 4px; color: var(--text-muted); }
  .eye:hover { background: var(--bg-hover); color: var(--text); }
  code { font: 11px var(--mono); background: var(--bg-active); padding: 1px 5px; border-radius: 4px; }
  .script { margin-bottom: 10px; }
  .lbl { font-size: 12px; margin-bottom: 4px; }
  .editor-wrap { height: 120px; border: 1px solid var(--border-strong); border-radius: 7px; overflow: hidden; }
</style>
