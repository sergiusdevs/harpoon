<script lang="ts">
  // Resultado de `harpoon validate` en la app: errores y avisos de la colección.
  import { onMount } from "svelte";
  import { store } from "../../state/store.svelte";
  import { api } from "../../services/api";
  import type { ValidationIssue } from "../../domain/types";
  import Modal from "../common/Modal.svelte";

  let issues = $state<ValidationIssue[] | null>(null);
  let error = $state("");
  const errors = $derived(issues?.filter((i) => i.severity === "error").length ?? 0);
  const warnings = $derived((issues?.length ?? 0) - errors);

  async function run() {
    issues = null; error = "";
    try { issues = await (await api).validateCollection(store.collection!.root, store.env); }
    catch (e) { error = String(e); }
  }
  onMount(run);

  function open(issue: ValidationIssue) {
    if (!issue.request || !store.collection?.requests[issue.request]) return;
    store.modal = null;
    store.select(issue.request);
  }
</script>

<Modal title="Validar colección" width={720} onclose={() => (store.modal = null)}>
  <p class="muted lead">Revisa la colección sin enviar nada{store.env ? ` con el entorno ${store.env}` : ""}: variables sin definir, URLs, JSON, archivos y secretos sin valor. Desde la terminal: <code>harpoon validate</code>.</p>
  {#if error}<p class="error" role="alert">{error}</p>
  {:else if !issues}<p class="muted">Revisando…</p>
  {:else if !issues.length}
    <div class="ok fade-in"><span aria-hidden="true">✓</span> Todo correcto: {Object.keys(store.collection?.requests ?? {}).length} solicitudes revisadas.</div>
  {:else}
    <p class="summary"><span class="err">{errors} {errors === 1 ? "error" : "errores"}</span> · <span class="warn">{warnings} {warnings === 1 ? "aviso" : "avisos"}</span></p>
    <ul class="issues">
      {#each issues as issue, i (i)}
        <li class={issue.severity}>
          <button disabled={!issue.request} onclick={() => open(issue)} title={issue.request ? "Abrir solicitud" : issue.file}>
            <span class="dot" aria-label={issue.severity === "error" ? "Error" : "Aviso"}></span>
            <span class="what"><strong>{issue.request ?? issue.file}</strong><span class="muted"> · {issue.location}</span></span>
            <span class="message">{issue.message}</span>
          </button>
        </li>
      {/each}
    </ul>
  {/if}
  {#snippet footer()}
    <button class="btn" onclick={() => (store.modal = null)}>Cerrar</button>
    <button class="btn primary" onclick={run} disabled={!issues && !error}>Volver a revisar</button>
  {/snippet}
</Modal>

<style>
  .lead { margin: 0 0 14px; font-size: 12px; }
  code { font: 11px var(--mono); background: var(--bg-active); padding: 1px 5px; border-radius: 4px; }
  .ok { padding: 18px; border: 1px solid color-mix(in srgb, var(--ok) 45%, transparent); border-radius: 8px; color: var(--ok); background: color-mix(in srgb, var(--ok) 8%, transparent); }
  .summary { margin: 0 0 10px; font-size: 12px; font-weight: 600; }
  .err { color: var(--err); } .warn { color: var(--warn); }
  .issues { list-style: none; margin: 0; padding: 0; max-height: 380px; overflow-y: auto; border: 1px solid var(--border); border-radius: 8px; }
  .issues li + li { border-top: 1px solid var(--border); }
  .issues button { display: grid; grid-template-columns: 10px 1fr; gap: 2px 10px; width: 100%; padding: 9px 12px; text-align: left; transition: background var(--t); }
  .issues button:not(:disabled):hover { background: var(--bg-hover); }
  .issues button:disabled { cursor: default; }
  .dot { grid-row: span 2; align-self: center; width: 8px; height: 8px; border-radius: 50%; }
  .error .dot { background: var(--err); } .warning .dot { background: var(--warn); }
  .what { font-size: 12px; } .message { font-size: 12px; color: var(--text-muted); }
  .error .message { color: var(--text); }
</style>
