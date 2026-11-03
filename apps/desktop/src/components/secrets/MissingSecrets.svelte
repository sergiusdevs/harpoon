<script lang="ts">
  // Secretos declarados en la colección cuyo valor no está en esta máquina
  // (colección clonada o importada): se piden una vez y se guardan en local.
  import { store } from "../../state/store.svelte";
  import type { MissingSecret, SecretScope } from "../../domain/types";
  import Modal from "../common/Modal.svelte";
  import SecretInput from "./SecretInput.svelte";

  const keyOf = (m: MissingSecret) => `${m.scope}:${m.owner}:${m.name}`;
  const groups = $derived.by(() => {
    const out: { title: string; items: MissingSecret[] }[] = [];
    for (const m of store.missingSecrets) {
      const title = m.scope === "collection" ? "Colección" : m.scope === "folder" ? `Carpeta ${m.owner}` : `Entorno ${m.owner}`;
      let group = out.find((g) => g.title === title);
      if (!group) out.push((group = { title, items: [] }));
      group.items.push(m);
    }
    return out;
  });
  let values = $state<Record<string, string>>({});
  let busy = $state(false);
  let error = $state("");
  const filled = $derived(store.missingSecrets.filter((m) => values[keyOf(m)]?.length));

  const scopeOf = (m: MissingSecret): SecretScope => m.scope === "collection" ? { kind: "collection" } : m.scope === "folder" ? { kind: "folder", path: m.owner } : { kind: "environment", name: m.owner };
  function later() { store.dismissedMissing = store.missingKey; store.modal = null; }
  async function save() {
    busy = true; error = "";
    try {
      for (const m of filled) { await store.setSecret(scopeOf(m), m.name, values[keyOf(m)]); delete values[keyOf(m)]; }
      later();
    } catch (cause) { error = cause instanceof Error ? cause.message : String(cause); }
    finally { busy = false; }
  }
</script>

<Modal title="Faltan secretos en esta máquina" width={600} onclose={later}>
  <p class="intro">Esta colección usa variables secretas cuyo valor no está en Git (por ejemplo, si la has clonado o importado). Rellénalas aquí: se guardan solo en este equipo.</p>
  {#if groups.length}
    {#each groups as group}
      <h4>{group.title}</h4>
      <div class="rows">
        {#each group.items as m (keyOf(m))}
          <div class="row">
            <span class="name mono" title={m.name}>{m.name}</span>
            <div class="field">
              <SecretInput value={values[keyOf(m)] ?? ""} oninput={(v) => (values[keyOf(m)] = v)} label={`Valor de ${m.name}`} placeholder="Valor" disabled={busy} onkeydown={(e) => e.key === "Enter" && filled.length && save()} />
              <span class="hint">o define la variable de sistema <code>{m.env_var}</code></span>
            </div>
          </div>
        {/each}
      </div>
    {/each}
  {:else}
    <p class="muted">No falta ningún secreto.</p>
  {/if}
  {#if error}<p class="error" role="alert">{error}</p>{/if}
  {#snippet footer()}
    <button class="btn" onclick={later} disabled={busy}>Más tarde</button>
    <button class="btn primary" onclick={save} disabled={busy || !filled.length}>Guardar{filled.length > 1 ? ` (${filled.length})` : ""}</button>
  {/snippet}
</Modal>

<style>
  .intro { margin: 0 0 6px; font-size: 12.5px; line-height: 1.55; color: var(--text-muted); }
  h4 { margin: 16px 0 6px; font-size: 11px; font-weight: 600; text-transform: uppercase; letter-spacing: .05em; color: var(--text-muted); }
  .rows { border: 1px solid var(--border-strong); border-radius: 7px; overflow: hidden; }
  .row { display: grid; grid-template-columns: 150px minmax(0, 1fr); gap: 12px; align-items: start; padding: 9px 12px; border-top: 1px solid var(--border); }
  .row:first-child { border-top: 0; }
  .name { font-size: 12px; padding-top: 6px; overflow: hidden; text-overflow: ellipsis; }
  .field { display: flex; flex-direction: column; gap: 4px; min-width: 0; }
  .field :global(input) { font-size: 12px; font-family: var(--mono); }
  .hint { font-size: 11px; color: var(--text-faint); }
  code { font: 10.5px var(--mono); background: var(--bg-active); padding: 0 4px; border-radius: 3px; color: var(--text-muted); }
  .error { color: var(--err); font-size: 12px; }
</style>
