<script lang="ts">
  import type { HistoryEntry } from "../../domain/types";
  import { diffLines, formatComparisonBody, formatComparisonHeaders } from "../../domain/diff";
  import Modal from "../common/Modal.svelte";
  interface Props { before: HistoryEntry; after: HistoryEntry; onclose: () => void }
  let { before, after, onclose }: Props = $props();
  let tab = $state<"body" | "headers">("body");
  const changes = $derived(tab === "body"
    ? diffLines(formatComparisonBody(before.body, before.headers["content-type"] ?? ""), formatComparisonBody(after.body, after.headers["content-type"] ?? ""))
    : diffLines(formatComparisonHeaders(before.headers), formatComparisonHeaders(after.headers)));
</script>
<Modal title="Comparar respuestas" width={950} {onclose}>
  <div class="compare-meta"><span>Antes · {before.at} · {before.status}</span><span>Después · {after.at} · {after.status}</span></div>
  {#if before.truncated || after.truncated}<p class="warn">Una de las respuestas tiene contenido incompleto en el historial.</p>{/if}
  <div class="segmented"><button class:active={tab === "body"} aria-pressed={tab === "body"} onclick={() => (tab = "body")}>Cuerpo</button><button class:active={tab === "headers"} aria-pressed={tab === "headers"} onclick={() => (tab = "headers")}>Cabeceras</button></div>
  {#if changes === null}<p class="muted">Las respuestas son demasiado grandes para compararlas aquí.</p>
  {:else}
    <div class="diff" aria-label="Diferencias entre respuestas">{#each changes as line}<div class:added={line.kind === "added"} class:removed={line.kind === "removed"}><span class="prefix">{line.kind === "added" ? "+" : line.kind === "removed" ? "−" : " "}</span><span>{line.text || " "}</span></div>{/each}</div>
  {/if}
  {#snippet footer()}<button class="btn" onclick={onclose}>Cerrar</button>{/snippet}
</Modal>
<style>
  .compare-meta { display: flex; flex-wrap: wrap; gap: 6px 24px; color: var(--text-muted); font-size: 11px; margin-bottom: 12px; }
  .warn { color: var(--warn); font-size: 12px; }
  .diff { max-height: min(55vh, 550px); overflow: auto; border: 1px solid var(--border); border-radius: var(--radius); background: var(--bg); margin-top: 10px; font: 12px/1.55 var(--mono); user-select: text; }
  .diff div { display: flex; white-space: pre; min-width: max-content; padding: 0 10px; }
  .prefix { flex: 0 0 20px; color: var(--text-muted); }
  .added { background: color-mix(in srgb, var(--ok) 15%, transparent); } .added .prefix { color: var(--ok); }
  .removed { background: color-mix(in srgb, var(--err) 15%, transparent); } .removed .prefix { color: var(--err); }
</style>
