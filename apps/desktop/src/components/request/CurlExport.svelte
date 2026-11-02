<script lang="ts">
  import { onMount } from "svelte";
  import { store } from "../../state/store.svelte";
  import { api } from "../../services/api";
  import { buildCurl } from "../../domain/curlExport";
  import Modal from "../common/Modal.svelte";
  interface Props { onclose: () => void }
  let { onclose }: Props = $props();
  let command = $state("");
  let error = $state("");
  let loading = $state(true);
  let copied = $state(false);
  onMount(() => {
    void (async () => {
      try {
        if (!store.collection || !store.draft || !store.selected) throw new Error("Selecciona una solicitud");
        const request = structuredClone($state.snapshot(store.draft));
        const result = await (await api).prepareCurl(store.collection.root, store.selected, store.env, request);
        command = buildCurl(result);
      } catch (e) { error = e instanceof Error ? e.message : String(e); }
      finally { loading = false; }
    })();
  });
  async function copy() {
    try { await navigator.clipboard.writeText(command); copied = true; }
    catch { error = "No se pudo copiar. Puedes seleccionar el comando y copiarlo manualmente."; }
  }
</script>
<Modal title="Exportar como cURL" width={760} {onclose}>
  {#if loading}<p class="muted">Preparando solicitud…</p>
  {:else if error && !command}<p class="error" role="alert">{error}</p>
  {:else}
    <p class="hint muted">El comando usa los valores actuales y puede contener credenciales o secretos. Revísalo antes de compartirlo.</p>
    <textarea class="mono" aria-label="Comando cURL" readonly value={command} rows="7" onfocus={(e) => e.currentTarget.select()}></textarea>
    <p class="hint muted">Formato para terminales compatibles con sh, zsh y bash.</p>
    {#if error}<p class="error" role="alert">{error}</p>{/if}
  {/if}
  {#snippet footer()}<button class="btn" onclick={onclose}>Cerrar</button><button class="btn primary" disabled={!command} onclick={copy}>{copied ? "Copiado ✓" : "Copiar cURL"}</button>{/snippet}
</Modal>
<style>
  textarea { display: block; width: 100%; resize: vertical; padding: 12px; color: var(--text); background: var(--bg); border: 1px solid var(--border); border-radius: var(--radius); user-select: text; font-size: 12px; line-height: 1.5; }
  .hint, .error { margin: 0 0 12px; font-size: 12px; } textarea + .hint { margin: 10px 0 0; } .error { color: var(--err); }
</style>
