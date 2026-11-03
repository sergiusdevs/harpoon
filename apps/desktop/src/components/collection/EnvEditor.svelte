<script lang="ts">
  import { store, slug } from "../../state/store.svelte";
  import type { Environment } from "../../domain/types";
  import Modal from "../common/Modal.svelte";

  type Row = { k: string; v: string; d: string; secret: boolean; show: boolean };
  const blank = (): Row => ({ k: "", v: "", d: "", secret: false, show: false });
  const names = $derived(Object.keys(store.collection?.environments ?? {}).sort((a, b) => a.localeCompare(b)));
  let current = $state<string | null>(null);
  let rows = $state<Row[]>([]);
  let dirty = $state(false);
  let busy = $state(false);
  let error = $state("");
  let search = $state("");
  let variableSearch = $state("");
  let newName = $state("");
  const filtered = $derived(names.filter((name) => name.toLowerCase().includes(search.toLowerCase())));
  const visibleRows = $derived(rows.map((row, index) => ({ row, index })).filter(({ row }) => !variableSearch || row.k.toLowerCase().includes(variableSearch.toLowerCase()) || row.d.toLowerCase().includes(variableSearch.toLowerCase())));
  $effect(() => { if (!current && names.length) load(store.env && names.includes(store.env) ? store.env : names[0]); });

  function load(name: string) {
    const env = store.collection?.environments[name]; if (!env) return;
    current = name; variableSearch = ""; error = "";
    rows = [...Object.entries(env.vars).map(([k, v]) => ({ k, v, d: env.descriptions?.[k] ?? "", secret: env.secrets.includes(k), show: false })), blank()];
    dirty = false;
  }
  function guard(action: () => void | Promise<void>) {
    if (busy) return;
    if (!dirty) { void action(); return; }
    store.dialog = { kind: "confirm", title: "Cambios del entorno sin guardar", text: "Descarta los cambios de este entorno para continuar, o cancela y guárdalos.", ok: "Descartar cambios", onOk: async () => { dirty = false; await action(); } };
  }
  function edit(index: number, patch: Partial<Row>) {
    Object.assign(rows[index], patch); if (patch.secret !== undefined) rows[index].show = false;
    if (index === rows.length - 1 && (rows[index].k || rows[index].v || rows[index].d)) rows.push(blank());
    dirty = true; error = "";
  }
  function remove(index: number) { rows.splice(index, 1); if (!rows.length || rows.at(-1)!.k) rows.push(blank()); dirty = true; }
  async function save() {
    if (!current || busy) return;
    busy = true; error = "";
    try {
      const env: Environment = { vars: {}, secrets: [], descriptions: {} };
      for (const row of rows) {
        const key = row.k.trim(); if (!key) continue;
        if (key in env.vars) throw new Error(`La variable ${key} está repetida.`);
        env.vars[key] = row.v; if (row.d.trim()) env.descriptions![key] = row.d;
        if (row.secret) env.secrets.push(key);
      }
      await store.saveEnvironment(current, env); load(current);
    } catch (cause) { error = cause instanceof Error ? cause.message : String(cause); }
    finally { busy = false; }
  }
  function create() {
    guard(async () => {
      const name = slug(newName); if (!name) return;
      if (names.includes(name)) { error = `Ya existe el entorno ${name}.`; return; }
      busy = true;
      try { await store.saveEnvironment(name, { vars: { base_url: "" }, secrets: [] }); search = ""; newName = ""; load(name); }
      catch (cause) { error = String(cause); }
      finally { busy = false; }
    });
  }
  function del() {
    if (!current) return;
    const name = current;
    store.dialog = { kind: "confirm", title: "Borrar entorno", text: `Se eliminarán ${name}.toml, su archivo de secretos y los cambios sin guardar.`, ok: "Borrar", danger: true, onOk: async () => { await store.deleteEnvironment(name); dirty = false; current = null; rows = []; } };
  }
  async function setDefault() {
    if (!store.collection || !current || busy) return;
    busy = true;
    await store.saveMeta({ ...$state.snapshot(store.collection.meta), default_environment: current });
    if (store.status.kind === "err") error = store.status.text;
    busy = false;
  }
</script>

