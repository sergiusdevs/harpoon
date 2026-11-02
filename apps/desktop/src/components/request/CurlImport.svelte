<script lang="ts">
  import { store } from "../../state/store.svelte";
  import { parseCurl } from "../../domain/curl";
  import Modal from "../common/Modal.svelte";
  let name = $state("");
  let command = $state("");
  let error = $state("");
  let busy = $state(false);
  async function create() {
    try {
      error = "";
      const request = parseCurl(command);
      busy = true;
      await store.createImportedRequest(name, request);
      store.modal = null;
    } catch (e) { error = e instanceof Error ? e.message : String(e); }
    finally { busy = false; }
  }
</script>
<Modal title="Importar desde cURL" width={650} onclose={() => (store.modal = null)}>
  <div class="form">
    <label>Nombre de la solicitud<input bind:value={name} placeholder="listar-usuarios" /></label>
    <label>Comando cURL<textarea bind:value={command} class="mono" rows="7" placeholder="curl 'https://api.example.com/users' -H 'Accept: application/json'" spellcheck="false"></textarea></label>
    <p class="muted hint">Se admite URL, método, cabeceras, datos y autenticación básica. Las opciones no compatibles muestran un error.</p>
    {#if error}<p class="error" role="alert">{error}</p>{/if}
  </div>
  {#snippet footer()}<button class="btn" onclick={() => (store.modal = null)}>Cancelar</button><button class="btn primary" disabled={busy || !name.trim() || !command.trim()} onclick={create}>Importar</button>{/snippet}
</Modal>
<style>
  .form, label { display: flex; flex-direction: column; gap: 8px; }
  .form { gap: 15px; } label { font-size: 12px; color: var(--text-muted); }
  textarea { resize: vertical; color: var(--text); background: var(--bg); border: 1px solid var(--border); border-radius: var(--radius); padding: 10px; user-select: text; }
  .hint, .error { margin: 0; font-size: 12px; } .error { color: var(--err); }
</style>
