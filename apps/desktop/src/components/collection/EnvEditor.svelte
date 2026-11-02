<script lang="ts">
  // Editor de entornos: lista a la izquierda, variables a la derecha.
  import { store, slug } from "../../state/store.svelte";
  import type { Environment } from "../../domain/types";
  import Modal from "../common/Modal.svelte";

  const names = $derived(Object.keys(store.collection?.environments ?? {}));
  let current = $state<string | null>(null);
  let draft = $state<Environment>({ vars: {}, secrets: [] });
  let rows = $state<{ k: string; v: string; secret: boolean }[]>([]);
  let dirty = $state(false);
  let newName = $state("");

  $effect(() => {
    if (!current && names.length) load(names[0]);
  });

  function load(name: string) {
    const env = store.collection?.environments[name];
    if (!env) return;
    current = name;
    draft = structuredClone($state.snapshot(env));
    rows = [...Object.entries(draft.vars).map(([k, v]) => ({ k, v, secret: draft.secrets.includes(k) })), { k: "", v: "", secret: false }];
    dirty = false;
  }

  function edit(i: number, patch: Partial<{ k: string; v: string; secret: boolean }>) {
    Object.assign(rows[i], patch);
    if (i === rows.length - 1 && (rows[i].k || rows[i].v)) rows.push({ k: "", v: "", secret: false });
    dirty = true;
  }

  function remove(i: number) {
    rows.splice(i, 1);
    dirty = true;
  }

  async function save() {
    if (!current) return;
    const env: Environment = { vars: {}, secrets: [] };
    for (const r of rows) {
      const k = r.k.trim();
      if (!k) continue;
      env.vars[k] = r.v;
      if (r.secret) env.secrets.push(k);
    }
    await store.saveEnvironment(current, env);
    dirty = false;
  }

  async function create() {
    const name = slug(newName);
    if (!name) return;
    await store.saveEnvironment(name, { vars: { base_url: "" }, secrets: [] });
    newName = "";
    load(name);
  }

  function del(name: string) {
    store.dialog = {
      kind: "confirm",
      title: "Borrar entorno",
      text: `Se eliminarán environments/${name}.toml y su archivo de secretos.`,
      ok: "Borrar",
      danger: true,
      onOk: async () => {
        await store.deleteEnvironment(name);
        current = null;
      },
    };
  }

  function setDefault(name: string) {
    if (!store.collection) return;
    void store.saveMeta({ ...$state.snapshot(store.collection.meta), default_environment: name });
  }
</script>

<Modal title="Entornos" width={820} onclose={() => (store.modal = null)}>
  <div class="layout">
    <aside>
      {#each names as n}
        <button class="env" class:active={n === current} onclick={() => load(n)}>
          <span>{n}</span>
          {#if store.collection?.meta.default_environment === n}<span class="badge">por defecto</span>{/if}
        </button>
      {/each}
      <div class="new">
        <input placeholder="nuevo entorno" bind:value={newName} onkeydown={(e) => e.key === "Enter" && create()} spellcheck="false" />
        <button class="btn" onclick={create} disabled={!newName.trim()}>+</button>
      </div>
    </aside>
    <section>
      {#if current}
        <div class="head">
          <h4>{current}</h4>
          <div class="row">
            {#if store.collection?.meta.default_environment !== current}
              <button class="btn" onclick={() => setDefault(current!)}>Hacer predeterminado</button>
            {/if}
            <button class="btn" onclick={() => del(current!)}>Borrar</button>
          </div>
        </div>
        <table class="kv">
          <thead><tr><th>Variable</th><th>Valor</th><th class="c" title="Los secretos se guardan en <entorno>.secrets.toml, fuera de Git">secreto</th><th></th></tr></thead>
          <tbody>
            {#each rows as r, i (i)}
              <tr class:secret={r.secret}>
                <td style="width: 32%"><input value={r.k} placeholder="nombre" oninput={(e) => edit(i, { k: e.currentTarget.value })} spellcheck="false" /></td>
                <td><input value={r.v} type={r.secret ? "password" : "text"} placeholder="valor" oninput={(e) => edit(i, { v: e.currentTarget.value })} spellcheck="false" /></td>
                <td class="c"><input type="checkbox" checked={r.secret} onchange={(e) => edit(i, { secret: e.currentTarget.checked })} /></td>
                <td class="del">{#if i < rows.length - 1}<button class="del" onclick={() => remove(i)}>✕</button>{/if}</td>
              </tr>
            {/each}
          </tbody>
        </table>
        <p class="muted hint">Los valores marcados como secreto no se escriben en <code>environments/{current}.toml</code>, sino en <code>{current}.secrets.toml</code>, que está en <code>.gitignore</code>.</p>
      {:else}
        <p class="muted">Crea un entorno para empezar.</p>
      {/if}
    </section>
  </div>
  {#snippet footer()}
    <button class="btn" onclick={() => (store.modal = null)}>Cerrar</button>
    <button class="btn primary" onclick={save} disabled={!dirty || !current}>Guardar</button>
  {/snippet}
</Modal>

<style>
  .layout {
    display: grid;
    grid-template-columns: 200px 1fr;
    gap: 16px;
    min-height: 320px;
  }
  aside {
    display: flex;
    flex-direction: column;
    gap: 2px;
    border-right: 1px solid var(--border);
    padding-right: 12px;
  }
  .env {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 6px;
    padding: 6px 8px;
    border-radius: var(--radius);
    text-align: left;
  }
  .env:hover {
    background: var(--bg-hover);
  }
  .env.active {
    background: var(--accent-soft);
  }
  .badge {
    font-size: 10px;
    color: var(--text-muted);
  }
  .new {
    display: flex;
    gap: 4px;
    margin-top: 8px;
  }
  .new input {
    min-width: 0;
    flex: 1;
    font-size: 12px;
  }
  .head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 8px;
  }
  h4 {
    margin: 0;
    font-family: var(--mono);
  }
  .row {
    display: flex;
    gap: 6px;
  }
  th {
    text-align: left;
    font-weight: 500;
    color: var(--text-muted);
    font-size: 11px;
    padding: 2px 4px 6px;
  }
  .c {
    text-align: center;
    width: 60px;
  }
  tr.secret input[type="text"],
  tr.secret input[type="password"] {
    color: var(--warn);
  }
  .hint {
    font-size: 11.5px;
    margin-top: 10px;
  }
  code {
    font-family: var(--mono);
    font-size: 11px;
  }
</style>
