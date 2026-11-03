<script lang="ts">
  import { store } from "../../state/store.svelte";
  import { folderChain } from "../../domain/paths";
  import { variableSuggestions } from "../../domain/variables";
  import Modal from "../common/Modal.svelte";

  let { embedded = false }: { embedded?: boolean } = $props();
  type Layer = { value: string; source: string; secret: boolean };
  let filter = $state("");
  let revealed = $state<Record<string, boolean>>({});
  const rows = $derived.by(() => {
    const col = store.collection;
    if (!col) return [];
    const values = new Map<string, Layer[]>();
    const env = store.env ?? col.meta.default_environment;
    const secretNames = new Set(env ? col.environments[env]?.secrets ?? [] : []);
    const sensitive = (name: string) => secretNames.has(name) || /token|secret|password|passwd|api.?key|authorization|credential/i.test(name);
    const add = (vars: Record<string, string>, source: string, hideAll = false) => {
      for (const [name, value] of Object.entries(vars)) {
        const layers = values.get(name) ?? [];
        layers.unshift({ value, source, secret: hideAll || sensitive(name) });
        values.set(name, layers);
      }
    };
    add(col.meta.vars, "Colección");
    for (const folder of folderChain(store.selected ?? "")) add(col.folder_configs[folder]?.vars ?? {}, `Carpeta · ${folder}`);
    if (env && col.environments[env]) add(col.environments[env].vars, `Entorno · ${env}`);
    add(store.runtimeVariables, "Ejecución", true);
    for (const { name } of variableSuggestions([])) {
      const layers = values.get(name) ?? [];
      layers.unshift({ value: "Se genera al enviar", source: "Dinámica", secret: false });
      values.set(name, layers);
    }
    const query = filter.trim().toLocaleLowerCase();
    return [...values].map(([name, layers]) => ({ name, layers }))
      .filter((row) => !query || row.name.toLocaleLowerCase().includes(query) || row.layers.some((layer) => layer.source.toLocaleLowerCase().includes(query)))
      .sort((a, b) => a.name.localeCompare(b.name));
  });
  function id(name: string, source: string) { return JSON.stringify([name, source]); }
</script>
{#snippet content()}
  <p class="muted hint">Prioridad: ejecución → entorno → carpeta más cercana → colección. Las variables dinámicas se generan al enviar.</p>
  <div class="context"><span class="muted">Solicitud: <span class="mono">{store.selected ?? "—"}</span></span><input bind:value={filter} placeholder="Filtrar variables u origen" aria-label="Filtrar variables u origen" /></div>
  <div class="table-wrap"><table><thead><tr><th>Variable</th><th>Valor efectivo</th><th>Origen</th></tr></thead><tbody>
    {#each rows as row (row.name)}
      {@const effective = row.layers[0]}
      <tr>
        <td class="mono">{row.name}</td>
        <td><div class="value"><span class="mono">{effective.secret && !revealed[id(row.name, effective.source)] ? "••••••••" : effective.value || "(vacío)"}</span>{#if effective.secret}<button class="reveal" onclick={() => { const key = id(row.name, effective.source); revealed[key] = !revealed[key]; }} aria-label={`${revealed[id(row.name, effective.source)] ? "Ocultar" : "Mostrar"} valor de ${row.name}`}>{revealed[id(row.name, effective.source)] ? "Ocultar" : "Mostrar"}</button>{/if}</div></td>
        <td class="muted">{effective.source}</td>
      </tr>
      {#if row.layers.length > 1}
        <tr class="overrides"><td colspan="3"><details><summary>{row.layers.length - 1} {row.layers.length === 2 ? "valor sobrescrito" : "valores sobrescritos"}</summary>
          {#each row.layers.slice(1) as layer (layer.source)}<div class="layer"><span class="muted">{layer.source}</span><span class="mono layer-value">{layer.secret && !revealed[id(row.name, layer.source)] ? "••••••••" : layer.value || "(vacío)"}</span>{#if layer.secret}<button class="reveal" onclick={() => { const key = id(row.name, layer.source); revealed[key] = !revealed[key]; }} aria-label={`${revealed[id(row.name, layer.source)] ? "Ocultar" : "Mostrar"} valor de ${row.name} en ${layer.source}`}>{revealed[id(row.name, layer.source)] ? "Ocultar" : "Mostrar"}</button>{/if}</div>{/each}
        </details></td></tr>
      {/if}
    {/each}
  </tbody></table>{#if !rows.length}<p class="muted hint empty">No hay variables que coincidan.</p>{/if}</div>
  <p class="muted note">Los valores marcados como secretos y los de ejecución se ocultan inicialmente. Los valores mostrados son los guardados para cada ámbito.</p>
{/snippet}
{#if embedded}{@render content()}{:else}<Modal title="Variables de esta solicitud" width={760} onclose={() => (store.modal = null)}>{@render content()}{#snippet footer()}<button class="btn" onclick={() => (store.modal = null)}>Cerrar</button>{/snippet}</Modal>{/if}
<style>
  .hint, .note { font-size: 12px; line-height: 1.5; margin: 0 0 12px; } .note { margin: 12px 0 0; }
  .context { display: flex; flex-wrap: wrap; align-items: center; justify-content: space-between; gap: 10px; font-size: 12px; margin-bottom: 12px; } .context input { width: min(240px, 100%); }
  .table-wrap { max-height: 420px; overflow: auto; border: 1px solid var(--border); border-radius: 7px; }
  table { width: 100%; border-collapse: collapse; font-size: 12px; text-align: left; table-layout: fixed; }
  th, td { padding: 8px 10px; border-bottom: 1px solid var(--border); vertical-align: top; overflow-wrap: anywhere; }
  th { color: var(--text-muted); font-weight: 500; } th:first-child { width: 27%; } th:last-child { width: 27%; }
  .value, .layer { display: flex; gap: 8px; justify-content: space-between; align-items: start; } .value span, .layer-value { overflow-wrap: anywhere; min-width: 0; }
  .reveal { color: var(--accent); font-size: 11px; flex: none; } .reveal:hover { text-decoration: underline; }
  .overrides td { padding-top: 0; } summary { color: var(--text-muted); cursor: pointer; font-size: 11px; } .layer { padding: 8px 0 0 14px; } .layer > span:first-child { flex: 1; } .layer-value { flex: 2; }
  .empty { padding: 12px; margin: 0; }
</style>
