<script lang="ts">
  import { onMount } from "svelte";
  import { store } from "../../state/store.svelte";
  import { api } from "../../services/api";
  import type { StoredCookie } from "../../domain/types";
  import Modal from "../common/Modal.svelte";

  let items = $state<StoredCookie[]>([]);
  let url = $state("");
  let cookie = $state("");
  let visible = $state(false);
  let error = $state("");
  let busy = $state(false);
  const root = store.collection!.root;
  async function refresh() {
    try { items = await (await api).listCookies(root); error = ""; }
    catch (e) { error = String(e); }
  }
  onMount(() => { void refresh(); });
  async function add() {
    try { busy = true; error = ""; await (await api).addCookie(root, url, cookie); cookie = ""; await refresh(); }
    catch (e) { error = String(e); } finally { busy = false; }
  }
  function remove(item: StoredCookie) {
    store.dialog = { kind: "confirm", title: "Eliminar cookie", text: `Se eliminará ${item.name} de ${item.domain}${item.path}.`, ok: "Eliminar", danger: true, onOk: async () => { await (await api).removeCookie(root, item.domain, item.path, item.name); await refresh(); } };
  }
</script>
<Modal title="Cookies de la colección" width={800} onclose={() => (store.modal = null)}>
  <p class="hint muted">Las cookies se guardan localmente en .harpoon y se envían según su dominio, ruta y atributos.</p>
  <div class="tools"><span class="muted">{items.length} cookies</span><button class="btn small" onclick={() => (visible = !visible)}>{visible ? "Ocultar valores" : "Mostrar valores"}</button></div>
  <div class="list">{#if items.length}<table><thead><tr><th>Nombre</th><th>Valor</th><th>Dominio y ruta</th><th>Atributos</th><th></th></tr></thead><tbody>{#each items as item}<tr><td class="mono">{item.name}</td><td class="mono value">{visible ? item.value : "••••••••"}</td><td class="mono">{item.domain}{item.path}</td><td>{[item.secure && "Secure", item.http_only && "HttpOnly", item.session && "Sesión"].filter(Boolean).join(" · ")}</td><td><button class="btn icon" aria-label={`Eliminar cookie ${item.name}`} onclick={() => remove(item)}>✕</button></td></tr>{/each}</tbody></table>{:else}<p class="muted hint">Aún no hay cookies guardadas.</p>{/if}</div>
  <div class="add"><b>Añadir cookie</b><label>URL de origen<input bind:value={url} placeholder="https://api.example.com/" /></label><label>Cabecera Set-Cookie<input bind:value={cookie} class="mono" placeholder="session=valor; Path=/; Secure; HttpOnly" /></label><button class="btn small" disabled={busy || !url.trim() || !cookie.trim()} onclick={add}>Añadir</button></div>
  {#if error}<p class="error" role="alert">{error}</p>{/if}
  {#snippet footer()}<button class="btn" onclick={() => (store.modal = null)}>Cerrar</button>{/snippet}
</Modal>
<style>
  .hint { margin: 0 0 12px; font-size: 12px; } .tools { display: flex; justify-content: space-between; align-items: center; font-size: 12px; }
  .list { max-height: 260px; overflow: auto; margin: 8px 0 16px; } table { width: 100%; border-collapse: collapse; font-size: 12px; text-align: left; }
  td, th { padding: 7px; border-bottom: 1px solid var(--border); overflow-wrap: anywhere; } th { color: var(--text-muted); font-weight: 500; } .value { user-select: text; }
  .add, label { display: flex; flex-direction: column; gap: 7px; } .add { gap: 10px; } label { font-size: 12px; color: var(--text-muted); } .add button { align-self: flex-start; }
  .error { color: var(--err); font-size: 12px; }
</style>
