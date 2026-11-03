<script lang="ts">
  // Botón de candado junto a un valor escrito tal cual: pide nombre y ámbito y
  // lo convierte en variable secreta (en la solicitud queda {{nombre}}).
  import { tick } from "svelte";
  import { store } from "../../state/store.svelte";
  import { isValidVarName, suggestSecretName } from "../../domain/secrets";
  import { modalFocus } from "../../domain/modalFocus";
  import Icon from "../common/Icon.svelte";
  import { envVarFor, scopeOptions } from "./scope";

  interface Props {
    /** Ubicación como la del detector: "header Authorization", "auth.token"… */
    location: string;
    suggestion?: string;
  }
  let { location, suggestion }: Props = $props();
  const id = $props.id();
  let button = $state<HTMLButtonElement>();
  let nameInput = $state<HTMLInputElement>();
  let open = $state(false);
  let name = $state("");
  let scopeId = $state("");
  let busy = $state(false);
  let error = $state("");
  let pos = $state({ left: 0, top: 0 });
  const options = $derived(scopeOptions());

  async function show() {
    const rect = button!.getBoundingClientRect();
    const width = 300;
    pos = { left: Math.max(8, Math.min(rect.right - width, window.innerWidth - width - 8)), top: Math.min(rect.bottom + 6, window.innerHeight - 220) };
    name = suggestion ?? suggestSecretName(location); scopeId = options[0]?.id ?? "collection"; error = ""; open = true;
    await tick(); nameInput?.select();
  }
  function close() { open = false; button?.focus(); }
  async function save() {
    const n = name.trim();
    if (!isValidVarName(n)) { error = "Usa letras, números y _ (sin espacios)."; return; }
    const scope = options.find((o) => o.id === scopeId)?.scope ?? { kind: "collection" as const };
    busy = true; error = "";
    try {
      const ok = await store.convertFindingToSecret({ file: "", location, kind: "", preview: "", suggestion: n }, n, scope);
      if (ok) close(); else error = "No se pudo convertir: edítalo a mano (usa {{variable}}).";
    } catch (cause) { error = cause instanceof Error ? cause.message : String(cause); }
    finally { busy = false; }
  }
</script>

<button type="button" class="lock" bind:this={button} title="Guardar como secreto" aria-label="Guardar como secreto" aria-haspopup="dialog" aria-expanded={open} onclick={() => (open ? close() : show())}>
  <Icon name="lock" size={13} />
</button>
{#if open}
  <div class="backdrop" role="presentation" onclick={close}></div>
  <div class="pop fade-in" role="dialog" aria-modal="true" aria-labelledby={`${id}-title`} tabindex="-1" style:left={`${pos.left}px`} style:top={`${pos.top}px`} use:modalFocus={{ onEscape: close }} onkeydown={() => {}}>
    <div class="title" id={`${id}-title`}><Icon name="lock" size={13} /> Guardar como secreto</div>
    <label><span>Variable</span><input class="mono" bind:this={nameInput} bind:value={name} spellcheck="false" autocomplete="off" onkeydown={(e) => e.key === "Enter" && save()} disabled={busy} /></label>
    <label><span>Guardar en</span><select bind:value={scopeId} disabled={busy}>{#each options as o}<option value={o.id}>{o.label}</option>{/each}</select></label>
    <p class="hint">En Git quedará <code>{`{{${name.trim() || "nombre"}}}`}</code>; el valor, solo en este equipo (o en <code>{envVarFor(name.trim() || "nombre")}</code>).</p>
    {#if error}<p class="error" role="alert">{error}</p>{/if}
    <div class="actions"><button class="btn small" onclick={close} disabled={busy}>Cancelar</button><button class="btn small primary" onclick={save} disabled={busy || !name.trim()}>Guardar secreto</button></div>
  </div>
{/if}

<style>
  .lock { display: inline-flex; align-items: center; justify-content: center; width: 24px; height: 24px; flex: 0 0 24px; border-radius: 4px; color: var(--warn); }
  .lock:hover, .lock[aria-expanded="true"] { background: color-mix(in srgb, var(--warn) 16%, transparent); }
  .backdrop { position: fixed; inset: 0; z-index: 60; }
  .pop { position: fixed; z-index: 61; width: 300px; display: flex; flex-direction: column; gap: 9px; padding: 12px; border: 1px solid var(--border-strong); border-radius: 9px; background: var(--bg-raised); box-shadow: 0 12px 32px rgba(0, 0, 0, 0.28); font-size: 12px; }
  .pop:focus { outline: none; }
  .title { display: flex; align-items: center; gap: 6px; font-weight: 600; color: var(--text); }
  label { display: grid; grid-template-columns: 74px 1fr; align-items: center; gap: 8px; }
  label span { color: var(--text-muted); }
  input, select { font-size: 12px; min-width: 0; }
  .hint { margin: 0; color: var(--text-muted); font-size: 11px; line-height: 1.5; }
  code { font: 10.5px var(--mono); background: var(--bg-active); padding: 0 4px; border-radius: 3px; }
  .error { margin: 0; color: var(--err); font-size: 11px; }
  .actions { display: flex; justify-content: flex-end; gap: 6px; }
</style>
