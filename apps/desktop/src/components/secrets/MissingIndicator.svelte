<script lang="ts">
  // Aviso discreto en la barra de título: secretos sin valor en esta máquina.
  import { store } from "../../state/store.svelte";
  import Icon from "../common/Icon.svelte";
  const count = $derived(store.missingSecrets.length);
</script>

{#if count}
  <button class="missing" title="Rellenar los secretos que faltan" onclick={() => (store.modal = "missing-secrets")}>
    <Icon name="lock" size={13} /><span>{count} {count === 1 ? "secreto sin valor" : "secretos sin valor"}</span>
  </button>
{/if}

<style>
  .missing { display: inline-flex; align-items: center; gap: 6px; height: 26px; padding: 0 9px; border-radius: 999px; font-size: 11.5px; color: var(--warn); background: color-mix(in srgb, var(--warn) 12%, transparent); border: 1px solid color-mix(in srgb, var(--warn) 30%, transparent); white-space: nowrap; -webkit-app-region: no-drag; }
  .missing:hover { background: color-mix(in srgb, var(--warn) 20%, transparent); }
</style>
