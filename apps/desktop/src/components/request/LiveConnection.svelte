<script lang="ts">
  import { store } from "../../state/store.svelte";
  import { streams } from "../../state/streams.svelte";
  import { requestType } from "../../domain/types";
  import { api, isMock } from "../../services/api";
  import Modal from "../common/Modal.svelte";

  const session = $derived(streams.current);
  let busy = $state(false);
  let saved = $state("");
  const stateLabel = { closed: "Desconectado", connecting: "Conectando…", open: "Conectado", closing: "Desconectando…" };

  function updateUrl(url: string) {
    if (!session) return;
    const request = { ...session.request, url };
    streams.manager.update(session, { request, urlEdited: true });
    saved = "";
    if (store.collection?.root === session.root && store.selected === session.key && store.draft) {
      store.draft.url = url;
      store.touch();
    }
  }

  async function save() {
    if (!session || busy) return;
    const current = session;
    busy = true; saved = "";
    try {
      const selected = store.collection?.root === current.root && store.selected === current.key && store.draft;
      const latest = selected ? structuredClone($state.snapshot(store.draft!)) : (await (await api).openCollection(current.root)).requests[current.key];
      if (!latest) throw new Error("La solicitud ya no existe en la colección");
      if (requestType(latest) !== current.protocol) throw new Error("El tipo de la solicitud ha cambiado. Vuelve a abrirla antes de guardar esta conexión.");
      const request = { ...latest, url: current.request.url };
      streams.manager.update(current, { request });
      await (await api).saveRequest(current.root, current.key, request);
      if (store.collection?.root === current.root) {
        store.collection.requests[current.key] = request;
        const cached = store.tabs.views[current.key];
        if (cached && JSON.stringify(cached.draft) === JSON.stringify(request)) cached.dirty = false;
        if (store.selected === current.key && store.draft && JSON.stringify($state.snapshot(store.draft)) === JSON.stringify(request)) { store.dirty = false; if (cached) { cached.dirty = false; cached.draft = request; } }
      }
      streams.manager.update(current, { urlEdited: false });
      saved = "Solicitud guardada";
    } catch (cause) { current.error = String(cause); streams.manager.update(current, {}); }
    finally { busy = false; }
  }
</script>

{#if session}
<Modal title="Conexiones en tiempo real" width={820} onclose={() => (store.modal = null)}>
  <label class="session-picker">Solicitud
    <select value={session.identity} disabled={busy} onchange={(event) => { streams.manager.select(event.currentTarget.value); saved = ""; }}>
      {#each streams.sessions as item (item.identity)}
        <option value={item.identity}>{item.request.name} · {item.protocol === "websocket" ? "WS" : "SSE"} · {stateLabel[item.state]} · {item.root}</option>
      {/each}
    </select>
  </label>
  <div class="top"><strong>{session.protocol === "websocket" ? "WebSocket" : "SSE"}</strong><span class="status" class:connected={session.state === "open"}>{stateLabel[session.state]}</span></div>
  <div class="address">
    <input value={session.request.url} oninput={(event) => updateUrl(event.currentTarget.value)} aria-label="URL de conexión" class="mono" placeholder={session.protocol === "websocket" ? "wss://ejemplo.com/socket" : "https://ejemplo.com/eventos"} disabled={session.state !== "closed" || busy} spellcheck="false" />
    {#if session.state === "closed"}<button class="btn primary" onclick={() => streams.manager.connect(session!)} disabled={!session.request.url.trim() || isMock || busy}>Conectar</button>
    {:else}<button class="btn" onclick={() => streams.manager.disconnect(session!)} disabled={session.state === "closing"}>Desconectar</button>{/if}
    <button class="btn" onclick={save} disabled={busy || isMock || !session.urlEdited}>Guardar</button>
  </div>
  <p class="muted hint">Usa los parámetros, cabeceras y autenticación capturados al abrir esta conexión · entorno: {session.env ?? "predeterminado"}. No ejecuta scripts. Cerrar esta ventana mantiene la conexión.{session.protocol === "sse" ? " SSE se reconecta automáticamente." : ""}</p>
  {#if saved}<p class="muted hint" role="status">{saved}</p>{/if}
  {#if session.notice}<p class="muted hint" aria-live="polite">{session.notice}</p>{/if}
  {#if session.error}<p class="error" role="alert">{session.error}</p>{/if}
  <div class="messages" aria-live="polite">
    {#if !session.entries.length}<p class="muted empty">Los mensajes aparecerán aquí.</p>{/if}
    {#each session.entries as entry (entry.key)}<div class="entry"><div class="entry-head"><span class:sent={entry.kind === "sent"}>{entry.kind === "sent" ? "Enviado" : "Recibido"}</span>{#if entry.event}<span>{entry.event}</span>{/if}{#if entry.id}<span>ID: {entry.id}</span>{/if}<time>{entry.time}</time></div><pre>{entry.data}</pre></div>{/each}
  </div>
  {#if session.protocol === "websocket"}<div class="compose"><textarea value={session.message} oninput={(event) => streams.manager.update(session!, { message: event.currentTarget.value })} aria-label="Mensaje WebSocket" placeholder="Escribe un mensaje de texto…" disabled={session.state !== "open"} onkeydown={(event) => { if ((event.metaKey || event.ctrlKey) && event.key === "Enter") { event.preventDefault(); void streams.manager.send(session!); } }}></textarea><button class="btn primary" disabled={session.state !== "open" || !session.message} onclick={() => streams.manager.send(session!)}>Enviar</button></div>{/if}
  {#snippet footer()}<button class="btn" onclick={() => streams.manager.update(session!, { entries: [] })}>Limpiar mensajes</button><button class="btn" onclick={() => (store.modal = null)}>Cerrar ventana</button>{/snippet}
</Modal>
{/if}
<style>
  .session-picker { display: flex; flex-direction: column; gap: 6px; margin-bottom: 14px; color: var(--text-muted); font-size: 12px; }
  .session-picker select { width: 100%; min-width: 0; }
  .top, .address, .compose { display: flex; align-items: center; gap: 10px; } .top { justify-content: space-between; } .status { color: var(--text-muted); font-size: 12px; } .status.connected { color: var(--ok); }
  .address { flex-wrap: wrap; margin-top: 12px; } .address input { flex: 1; min-width: 0; } .hint, .error { font-size: 12px; line-height: 1.5; } .error { color: var(--err); overflow-wrap: anywhere; }
  .messages { height: min(38vh, 320px); overflow: auto; border: 1px solid var(--border); background: var(--bg); border-radius: 7px; } .empty { padding: 12px; font-size: 12px; }
  .entry { padding: 9px 12px; border-bottom: 1px solid var(--border); } .entry-head { display: flex; gap: 10px; color: var(--text-muted); font-size: 11px; } .entry-head span:first-child { color: var(--accent); } .entry-head .sent { color: var(--ok); } time { margin-left: auto; } pre { white-space: pre-wrap; overflow-wrap: anywhere; margin: 6px 0 0; font: 12px/1.5 var(--mono); }
  .compose { margin-top: 10px; align-items: stretch; } textarea { flex: 1; min-width: 0; resize: vertical; min-height: 52px; }
</style>
