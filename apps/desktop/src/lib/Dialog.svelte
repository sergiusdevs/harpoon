<script lang="ts">
  // Diálogo modal: prompt (con campo) o confirm.
  import { store } from "./store.svelte";

  const d = $derived(store.dialog);
  let value = $state("");
  let error = $state<string | null>(null);
  let busy = $state(false);
  let input = $state<HTMLInputElement | null>(null);

  $effect(() => {
    if (d?.kind === "prompt") {
      value = d.value;
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
    try {
      if (d.kind === "prompt") await d.onOk(value);
      else await d.onOk();
      store.dialog = null;
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  function cancel() {
    store.dialog = null;
  }

  function onKey(e: KeyboardEvent) {
    if (!d) return;
    if (e.key === "Escape") cancel();
    if (e.key === "Enter" && !(e.target instanceof HTMLTextAreaElement)) ok();
  }
</script>

<svelte:window onkeydown={onKey} />

{#if d}
  <div class="backdrop" role="presentation" onclick={cancel}>
    <div class="dialog fade-in" role="dialog" aria-modal="true" tabindex="-1" onkeydown={() => {}} onclick={(e) => e.stopPropagation()}>
      <h3>{d.title}</h3>
      {#if d.kind === "prompt"}
        {#if d.label}<label for="dlg-input" class="muted">{d.label}</label>{/if}
        <input id="dlg-input" class="mono" bind:this={input} bind:value placeholder={d.placeholder ?? ""} spellcheck="false" />
      {:else}
        <p>{d.text}</p>
      {/if}
      {#if error}<p class="error">{error}</p>{/if}
      <div class="actions">
        <button class="btn" onclick={cancel}>Cancelar</button>
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
  .dialog {
    width: 420px;
    max-width: calc(100vw - 40px);
    background: var(--bg-raised);
    border: 1px solid var(--border-strong);
    border-radius: 10px;
    padding: 18px;
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
