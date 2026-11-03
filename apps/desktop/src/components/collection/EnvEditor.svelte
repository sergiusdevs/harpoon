<script lang="ts">
  // Entornos de la colección: lista a la izquierda y, a la
  // derecha, pestañas Variables / Secretos. Los secretos solo guardan su
  // nombre en Git; el valor queda en este equipo (`<env>.secrets.toml`).
  import { store, slug } from "../../state/store.svelte";
  import type { Environment } from "../../domain/types";
  import type { EnvironmentPreview } from "../../services/api";
  import Modal from "../common/Modal.svelte";

  type Row = { k: string; v: string; d: string; secret: boolean; show: boolean };
  type Tab = "vars" | "secrets";
  const blank = (secret: boolean): Row => ({ k: "", v: "", d: "", secret, show: false });
  const isBlank = (r: Row) => !r.k && !r.v && !r.d;

  const names = $derived(Object.keys(store.collection?.environments ?? {}).sort((a, b) => a.localeCompare(b)));
  let current = $state<string | null>(null);
  let rows = $state<Row[]>([]);
  let tab = $state<Tab>("vars");
  let dirty = $state(false);
  let busy = $state(false);
  let error = $state("");
  let search = $state("");
  let variableSearch = $state("");
  let searchingVars = $state(false);
  let creating = $state(false);
  let newName = $state("");
  let renaming = $state(false);
  let renameTo = $state("");
  let exportMenu = $state(false);
  let importing = $state<{ path: string; preview: EnvironmentPreview; name: string } | null>(null);

  const filtered = $derived(names.filter((name) => name.toLowerCase().includes(search.toLowerCase())));
  const counts = $derived({ vars: rows.filter((r) => !r.secret && !isBlank(r)).length, secrets: rows.filter((r) => r.secret && !isBlank(r)).length });
  const visibleRows = $derived(
    rows
      .map((row, index) => ({ row, index }))
      .filter(({ row }) => row.secret === (tab === "secrets"))
      .filter(({ row }) => isBlank(row) || !variableSearch || `${row.k} ${row.d}`.toLowerCase().includes(variableSearch.toLowerCase()))
      .sort((a, b) => Number(isBlank(a.row)) - Number(isBlank(b.row))),
  );

  $effect(() => { if (!current && names.length) load(store.env && names.includes(store.env) ? store.env : names[0]); });

  function ensureBlanks() {
    for (const secret of [false, true]) if (!rows.some((r) => r.secret === secret && isBlank(r))) rows.push(blank(secret));
  }
  function load(name: string) {
    const env = store.collection?.environments[name]; if (!env) return;
    current = name; variableSearch = ""; error = ""; renaming = false; importing = null;
    rows = Object.entries(env.vars).map(([k, v]) => ({ k, v, d: env.descriptions?.[k] ?? "", secret: env.secrets.includes(k), show: false }));
    // Secretos declarados sin valor en este equipo: también se listan.
    for (const s of env.secrets) if (!(s in env.vars)) rows.push({ k: s, v: "", d: env.descriptions?.[s] ?? "", secret: true, show: false });
    ensureBlanks();
    dirty = false;
  }
  function guard(action: () => void | Promise<void>) {
    if (busy) return;
    if (!dirty) { void action(); return; }
    store.dialog = { kind: "confirm", title: "Cambios del entorno sin guardar", text: "Descarta los cambios de este entorno para continuar, o cancela y guárdalos.", ok: "Descartar cambios", onOk: async () => { dirty = false; await action(); } };
  }
  function edit(index: number, patch: Partial<Row>) {
    Object.assign(rows[index], patch);
    ensureBlanks();
    dirty = true; error = "";
  }
  function toggleSecret(index: number) {
    rows[index].secret = !rows[index].secret; rows[index].show = false;
    ensureBlanks(); dirty = true;
  }
  function remove(index: number) { rows.splice(index, 1); ensureBlanks(); dirty = true; }

  function collect(): Environment {
    const env: Environment = { vars: {}, secrets: [], descriptions: {} };
    for (const row of rows) {
      const key = row.k.trim(); if (!key) continue;
      if (key in env.vars) throw new Error(`La variable ${key} está repetida.`);
      env.vars[key] = row.v; if (row.d.trim()) env.descriptions![key] = row.d;
      if (row.secret) env.secrets.push(key);
    }
    return env;
  }
  async function save() {
    if (!current || busy) return;
    busy = true; error = "";
    try { await store.saveEnvironment(current, collect()); load(current); void store.refreshMissingSecrets(); }
    catch (cause) { error = cause instanceof Error ? cause.message : String(cause); }
    finally { busy = false; }
  }
  async function act(fn: () => Promise<void>) {
    busy = true; error = "";
    try { await fn(); } catch (cause) { error = cause instanceof Error ? cause.message : String(cause); } finally { busy = false; }
  }
  function create() {
    const name = slug(newName); if (!name) return;
    if (names.includes(name)) { error = `Ya existe el entorno ${name}.`; return; }
    guard(() => act(async () => { await store.saveEnvironment(name, { vars: { base_url: "" }, secrets: [] }); search = ""; newName = ""; creating = false; load(name); }));
  }
  function startRename() { if (!current) return; renaming = true; renameTo = current; }
  function rename() {
    const from = current, to = slug(renameTo);
    if (!from || !to || to === from) { renaming = false; return; }
    if (names.includes(to)) { error = `Ya existe el entorno ${to}.`; return; }
    void act(async () => {
      await store.saveEnvironment(to, collect());
      if (store.collection?.meta.default_environment === from) await store.saveMeta({ ...$state.snapshot(store.collection.meta), default_environment: to });
      await store.deleteEnvironment(from);
      if (store.env === from || !store.env) store.env = to;
      dirty = false; renaming = false; load(to);
    });
  }
  function duplicate() {
    if (!current) return;
    let name = `${current}-copia`, i = 2;
    while (names.includes(name)) name = `${current}-copia-${i++}`;
    guard(() => act(async () => { await store.saveEnvironment(name, collect()); load(name); }));
  }
  function del() {
    if (!current) return;
    const name = current;
    store.dialog = { kind: "confirm", title: "Borrar entorno", text: `Se eliminarán ${name}.toml, su archivo de secretos y los cambios sin guardar.`, ok: "Borrar", danger: true, onOk: async () => { await store.deleteEnvironment(name); dirty = false; current = null; rows = []; } };
  }
  function use(name: string) { store.env = name; void store.refreshMissingSecrets(); }
  function setDefault() {
    if (!store.collection || !current) return;
    const name = current;
    void act(() => store.saveMeta({ ...$state.snapshot(store.collection!.meta), default_environment: name }));
  }
  function exportAs(format: "postman" | "harpoon") {
    exportMenu = false;
    if (!current) return;
    const name = current;
    guard(() => act(() => store.exportEnvironment(name, format)));
  }
  function startImport() {
    guard(() => act(async () => {
      const picked = await store.pickEnvironmentFile();
      if (!picked) return;
      let name = slug(picked.preview.name) || "importado", i = 2;
      while (names.includes(name)) name = `${slug(picked.preview.name) || "importado"}-${i++}`;
      importing = { ...picked, name };
    }));
  }
  function confirmImport() {
    if (!importing) return;
    const { path } = importing, name = slug(importing.name);
    if (!name) { error = "Pon un nombre al entorno."; return; }
    if (names.includes(name)) { error = `Ya existe el entorno ${name}.`; return; }
    void act(async () => { await store.importEnvironment(path, name); importing = null; load(name); });
  }
