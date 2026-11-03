<script lang="ts">
  // Diálogos para crear solicitudes, introducir texto o confirmar acciones.
  import { modalFocus } from "../../domain/modalFocus";
  import { store, slug } from "../../state/store.svelte";

  import MethodSelect from "../request/MethodSelect.svelte";
  import { REQUEST_TYPES, type RequestType, type Method } from "../../domain/types";

  const d = $derived(store.dialog);
  let value = $state("");
  let type = $state<RequestType>("http");
  let method = $state<Method>("GET");
  let url = $state("");
  let showOptions = $state(false);
  let file = $state("");
  let fileTouched = $state(false);
  const fileValue = $derived(fileTouched ? file : slug(value));
  let error = $state<string | null>(null);
  let busy = $state(false);
  let input = $state<HTMLInputElement | null>(null);

  $effect(() => {
    if (d?.kind === "prompt" || d?.kind === "request") {
      value = d.value;
      url = ""; showOptions = false; file = ""; fileTouched = false;
      type = "http";
      method = d.kind === "request" ? d.method : "GET";
      error = null;
      // Después de que modalFocus coloque el foco inicial.
      setTimeout(() => {
        input?.focus();
        input?.select();
      }, 0);
    }
  });

  async function ok() {
    if (!d || busy) return;
    busy = true;
    error = null;
    const current = d;
    try {
      if (current.kind === "request") {
        await current.onOk(value, method, type, url, fileValue);
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
          <legend>Tipo</legend>
          <div class="kind-options">
            {#each REQUEST_TYPES as option}<label><input type="radio" name="request-kind" checked={type === option.value} onchange={() => (type = option.value)} /><span>{option.label}</span></label>{/each}
          </div>
        </fieldset>
        <label for="dlg-input" class="field-label">Nombre de la solicitud</label>
        <input id="dlg-input" disabled={busy} bind:this={input} bind:value placeholder="Nombre de la solicitud" spellcheck="false" />
        <label for="dlg-url" class="field-label">URL</label>
        <div class="create-address">
          {#if type === "http"}<span class="method-slot"><MethodSelect value={method} onchange={(value) => (method = value)} /></span>{:else if type === "graphql"}<span class="fixed-method">POST</span>{:else}<span class="fixed-method proto">{type === "websocket" ? "WS" : "SSE"}</span>{/if}
          <input id="dlg-url" class="mono" bind:value={url} placeholder={type === "websocket" ? "wss://ejemplo.com/socket" : type === "graphql" ? "{{base_url}}/graphql" : type === "sse" ? "{{base_url}}/events" : "URL de la solicitud"} disabled={busy} spellcheck="false" />
        </div>
        {#if showOptions}
          <div class="options fade-in">
            <label for="dlg-file" class="field-label">Nombre de archivo</label>
            <div class="file-field">
              <input id="dlg-file" class="mono" value={fileValue} oninput={(e) => { file = e.currentTarget.value; fileTouched = true; }} placeholder="listar-usuarios" disabled={busy} spellcheck="false" />
              <span class="ext mono">.toml</span>
            </div>
            <p class="path-hint mono">requests/{d.folder ? d.folder + "/" : ""}{slug(fileValue) || "…"}.toml</p>
          </div>
        {/if}      {:else if d.kind === "prompt"}
        {#if d.label}<label for="dlg-input" class="muted">{d.label}</label>{/if}
        <input id="dlg-input" disabled={busy} class="mono" bind:this={input} bind:value placeholder={d.placeholder ?? ""} spellcheck="false" />
      {:else}<p>{d.text}</p>{/if}
      {#if error}<p class="error">{error}</p>{/if}
      <div class="actions">
        {#if d.kind === "request"}<button class="options-toggle" class:open={showOptions} onclick={() => (showOptions = !showOptions)} aria-expanded={showOptions} disabled={busy}>Opciones <svg class="caret" width="12" height="12" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><path d="M5 9h14l-7 8z" /></svg></button>{/if}
        <button class="btn" class:text-btn={d.kind === "request"} onclick={cancel} disabled={busy}>Cancelar</button>
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
  .create-address:focus-within, .file-field:focus-within { border-color: var(--accent); }
  .method-slot { display: flex; align-self: stretch; flex: 0 0 auto; border-right: 1px solid var(--border-strong); }
  .method-slot :global(.trigger) { flex: 0 0 auto; width: 104px; margin: 0; padding: 0 14px 0 16px; min-height: 0; border-radius: 0; background: transparent; font-size: 13px; }
  .method-slot :global(.trigger:hover), .method-slot :global(.trigger[aria-expanded="true"]) { background: var(--bg-hover); }
  .create-address { overflow: hidden; height: 40px; }
  .create-address input::placeholder { font-family: var(--font); }
  .dialog:focus, .dialog:focus-visible { outline: none; }
  .new-request > input { border: 1px solid var(--border-strong); border-radius: 6px; padding: 8px 12px; font-size: 13px; }
  .new-request > input:focus, .new-request > input:focus-visible { outline: none; border-color: var(--accent); }
  .new-request .kind-options input {
    appearance: none; -webkit-appearance: none; flex: 0 0 auto; width: 16px; height: 16px; margin: 0; padding: 0;
    border-radius: 50%; border: 0; background: #fff; cursor: pointer; transition: box-shadow .12s, background .12s;
  }
  .new-request .kind-options input:checked { background: #fff; box-shadow: inset 0 0 0 4.5px var(--accent); }
  .new-request .kind-options input:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
  .new-request .kind-options label { cursor: pointer; gap: 10px; }
  .fixed-method { align-self: stretch; display: flex; align-items: center; width: 104px; padding: 0 16px; border-right: 1px solid var(--border-strong); font-size: 13px; }
  .fixed-method.proto { color: var(--accent); }
  .options { display: flex; flex-direction: column; gap: 8px; }
  .file-field { display: flex; align-items: center; min-height: 38px; border: 1px solid var(--border-strong); border-radius: 6px; }
  .file-field input { flex: 1; width: 0; border: 0; outline: none; background: transparent; padding: 8px 10px; }
  .file-field input:focus-visible { outline: none; }
  .ext { padding: 0 10px; color: var(--text-faint); font-size: 12px; }
  .path-hint { font-size: 11px; color: var(--text-faint); }
  .new-request .actions { margin-top: 22px; align-items: center; }
  .options-toggle { margin-right: auto; display: inline-flex; align-items: center; gap: 6px; font-size: 13px; color: var(--accent); padding: 4px 2px; }
  .options-toggle:hover { text-decoration: underline; }
  .caret { flex: 0 0 auto; transition: transform .15s; }
  .options-toggle.open .caret { transform: rotate(180deg); }
  .new-request .btn { min-height: 36px; padding: 0 18px; font-size: 13px; }
  .btn.text-btn { background: transparent; border-color: transparent; }
  .btn.text-btn:hover { background: var(--bg-hover); }
  .new-request .kind-options label { font-size: 13px; color: var(--text); }
  .new-request .request-kinds legend, .new-request .field-label { font-size: 13px; }
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
