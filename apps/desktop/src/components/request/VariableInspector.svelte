<script lang="ts">
  import { store } from "../../state/store.svelte";
  import { folderChain } from "../../domain/paths";
  import { variableSuggestions } from "../../domain/variables";
  import Modal from "../common/Modal.svelte";

  const rows = $derived.by(() => {
    const col = store.collection;
    if (!col) return [];
    const values = new Map<string, { value: string; source: string; secret: boolean }>();
    const sensitive = (name: string) => /token|secret|password|passwd|api.?key|authorization|credential/i.test(name);
    const add = (vars: Record<string, string>, source: string, secretNames: string[] = [], hideAll = false) => {
      for (const [name, value] of Object.entries(vars)) values.set(name, { value, source, secret: hideAll || secretNames.includes(name) || sensitive(name) });
    };
    add(col.meta.vars, "Colección");
    for (const folder of folderChain(store.selected ?? "")) add(col.folder_configs[folder]?.vars ?? {}, `Carpeta · ${folder}`);
    const env = store.env ?? col.meta.default_environment;
    if (env && col.environments[env]) add(col.environments[env].vars, `Entorno · ${env}`, col.environments[env].secrets);
    add(store.runtimeVariables, "Ejecución", [], true);
    const dynamic = variableSuggestions([]).map((item) => ({ name: item.name, value: "Se genera al enviar", source: item.source, secret: false }));
    return [...[...values].map(([name, info]) => ({ name, ...info })), ...dynamic].sort((a, b) => a.name.localeCompare(b.name));
  });
</script>
<Modal title="Variables de esta solicitud" width={700} onclose={() => (store.modal = null)}>
  <p class="muted hint">Se muestra el valor de mayor prioridad. Las variables de ejecución y las identificadas como secretas están ocultas.</p>
  <div class="table-wrap"><table><thead><tr><th>Variable</th><th>Valor</th><th>Origen</th></tr></thead><tbody>
    {#each rows as row}<tr><td class="mono">{row.name}</td><td class="mono value">{row.secret ? "••••••••" : row.value}</td><td class="muted">{row.source}</td></tr>{/each}
  </tbody></table></div>
  {#snippet footer()}<button class="btn" onclick={() => (store.modal = null)}>Cerrar</button>{/snippet}
</Modal>
<style>
  .hint { font-size: 12px; margin: 0 0 12px; } .table-wrap { max-height: 420px; overflow: auto; }
  table { width: 100%; border-collapse: collapse; font-size: 12px; text-align: left; }
  th, td { padding: 8px; border-bottom: 1px solid var(--border); vertical-align: top; }
  th { color: var(--text-muted); font-weight: 500; } td { overflow-wrap: anywhere; } .value { user-select: text; }
</style>
