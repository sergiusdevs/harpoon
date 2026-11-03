<script lang="ts">
  // Genera código (cURL, fetch, Python…) con la solicitud ya resuelta.
  import { onMount } from "svelte";
  import { store } from "../../state/store.svelte";
  import { api } from "../../services/api";
  import type { CurlPrepared } from "../../domain/curlExport";
  import { generateCode, LANGUAGES, type Language } from "../../domain/codegen";
  import Modal from "../common/Modal.svelte";
  import CodeEditor from "../common/CodeEditor.svelte";
  interface Props { onclose: () => void }
  let { onclose }: Props = $props();

  const KEY = "harpoon.codeLanguage";
  let language = $state<Language>("curl");
  try {
    const saved = localStorage.getItem(KEY);
    if (LANGUAGES.some((l) => l.id === saved)) language = saved as Language;
  } catch { /* sin almacenamiento */ }
  let prepared = $state<CurlPrepared | null>(null);
  let error = $state("");
  let loading = $state(true);
  let copied = $state(false);
  const result = $derived.by(() => {
    if (!prepared) return { code: "", error: "" };
    try { return { code: generateCode(language, prepared), error: "" }; }
    catch (e) { return { code: "", error: e instanceof Error ? e.message : String(e) }; }
  });
  const info = $derived(LANGUAGES.find((l) => l.id === language)!);

  onMount(() => {
    void (async () => {
      try {
        if (!store.collection || !store.draft || !store.selected) throw new Error("Selecciona una solicitud");
        const request = structuredClone($state.snapshot(store.draft));
        prepared = await (await api).prepareCurl(store.collection.root, store.selected, store.env, request);
      } catch (e) { error = e instanceof Error ? e.message : String(e); }
      finally { loading = false; }
    })();
  });
  function choose(id: Language) {
    language = id;
    copied = false;
    try { localStorage.setItem(KEY, id); } catch { /* sin almacenamiento */ }
  }
  async function copy() {
    try { await navigator.clipboard.writeText(result.code); copied = true; setTimeout(() => (copied = false), 1600); }
    catch { error = "No se pudo copiar. Puedes seleccionar el código y copiarlo manualmente."; }
  }
</script>

<Modal title="Generar código" width={820} {onclose}>
  {#if loading}<p class="muted">Preparando solicitud…</p>
  {:else if error && !prepared}<p class="error" role="alert">{error}</p>
  {:else}
    <div class="langs" role="tablist" aria-label="Lenguaje">
      {#each LANGUAGES as l (l.id)}
        <button role="tab" aria-selected={language === l.id} class:on={language === l.id} onclick={() => choose(l.id)}>{l.name}</button>
      {/each}
    </div>
    {#each prepared?.warnings ?? [] as warning}<p class="warning" role="status">⚠ {warning}</p>{/each}
    <div class="code">
      {#if result.error}<p class="error" role="alert">{result.error}</p>
      {:else}<CodeEditor value={result.code} readonly wrap language={language === "fetch" || language === "axios" ? "javascript" : "text"} label={`Código ${info.name}`} />{/if}
    </div>
    <p class="hint muted">{info.hint} · Usa los valores actuales (variables, auth y carpetas ya resueltas) y puede contener credenciales: revísalo antes de compartirlo.</p>
    {#if error}<p class="error" role="alert">{error}</p>{/if}
  {/if}
  {#snippet footer()}<button class="btn" onclick={onclose}>Cerrar</button><button class="btn primary" disabled={!result.code} onclick={copy}>{copied ? "Copiado ✓" : "Copiar código"}</button>{/snippet}
</Modal>

<style>
  .langs { display: flex; flex-wrap: wrap; gap: 6px; margin-bottom: 12px; }
  .langs button { padding: 5px 11px; border: 1px solid var(--border); border-radius: 999px; font-size: 12px; color: var(--text-muted); transition: background var(--t), color var(--t), border-color var(--t); }
  .langs button:hover { color: var(--text); border-color: var(--border-strong); }
  .langs button.on { background: var(--accent-soft); border-color: var(--accent); color: var(--text); }
  .code { height: 340px; border: 1px solid var(--border-strong); border-radius: 7px; overflow: hidden; background: var(--bg-panel); }
  .hint, .error { margin: 10px 0 0; font-size: 12px; }
  .error { color: var(--err); padding: 12px; margin: 0; }
  .warning { margin: 0 0 10px; padding: 8px 10px; font-size: 12px; color: var(--warn); border: 1px solid color-mix(in srgb, var(--warn) 45%, transparent); border-radius: var(--radius); background: color-mix(in srgb, var(--warn) 10%, transparent); }
</style>