</script>

<svelte:window onclick={() => (exportMenu = false)} />

<Modal title="Entornos" width={1080} onclose={() => guard(() => { store.modal = null; })}>
  <div class="layout">
    <aside>
      <div class="list-heading">
        <span class="label">Entornos</span>
        <div class="list-actions">
          <button class="ico" title="Nuevo entorno" aria-label="Nuevo entorno" onclick={() => { creating = !creating; newName = ""; }} disabled={busy}>
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" aria-hidden="true"><path d="M12 5v14M5 12h14"/></svg>
          </button>
          <button class="ico" title="Importar entorno (Postman o Harpoon)" aria-label="Importar entorno" onclick={startImport} disabled={busy}>
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M12 4v11m-4-4 4 4 4-4M4 17v3h16v-3"/></svg>
          </button>
          <div class="menu-anchor">
            <button class="ico" title="Exportar entorno" aria-label="Exportar entorno" aria-haspopup="menu" aria-expanded={exportMenu} onclick={(e) => { e.stopPropagation(); exportMenu = !exportMenu; }} disabled={busy || !current}>
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M12 15V4m-4 4 4-4 4 4M4 17v3h16v-3"/></svg>
            </button>
            {#if exportMenu}
              <div class="menu fade-in" role="menu">
                <button role="menuitem" onclick={() => exportAs("postman")}>Postman (JSON)</button>
                <button role="menuitem" onclick={() => exportAs("harpoon")}>Harpoon (TOML)</button>
                <p>Los valores de los secretos nunca se exportan: salen vacíos.</p>
              </div>
            {/if}
          </div>
        </div>
      </div>
      {#if creating}
        <input class="field" aria-label="Nombre del nuevo entorno" placeholder="Nombre del entorno" bind:value={newName} onkeydown={(e) => { if (e.key === "Enter") create(); if (e.key === "Escape") creating = false; }} disabled={busy} spellcheck="false" />
      {/if}
      <label class="search">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true"><circle cx="11" cy="11" r="7"/><path d="m16 16 4 4"/></svg>
        <input aria-label="Buscar entornos" placeholder="Buscar entornos…" bind:value={search} spellcheck="false" />
      </label>
      <div class="env-list">
        {#each filtered as name}
          <button class="env" class:active={name === current} onclick={() => guard(() => load(name))} ondblclick={() => use(name)} disabled={busy} title={store.env === name ? "Entorno activo" : "Doble clic para usarlo"}>
            <span class="env-label">{name}</span>
            {#if store.collection?.meta.default_environment === name}<span class="tag">predet.</span>{/if}
            {#if store.env === name}<svg class="check" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" aria-label="Entorno activo"><path d="m5 12 5 5 9-10"/></svg>{/if}
          </button>
        {/each}
        {#if !filtered.length}<p class="muted hint">{names.length ? "No hay entornos que coincidan." : "Aún no hay entornos."}</p>{/if}
      </div>
    </aside>

    <section aria-label="Editor del entorno">
      {#if importing}
        <div class="import fade-in">
          <h4>Importar entorno</h4>
          <p class="muted">{importing.preview.variables} variables · {importing.preview.secrets.length} secretas{importing.preview.secrets.length ? ` (${importing.preview.secrets.join(", ")})` : ""}</p>
          <label>Nombre<input class="field" bind:value={importing.name} spellcheck="false" onkeydown={(e) => e.key === "Enter" && confirmImport()} /></label>
          {#each importing.preview.warnings as warning}<p class="warning">{warning}</p>{/each}
          {#if error}<p class="error" role="alert">{error}</p>{/if}
          <div class="save-actions"><button class="btn primary" onclick={confirmImport} disabled={busy}>Importar</button><button class="reset" onclick={() => { importing = null; error = ""; }}>Cancelar</button></div>
        </div>
      {:else if current}
        <div class="head">
          <div class="title">
            {#if renaming}
              <!-- svelte-ignore a11y_autofocus -->
              <input class="field rename" bind:value={renameTo} autofocus onkeydown={(e) => { if (e.key === "Enter") rename(); if (e.key === "Escape") renaming = false; }} onblur={rename} spellcheck="false" aria-label="Nuevo nombre del entorno" />
            {:else}
              <h4>{current}</h4>
              <button class="ico" title="Renombrar" aria-label="Renombrar entorno" onclick={startRename} disabled={busy}><svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M12 20h9"/><path d="M16.5 3.5a2.1 2.1 0 0 1 3 3L7 19l-4 1 1-4z"/></svg></button>
            {/if}
            {#if store.env === current}<span class="pill">activo</span>{:else}<button class="link" onclick={() => use(current!)} disabled={busy}>Usar este entorno</button>{/if}
            {#if store.collection?.meta.default_environment !== current}<button class="link" onclick={setDefault} disabled={busy}>Hacer predeterminado</button>{/if}
          </div>
          <div class="actions">
            <button class="ico" title="Guardar (⌘S)" aria-label="Guardar entorno" onclick={save} disabled={!dirty || busy}><svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M5 3h12l4 4v14H3V3h2z"/><path d="M7 3v6h10V4M7 21v-8h10v8"/></svg></button>
            <button class="ico" title="Renombrar" aria-label="Renombrar entorno" onclick={startRename} disabled={busy}><svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M4 20h4L19 9a2.8 2.8 0 0 0-4-4L4 16z"/><path d="m13.5 6.5 4 4"/></svg></button>
            <button class="ico" title="Duplicar" aria-label="Duplicar entorno" onclick={duplicate} disabled={busy}><svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><rect x="9" y="9" width="12" height="12" rx="2"/><path d="M5 15H4a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1h10a1 1 0 0 1 1 1v1"/></svg></button>
            <button class="ico danger" title="Borrar" aria-label="Borrar entorno" onclick={del} disabled={busy}><svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M4 7h16M9 7V4h6v3M6 7l1 13h10l1-13M10 11v6M14 11v6"/></svg></button>
          </div>
        </div>

        <div class="tabs">
          <div role="tablist">
            <button role="tab" class:active={tab === "vars"} aria-selected={tab === "vars"} onclick={() => (tab = "vars")}>Variables{#if counts.vars}<sup>{counts.vars}</sup>{/if}</button>
            <button role="tab" class:active={tab === "secrets"} aria-selected={tab === "secrets"} onclick={() => (tab = "secrets")}>Secretos{#if counts.secrets}<sup>{counts.secrets}</sup>{/if}</button>
          </div>
          {#if searchingVars}
            <!-- svelte-ignore a11y_autofocus -->
            <input class="field var-search" aria-label="Buscar variables" placeholder="Buscar variable…" bind:value={variableSearch} autofocus onkeydown={(e) => e.key === "Escape" && ((variableSearch = ""), (searchingVars = false))} />
          {/if}
          <button class="ico" title="Buscar variable" aria-label="Buscar variable" onclick={() => { searchingVars = !searchingVars; if (!searchingVars) variableSearch = ""; }}><svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" aria-hidden="true"><circle cx="11" cy="11" r="7"/><path d="m16 16 4 4"/></svg></button>
        </div>

        <div class="table-frame"><table><thead><tr><th scope="col">Nombre</th><th scope="col">Valor</th><th scope="col">Descripción</th><th class="row-actions" scope="col"><span class="sr-only">Acciones</span></th></tr></thead><tbody>
          {#each visibleRows as { row, index } (row)}
            <tr>
              <td><input aria-label={`Nombre de variable ${index + 1}`} value={row.k} placeholder="Nombre" oninput={(e) => edit(index, { k: e.currentTarget.value })} disabled={busy} spellcheck="false" /></td>
              <td><div class="value-cell">
                <input aria-label={`Valor de ${row.k || `variable ${index + 1}`}`} class:mono={row.secret && !row.show && !!row.v} type={row.secret && !row.show ? "password" : "text"} value={row.v} placeholder={row.secret ? "Valor (solo en este equipo)" : "Valor"} oninput={(e) => edit(index, { v: e.currentTarget.value })} disabled={busy} spellcheck="false" autocomplete="off" />
                {#if row.secret && !isBlank(row)}<button class="ico small" aria-label={`${row.show ? "Ocultar" : "Mostrar"} valor de ${row.k}`} aria-pressed={row.show} onclick={() => (row.show = !row.show)}><svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" aria-hidden="true"><path d="M2 12s3-7 10-7 10 7 10 7-3 7-10 7S2 12 2 12z"/><circle cx="12" cy="12" r="3"/>{#if row.show}<path d="m3 3 18 18"/>{/if}</svg></button>{/if}
              </div></td>
              <td><input aria-label={`Descripción de ${row.k || `variable ${index + 1}`}`} value={row.d} placeholder="Descripción" oninput={(e) => edit(index, { d: e.currentTarget.value })} disabled={busy} /></td>
              <td class="row-actions">{#if !isBlank(row)}
                <button class="ico small" title={row.secret ? "Hacer pública (su valor irá a Git)" : "Marcar como secreta"} aria-label={row.secret ? `Hacer pública ${row.k}` : `Marcar ${row.k} como secreta`} onclick={() => toggleSecret(index)} disabled={busy}>
                  {#if row.secret}<svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><rect x="5" y="11" width="14" height="9" rx="2"/><path d="M8 11V8a4 4 0 0 1 7.5-2"/></svg>{:else}<svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><rect x="5" y="11" width="14" height="9" rx="2"/><path d="M8 11V8a4 4 0 0 1 8 0v3"/></svg>{/if}
                </button>
                <button class="ico small danger" title="Eliminar" aria-label={`Eliminar variable ${row.k}`} onclick={() => remove(index)} disabled={busy}><svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" aria-hidden="true"><path d="M4 7h16M9 7V4h6v3M6 7l1 13h10l1-13M10 11v6M14 11v6"/></svg></button>
              {/if}</td>
            </tr>
          {/each}
        </tbody></table></div>
        {#if tab === "secrets"}
          <p class="hint muted">Solo el nombre va a Git. El valor se guarda en este equipo (<code>environments/{current}.secrets.toml</code>, ignorado por Git) o en la variable de sistema <code>HARPOON_&lt;NOMBRE&gt;</code>.</p>
        {:else}
          <p class="hint muted">Estas variables se guardan en <code>environments/{current}.toml</code> y van a Git. Para un token o una contraseña usa el candado: pasa a Secretos.</p>
        {/if}
        {#if error}<p class="error" role="alert">{error}</p>{/if}
        <div class="save-actions"><button class="btn primary" onclick={save} disabled={!dirty || busy}>Guardar</button><button class="reset" onclick={() => load(current!)} disabled={!dirty || busy}>Restablecer</button>{#if dirty}<span class="muted small">Cambios sin guardar</span>{/if}</div>
      {:else}
        <div class="empty"><p class="muted">Crea un entorno con <b>+</b> o importa uno de Postman.</p>{#if error}<p class="error" role="alert">{error}</p>{/if}</div>
      {/if}
    </section>
  </div>
</Modal>

<style>
  .layout { display: grid; grid-template-columns: 250px minmax(0, 1fr); min-height: 420px; margin: -4px 0 0; }
  aside { display: flex; flex-direction: column; gap: 10px; border-right: 1px solid var(--border); padding: 4px 18px 4px 0; }
  .list-heading { display: flex; align-items: center; justify-content: space-between; }
  .label { font-size: 12px; font-weight: 700; letter-spacing: .07em; text-transform: uppercase; color: var(--text); }
  .list-actions { display: flex; gap: 2px; }
  .ico { display: inline-flex; align-items: center; justify-content: center; width: 30px; height: 28px; border-radius: 6px; color: var(--text-muted); flex: 0 0 auto; }
  .ico:hover:not(:disabled) { color: var(--text); background: var(--bg-hover); }
  .ico:disabled { opacity: .4; }
  .ico.small { width: 26px; height: 26px; }
  .ico.danger:hover:not(:disabled) { color: var(--err); }
  .menu-anchor { position: relative; }
  .menu { position: absolute; right: 0; top: 32px; z-index: 5; width: 230px; display: flex; flex-direction: column; padding: 5px; background: var(--bg-raised); border: 1px solid var(--border-strong); border-radius: 8px; box-shadow: var(--shadow-pop); animation: pop-in 160ms cubic-bezier(.2, .7, .3, 1) backwards; transform-origin: top right; }
  .menu button { text-align: left; padding: 8px 10px; border-radius: 5px; font-size: 13px; }
  .menu button:hover { background: var(--bg-hover); }
  .menu p { margin: 4px 10px 6px; font-size: 11px; color: var(--text-faint); line-height: 1.4; }
  .field { width: 100%; min-width: 0; font-size: 13px; padding: 7px 10px; border: 1px solid var(--border-strong); border-radius: 6px; background: var(--bg); }
  .search { display: flex; align-items: center; gap: 8px; padding: 0 10px; border: 1px solid var(--border-strong); border-radius: 7px; color: var(--text-faint); }
  .search:focus-within { border-color: var(--accent); }
  .search input { flex: 1; min-width: 0; border: 0; background: transparent; padding: 8px 0; font-size: 13px; outline: none; }
  .env-list { flex: 1; min-height: 0; overflow: auto; display: flex; flex-direction: column; gap: 3px; }
  .env { display: flex; align-items: center; gap: 8px; padding: 10px 12px; border-radius: 7px; text-align: left; font-size: 14px; }
  .env:hover { background: var(--bg-hover); }
  .env.active { background: var(--bg-active); }
  .env-label { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .check { color: var(--ok); }
  .tag { font-size: 10px; color: var(--text-faint); }
  section { min-width: 0; padding: 4px 0 4px 22px; display: flex; flex-direction: column; }
  .head { display: flex; align-items: center; justify-content: space-between; gap: 12px; margin-bottom: 16px; }
  .title { display: flex; align-items: center; gap: 8px; min-width: 0; flex-wrap: wrap; }
  h4 { margin: 0; font-size: 16px; font-weight: 600; }
  .rename { width: 220px; }
  .pill { font-size: 10.5px; padding: 1px 7px; border-radius: 9px; color: var(--ok); background: color-mix(in srgb, var(--ok) 15%, transparent); }
  .link { font-size: 12px; color: var(--accent); }
  .link:hover { text-decoration: underline; }
  .actions { display: flex; gap: 2px; }
  .tabs { display: flex; align-items: center; gap: 10px; margin-bottom: 14px; }
  .tabs [role="tablist"] { flex: 1; display: flex; gap: 22px; }
  .tabs [role="tab"] { padding: 6px 0 8px; font-size: 14px; color: var(--text-faint); border-bottom: 2px solid transparent; border-radius: 0; }
  .tabs [role="tab"]:hover { color: var(--text-muted); }
  .tabs [role="tab"].active { color: var(--text); border-bottom-color: var(--accent); }
  sup { font-size: 10px; margin-left: 3px; color: var(--text-faint); }
  .tabs [role="tab"].active sup { color: var(--accent); }
  .var-search { width: 200px; padding: 5px 9px; font-size: 12px; }
  .table-frame { border: 1px solid var(--border-strong); border-radius: 8px; overflow: hidden; }
  table { width: 100%; table-layout: fixed; border-collapse: collapse; font-size: 13px; }
  th { padding: 11px 12px; font-weight: 600; text-align: left; border-bottom: 1px solid var(--border); }
  th:first-child { width: 28%; }
  th:nth-child(3) { width: 30%; }
  td { padding: 3px 4px; height: 42px; border-top: 1px solid var(--border); }
  tbody tr:first-child td { border-top: 0; }
  th + th, td + td { border-left: 1px solid var(--border); }
  td input { width: 100%; min-width: 0; border: 1px solid transparent; background: transparent; padding: 7px 8px; font-size: 13px; }
  td:focus-within { background: var(--bg-hover); }
  td input:hover, td input:focus, td input:focus-visible { border-color: transparent; box-shadow: none; outline: none; }
  .value-cell { display: flex; align-items: center; min-width: 0; }
  .value-cell input { flex: 1; min-width: 0; }
  .row-actions { width: 66px; text-align: center; white-space: nowrap; }
  tr:not(:hover) .row-actions .ico { opacity: .55; }
  .hint { font-size: 11.5px; line-height: 1.6; margin: 12px 0 0; }
  code { font: 11px var(--mono); }
  .error { color: var(--err); font-size: 12px; margin: 10px 0 0; }
  .warning { color: var(--warn); font-size: 12px; margin: 0; }
  .save-actions { display: flex; align-items: center; gap: 18px; margin-top: 18px; }
  .reset { color: var(--accent); font-size: 13px; }
  .reset:disabled { color: var(--text-faint); }
  .small { font-size: 11.5px; }
  .import { display: flex; flex-direction: column; gap: 12px; max-width: 520px; }
  .import h4 { margin-bottom: 2px; }
  .import p { margin: 0; font-size: 12.5px; }
  .import label { display: flex; flex-direction: column; gap: 6px; font-size: 12px; font-weight: 600; }
  .empty { display: flex; flex-direction: column; justify-content: center; align-items: center; flex: 1; }
  .sr-only { position: absolute; width: 1px; height: 1px; overflow: hidden; clip-path: inset(50%); }
  @media (max-width: 760px) { .layout { grid-template-columns: minmax(0, 1fr); } aside { border-right: 0; border-bottom: 1px solid var(--border); padding: 0 0 14px; } section { padding: 14px 0 0; } }
</style>
