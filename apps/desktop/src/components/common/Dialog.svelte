<script lang="ts">
  // Diálogos para crear solicitudes, introducir texto o confirmar acciones.
  import { modalFocus } from "../../domain/modalFocus";
  import { store } from "../../state/store.svelte";

  import MethodSelect from "../request/MethodSelect.svelte";
  import { REQUEST_TYPES, type RequestType, type Method } from "../../domain/types";

  const d = $derived(store.dialog);
  let value = $state("");
  let type = $state<RequestType>("http");
  let method = $state<Method>("GET");
  let url = $state("");
  let fromCurl = $state(false);
  let curl = $state("");
  let error = $state<string | null>(null);
  let busy = $state(false);
  let input = $state<HTMLInputElement | null>(null);

  $effect(() => {
    if (d?.kind === "prompt" || d?.kind === "request") {
      value = d.value;
      url = ""; fromCurl = false; curl = "";
      type = "http";
      method = d.kind === "request" ? d.method : "GET";
      error = null;
      queueMicrotask(() => {
        input?.focus();
        input?.select();
      });
    }
  });

  async function ok() {
    if (!d || busy) return;
    busy = true;
    error = null;
    const current = d;
    try {
      if (current.kind === "request") {
        if (fromCurl && !curl.trim()) throw new Error("Pega un comando cURL para importar la solicitud.");
        await current.onOk(value, method, type, url, fromCurl ? curl : undefined);
      }
      else if (current.kind === "prompt") await current.onOk(value);
      else await current.onOk();
      if (store.dialog === current) store.dialog = null;
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  function cancel() {
    if (busy) return;
    store.dialog = null;
  }

  function onKey(e: KeyboardEvent) {
    if (!d) return;
    if (e.key === "Escape") cancel();
    if (e.key === "Enter" && !(e.target instanceof HTMLTextAreaElement) && !(e.target instanceof HTMLButtonElement) && !(e.target instanceof HTMLSelectElement)) { e.preventDefault(); void ok(); }
  }
</script>

<svelte:window onkeydown={onKey} />

{#if d}
  <div class="backdrop" role="presentation" onclick={cancel}>
    <div class="dialog fade-in" class:new-request={d.kind === "request"} use:modalFocus={{ onEscape: cancel }} role="dialog" aria-modal="true" aria-labelledby="dlg-title" tabindex="-1" onkeydown={() => {}} onclick={(e) => e.stopPropagation()}>
      <div class="dialog-heading"><h3 id="dlg-title">{d.title}</h3><button class="close-dialog" aria-label="Cerrar diálogo" onclick={cancel} disabled={busy}>×</button></div>
      {#if d.kind === "request"}
        <fieldset class="request-kinds" disabled={busy}>
          <legend>Tipo de solicitud</legend>
          <div class="kind-options">
            {#each REQUEST_TYPES as option}<label><input type="radio" name="request-kind" checked={!fromCurl && type === option.value} onchange={() => { type = option.value; fromCurl = false; }} /><span>{option.label}</span></label>{/each}
            <label><input type="radio" name="request-kind" checked={fromCurl} onchange={() => (fromCurl = true)} /><span>Desde cURL</span></label>
          </div>
        </fieldset>
        <label for="dlg-input" class="field-label">Nombre de la solicitud</label>
        <input id="dlg-input" disabled={busy} bind:this={input} bind:value placeholder="Por ejemplo, Listar usuarios" spellcheck="false" />
        {#if fromCurl}
          <label for="dlg-curl" class="field-label">Comando cURL</label>
          <textarea id="dlg-curl" class="mono curl-source" bind:value={curl} placeholder="curl https://api.ejemplo.com/usuarios" disabled={busy} spellcheck="false"></textarea>
        {:else}
          <label for="dlg-url" class="field-label">URL</label>
          <div class="create-address">
            {#if type === "http"}<MethodSelect value={method} onchange={(value) => (method = value)} />{:else if type === "graphql"}<span class="fixed-method">POST</span>{/if}
            <input id="dlg-url" class="mono" bind:value={url} placeholder={type === "websocket" ? "wss://ejemplo.com/socket" : type === "graphql" ? "{{base_url}}/graphql" : type === "sse" ? "{{base_url}}/events" : "{{base_url}}/usuarios"} disabled={busy} spellcheck="false" />
          </div>
        {/if}
      {:else if d.kind === "prompt"}
        {#if d.label}<label for="dlg-input" class="muted">{d.label}</label>{/if}
        <input id="dlg-input" disabled={busy} class="mono" bind:this={input} bind:value placeholder={d.placeholder ?? ""} spellcheck="false" />
      {:else}<p>{d.text}</p>{/if}
      {#if error}<p class="error">{error}</p>{/if}
      <div class="actions">
        <button class="btn" onclick={cancel} disabled={busy}>Cancelar</button>
        <button class="btn primary" class:danger={d.kind === "confirm" && d.danger} onclick={ok} disabled={busy}>{d.ok}</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.45);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 50;
  }
  .dialog.new-request { width: 560px; padding: 24px; gap: 12px; border-radius: 10px; }
  .dialog-heading { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
  .close-dialog { width: 24px; height: 24px; font-size: 22px; color: var(--text-faint); border-radius: 4px; }
  .close-dialog:hover { color: var(--text); background: var(--bg-hover); }
  .new-request h3 { font-size: 17px; font-weight: 600; }
  .request-kinds { border: 0; padding: 10px 0 6px; margin: 5px 0 0; min-width: 0; }
  .request-kinds legend, .field-label { font-size: 12px; font-weight: 600; color: var(--text); }
  .kind-options { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 14px 12px; padding-top: 12px; }
  .kind-options label { display: inline-flex; align-items: center; gap: 8px; font-size: 12px; color: var(--text-muted); }
  .kind-options input { width: 14px; height: 14px; padding: 0; accent-color: var(--accent); }
  .new-request > input { min-height: 38px; }
  .field-label { margin-top: 9px; }
  .create-address { display: flex; align-items: center; min-height: 38px; min-width: 0; border: 1px solid var(--border-strong); border-radius: 6px; }
  .create-address input { flex: 1; width: 0; min-width: 0; border: 0; outline: none; background: transparent; padding: 8px 10px; }
  .create-address input:focus-visible { outline: none; }
  .fixed-method { padding: 7px 12px; font: 600 11px var(--mono); color: var(--m-post); }
  .curl-source { width: 100%; min-height: 110px; resize: vertical; border: 1px solid var(--border-strong); background: var(--bg); border-radius: 6px; padding: 10px; font-size: 12px; }
  .new-request .actions { margin-top: 22px; }
  @media (max-width: 500px) { .kind-options { grid-template-columns: repeat(2, minmax(0, 1fr)); } }
  .dialog {
    width: 420px;
    max-width: calc(100vw - 40px);
    background: var(--bg-raised);
    border: 1px solid var(--border-strong);
    border-radius: 10px;
    padding: 18px;
    max-height: calc(100vh - 40px);
    overflow: auto;
    box-shadow: 0 20px 50px rgba(0, 0, 0, 0.4);
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  h3 {
    margin: 0;
    font-size: 14px;
  }
  label {
    font-size: 12px;
  }
  input {
    width: 100%;
    font-size: 12.5px;
  }
  p {
    margin: 0;
    font-size: 12.5px;
  }
  .error {
    color: var(--err);
    font-size: 12px;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 4px;
  }
  .btn.primary.danger {
    background: var(--err);
    border-color: var(--err);
    color: #fff;
  }
</style>