<Modal title="Entornos de la colección" width={1060} onclose={() => guard(() => { store.modal = null; })}>
  <div class="layout">
    <aside>
      <div class="list-heading">Entornos<span>{names.length}</span></div>
      <input class="search" aria-label="Buscar entornos" placeholder="Buscar entornos…" bind:value={search} />
      <div class="env-list">{#each filtered as name}<button class="env" class:active={name === current} onclick={() => guard(() => load(name))} disabled={busy}><span class="env-label">{name}</span>{#if store.env === name}<span class="active-check" title="Entorno activo">✓</span>{/if}{#if store.collection?.meta.default_environment === name}<span class="default-mark" title="Entorno predeterminado">●</span>{/if}</button>{/each}{#if !filtered.length}<p class="muted hint">No hay entornos que coincidan.</p>{/if}</div>
      <div class="new"><input aria-label="Nombre del nuevo entorno" placeholder="Nuevo entorno" bind:value={newName} onkeydown={(event) => event.key === "Enter" && create()} disabled={busy} spellcheck="false" /><button class="btn icon" aria-label="Crear entorno" onclick={create} disabled={busy || !newName.trim()}>+</button></div>
    </aside>
    <section aria-label="Editor del entorno">
      {#if current}
        <div class="head"><div><h4>{current}</h4><span class="context">{store.env === current ? "Entorno activo" : "Editando entorno"}{dirty ? " · cambios sin guardar" : ""}</span></div><div class="actions">
          {#if store.env !== current}<button class="btn small" onclick={() => (store.env = current)} disabled={dirty || busy}>Usar entorno</button>{/if}
          {#if store.collection?.meta.default_environment !== current}<button class="btn small" onclick={setDefault} disabled={busy}>Predeterminado</button>{/if}
          <button class="btn icon delete" aria-label="Borrar entorno" title="Borrar entorno" onclick={del} disabled={busy}><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" aria-hidden="true"><path d="M3 6h18M9 6V3h6v3M5 6l1 15h12l1-15M10 10v7M14 10v7"/></svg></button>
        </div></div>
        <div class="table-heading"><span>Variables y secretos</span><input aria-label="Buscar variables del entorno" placeholder="Buscar variable…" bind:value={variableSearch} /></div>
        <div class="table-frame"><table><thead><tr><th scope="col">Nombre</th><th scope="col">Valor</th><th scope="col">Descripción</th><th class="secret-col" scope="col">Secreto</th><th class="delete-col" scope="col"><span class="sr-only">Eliminar</span></th></tr></thead><tbody>
          {#each visibleRows as { row, index } (index)}<tr>
            <td><input aria-label={`Nombre de variable ${index + 1}`} value={row.k} placeholder="Nombre" oninput={(event) => edit(index, { k: event.currentTarget.value })} disabled={busy} spellcheck="false" /></td>
            <td><div class="value-cell"><input aria-label={`Valor de ${row.k || `variable ${index + 1}`}`} type={row.secret && !row.show ? "password" : "text"} value={row.v} placeholder="Valor" oninput={(event) => edit(index, { v: event.currentTarget.value })} disabled={busy} spellcheck="false" autocomplete="off" />{#if row.secret}<button class="eye" aria-label={`${row.show ? "Ocultar" : "Mostrar"} valor de ${row.k}`} aria-pressed={row.show} onclick={() => (row.show = !row.show)}><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" aria-hidden="true"><path d="M2 12s3-7 10-7 10 7 10 7-3 7-10 7S2 12 2 12z"/><circle cx="12" cy="12" r="3"/>{#if row.show}<path d="m3 3 18 18"/>{/if}</svg></button>{/if}</div></td>
            <td><input aria-label={`Descripción de ${row.k || `variable ${index + 1}`}`} value={row.d} placeholder="Descripción" oninput={(event) => edit(index, { d: event.currentTarget.value })} disabled={busy} /></td>
            <td class="secret-col"><input type="checkbox" aria-label={`Marcar ${row.k || `variable ${index + 1}`} como secreto`} checked={row.secret} onchange={(event) => edit(index, { secret: event.currentTarget.checked })} disabled={busy} /></td>
            <td class="delete-col">{#if index < rows.length - 1}<button class="eye delete" aria-label={`Eliminar variable ${row.k}`} onclick={() => remove(index)} disabled={busy}><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" aria-hidden="true"><path d="M3 6h18M9 6V3h6v3M5 6l1 15h12l1-15M10 10v7M14 10v7"/></svg></button>{/if}</td>
          </tr>{/each}
        </tbody></table></div>
        <p class="hint muted">Los secretos se guardan en <code>{current}.secrets.toml</code>, fuera de Git. El ojo solo cambia su visibilidad en pantalla.</p>
        {#if error}<p class="error" role="alert">{error}</p>{/if}
        <div class="save-actions"><button class="btn primary" onclick={save} disabled={!dirty || busy}>Guardar</button><button class="reset" onclick={() => load(current!)} disabled={!dirty || busy}>Restablecer</button></div>
      {:else}<p class="muted">Crea un entorno para empezar.</p>{/if}
    </section>
  </div>
</Modal>

<style>
  .layout { display: grid; grid-template-columns: 210px minmax(0, 1fr); gap: 22px; min-height: 380px; }
  aside { display: flex; flex-direction: column; gap: 12px; border-right: 1px solid var(--border); padding-right: 16px; }
  .list-heading { display: flex; justify-content: space-between; font-size: 12px; font-weight: 600; padding: 3px 0; }
  .list-heading span { color: var(--text-faint); font-weight: 400; }
  .search, .new input { width: 100%; min-width: 0; font-size: 12px; }
  .env-list { flex: 1; min-height: 0; display: flex; flex-direction: column; gap: 3px; }
  .env { display: flex; align-items: center; gap: 8px; padding: 9px 10px; border: 1px solid transparent; border-radius: 5px; text-align: left; font-size: 12px; }
  .env:hover { background: var(--bg-hover); }
  .env.active { background: var(--bg-active); border-color: var(--border-strong); font-weight: 600; }
  .env-label { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .active-check { color: var(--ok); }
  .default-mark { color: var(--text-faint); font-size: 6px; }
  .new { display: flex; gap: 4px; }
  section { min-width: 0; }
  .head { display: flex; align-items: center; justify-content: space-between; gap: 12px; flex-wrap: wrap; margin-bottom: 24px; }
  h4 { margin: 0 0 4px; font-size: 15px; font-weight: 600; }
  .context { font-size: 11px; color: var(--text-faint); }
  .actions { display: flex; gap: 6px; align-items: center; }
  .table-heading { display: flex; align-items: center; justify-content: space-between; gap: 10px; font-size: 12px; color: var(--text-muted); margin-bottom: 12px; }
  .table-heading input { font-size: 11px; width: 180px; min-width: 0; }
  .table-frame { border: 1px solid var(--border-strong); border-radius: 7px; overflow: hidden; }
  table { width: 100%; table-layout: fixed; border-collapse: collapse; font-size: 12px; }
  th { padding: 10px; background: var(--bg-panel); font-weight: 600; text-align: left; }
  td { padding: 3px; height: 38px; border-top: 1px solid var(--border); }
  th + th, td + td { border-left: 1px solid var(--border); }
  td input:not([type="checkbox"]) { width: 100%; min-width: 0; border: 1px solid transparent; background: transparent; padding: 6px; font-size: 12px; }
  td:focus-within { background: var(--bg-hover); }
  td input:not([type="checkbox"]):hover, td input:not([type="checkbox"]):focus, td input:not([type="checkbox"]):focus-visible { border-color: transparent; box-shadow: none; outline: none; }
  .value-cell { display: flex; align-items: center; min-width: 0; }
  .value-cell input { flex: 1; min-width: 0; }
  .eye { display: inline-flex; align-items: center; justify-content: center; width: 26px; height: 26px; flex: 0 0 26px; color: var(--text-muted); border-radius: 4px; }
  .eye:hover { background: var(--bg-hover); color: var(--text); }
  .delete:hover { color: var(--err); }
  .secret-col { width: 61px; text-align: center; }
  .delete-col { width: 32px; text-align: center; }
  input[type="checkbox"] { accent-color: var(--accent); }
  .hint { font-size: 11px; line-height: 1.6; margin: 12px 0; }
  code { font: 11px var(--mono); }
  .error { color: var(--err); font-size: 12px; }
  .save-actions { display: flex; align-items: center; gap: 18px; margin-top: 20px; }
  .reset { color: var(--text-muted); font-size: 12px; }
  .reset:hover { color: var(--text); }
  .sr-only { position: absolute; width: 1px; height: 1px; overflow: hidden; clip-path: inset(50%); }
  @media (max-width: 750px) { .layout { grid-template-columns: minmax(0, 1fr); } aside { border-right: 0; border-bottom: 1px solid var(--border); padding-right: 0; padding-bottom: 14px; } .env-list { flex-direction: row; flex-wrap: wrap; } .table-heading { flex-wrap: wrap; } }
</style>
