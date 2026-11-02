<script lang="ts">
  import { store } from "../../state/store.svelte";
  import { api } from "../../services/api";
  import type { Outcome, Request } from "../../domain/types";
  import Modal from "../common/Modal.svelte";
  import MethodBadge from "../common/MethodBadge.svelte";

  type RunResult = { key: string; name: string; method: Request["method"]; passed: boolean; status?: number; elapsed?: number; checks?: number; failures: Outcome[]; error?: string };
  let folder = $state(store.runFolder);
  let bail = $state(false);
  let running = $state(false);
  let stop = $state(false);
  let results = $state<RunResult[]>([]);
  let completed = $state(false);
  let current = $state("");
  let error = $state("");
  const selected = $derived(Object.entries(store.collection?.requests ?? {}).filter(([key]) => !folder || key.startsWith(folder + "/")));
  const failures = $derived(results.filter((result) => !result.passed).length);

  function close() { if (!running) store.modal = null; }
  async function run() {
    if (!store.collection || running || !selected.length) return;
    if (store.dirty) { error = "Guarda la solicitud actual antes de ejecutar la colección."; return; }
    if (store.sending) { error = "Espera a que termine la solicitud en curso."; return; }
    const root = store.collection.root;
    const env = store.env;
    const requests = selected.map(([key, request]) => [key, structuredClone($state.snapshot(request))] as const);
    results = []; completed = false; stop = false; error = ""; running = true;
    try {
      const bridge = await api;
      for (const [key, request] of requests) {
        if (stop) break;
        current = key;
        try {
          const exchange = await bridge.sendRequest(root, env, request, key);
          results.push({ key, name: request.name, method: request.method, passed: exchange.passed, status: exchange.response.status, elapsed: exchange.response.elapsed_ms, checks: exchange.assertions.length + exchange.tests.length, failures: [...exchange.assertions, ...exchange.tests].filter((item) => !item.passed) });
          if (store.collection?.root === root) store.runtimeVariables = exchange.vars;
          if (bail && !exchange.passed) break;
        } catch (cause) {
          results.push({ key, name: request.name, method: request.method, passed: false, failures: [], error: String(cause) });
          if (bail) break;
        }
      }
      if (store.collection?.root === root) void store.loadHistory();
      completed = true;
    } catch (cause) { error = String(cause); }
    finally { current = ""; running = false; }
  }
  function openRequest(key: string) { close(); store.select(key); }
</script>

<Modal title="Ejecutar colección" width={760} onclose={close}>
  <div class="controls">
    <label>Alcance<select bind:value={folder} disabled={running} onchange={() => { results = []; completed = false; error = ""; }}><option value="">Toda la colección</option>{#each store.folders as path}<option value={path}>{path}/</option>{/each}</select></label>
    <label class="check"><input type="checkbox" bind:checked={bail} disabled={running} /> Detener al primer fallo</label>
  </div>
  <p class="muted hint">{selected.length} solicitudes · entorno: {store.env ?? store.collection?.meta.default_environment ?? "sin entorno"}. Se ejecutan en orden y comparten las variables de ejecución. El resultado depende de las aserciones y tests configurados.</p>
  {#if store.dirty}<p class="warning">Guarda los cambios de la solicitud actual para ejecutar la versión guardada.</p>{/if}
  {#if error}<p class="error" role="alert">{error}</p>{/if}
  {#if running}<p class="muted hint" aria-live="polite">{stop ? "Se detendrá al terminar la solicitud actual" : `Ejecutando ${current}…`}</p>{/if}
  {#if completed}<p class="summary" aria-live="polite">{stop ? "Ejecución detenida" : "Ejecución finalizada"} · {results.length} ejecutadas · <span class:failed={failures > 0}>{failures} con fallos</span></p>{/if}
  {#if results.length}
    <div class="results">
      {#each results as result (result.key)}
        <div class="result">
          <button class="result-head" onclick={() => openRequest(result.key)} disabled={running} title="Abrir solicitud">
            <span class:failed={!result.passed} class:passed={result.passed}>{result.passed ? "✓" : "✕"}</span><MethodBadge method={result.method} /><span class="name">{result.name}</span><span class="muted">{result.status ?? "Error"}{result.elapsed !== undefined ? ` · ${result.elapsed} ms` : ""}</span>
          </button>
          {#if result.checks === 0}<div class="detail muted">Sin aserciones ni tests</div>{/if}
          {#if result.error}<div class="detail error">{result.error}</div>{/if}
          {#each result.failures as failure}<div class="detail error">{failure.name}{failure.detail ? `: ${failure.detail}` : ""}</div>{/each}
        </div>
      {/each}
    </div>
  {/if}
  {#snippet footer()}
    {#if running}<button class="btn" disabled={stop} onclick={() => { stop = true; }}>Detener</button>{:else}<button class="btn" onclick={close}>Cerrar</button><button class="btn primary" disabled={!selected.length || store.dirty || store.sending} onclick={run}>{results.length ? "Ejecutar de nuevo" : "Ejecutar"}</button>{/if}
  {/snippet}
</Modal>
<style>
  .controls { display: flex; gap: 20px; align-items: end; flex-wrap: wrap; }
  .controls label:not(.check) { display: flex; flex-direction: column; gap: 5px; flex: 1; min-width: 180px; color: var(--text-muted); font-size: 12px; }
  .check { display: flex; align-items: center; gap: 7px; font-size: 12px; padding-bottom: 7px; }
  .hint, .warning, .error, .summary { font-size: 12px; line-height: 1.5; } .warning { color: var(--warn); } .error, .failed { color: var(--err); } .passed { color: var(--ok); }
  .results { border: 1px solid var(--border); border-radius: 7px; overflow: hidden; }
  .result + .result { border-top: 1px solid var(--border); }
  .result-head { display: flex; align-items: center; gap: 10px; width: 100%; padding: 9px 10px; text-align: left; font-size: 12px; }
  .result-head:hover:not(:disabled) { background: var(--bg-hover); } .name { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .detail { padding: 0 12px 8px 32px; overflow-wrap: anywhere; }
</style>
