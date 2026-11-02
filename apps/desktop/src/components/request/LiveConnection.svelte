<script lang="ts">
  import { onDestroy } from "svelte";
  import { store } from "../../state/store.svelte";
  import { api, isMock, type StreamEvent } from "../../services/api";
  import Modal from "../common/Modal.svelte";

  type Entry = { key: number; kind: StreamEvent["kind"]; data: string; event: string; id: string; time: string };
  let protocol = $state<"websocket" | "sse">(store.draft?.url.startsWith("ws") ? "websocket" : "sse");
  let url = $state(store.draft?.url ?? "");
  let connectionState = $state<"closed" | "connecting" | "open">("closed");
  let session = $state<string | null>(null);
  let message = $state("");
  let error = $state("");
  let notice = $state("");
  let entries = $state<Entry[]>([]);
  let nextKey = 0;
  let disposed = false;
  function append(event: StreamEvent) {
    if (disposed) return;
    if (event.kind === "open") { connectionState = "open"; notice = ""; }
    if (event.kind === "reconnecting") { connectionState = "connecting"; notice = event.data; }
    if (event.kind === "error") error = event.data;
    if (event.kind === "closed") { connectionState = "closed"; session = null; }
    if (event.kind === "message" || event.kind === "sent") {
      const data = event.data.length > 20000 ? event.data.slice(0, 20000) + "\n… contenido recortado en la vista" : event.data;
      entries = [...entries.slice(-199), { ...event, data, key: nextKey++, time: new Date().toLocaleTimeString() }];
    }
  }
  async function connect() {
    if (!store.collection || !store.draft || !store.selected || connectionState !== "closed") return;
    const root = store.collection.root, key = store.selected, env = store.env;
    const request = structuredClone($state.snapshot(store.draft));
    request.url = url;
    const id = crypto.randomUUID();
    session = id; connectionState = "connecting"; error = ""; notice = ""; entries = [];
    try {
      await (await api).startStream(root, key, env, request, protocol, id, append);
      if (disposed) await (await api).stopStream(id);
    } catch (cause) { if (!disposed) { error = String(cause); connectionState = "closed"; session = null; } }
  }
  async function disconnect() { if (session) try { await (await api).stopStream(session); } catch (cause) { error = String(cause); } }
  async function send() {
    if (!session || !message || connectionState !== "open") return;
    try { await (await api).sendStream(session, message); message = ""; } catch (cause) { error = String(cause); }
  }
  onDestroy(() => { disposed = true; const id = session; if (id) void api.then((bridge) => bridge.stopStream(id)).catch(() => {}); });
</script>
<Modal title="Tiempo real" width={820} onclose={() => (store.modal = null)}>
  <div class="top"><div class="segmented" role="radiogroup" aria-label="Protocolo"><button role="radio" aria-checked={protocol === "websocket"} class:active={protocol === "websocket"} disabled={connectionState !== "closed"} onclick={() => (protocol = "websocket")}>WebSocket</button><button role="radio" aria-checked={protocol === "sse"} class:active={protocol === "sse"} disabled={connectionState !== "closed"} onclick={() => (protocol = "sse")}>SSE</button></div><span class="status" class:connected={connectionState === "open"}>{connectionState === "open" ? "Conectado" : connectionState === "connecting" ? "Conectando…" : "Desconectado"}</span></div>
  <div class="address"><input bind:value={url} aria-label="URL de conexión" class="mono" placeholder={protocol === "websocket" ? "wss://ejemplo.com/socket" : "https://ejemplo.com/eventos"} disabled={connectionState !== "closed"} spellcheck="false" />{#if connectionState === "closed"}<button class="btn primary" onclick={connect} disabled={!url.trim() || isMock}>Conectar</button>{:else}<button class="btn" onclick={disconnect}>Desconectar</button>{/if}</div>
  <p class="muted hint">Usa los parámetros, cabeceras, variables y autenticación compatible de la solicitud actual. No ejecuta scripts.{protocol === "websocket" ? " WebSocket aún no admite OAuth2, Digest, proxy ni TLS personalizado." : " SSE se reconecta automáticamente si la conexión se interrumpe."}</p>
  {#if notice}<p class="muted hint" aria-live="polite">{notice}</p>{/if}
  {#if error}<p class="error" role="alert">{error}</p>{/if}
  <div class="messages" aria-live="polite">
    {#if !entries.length}<p class="muted empty">Los mensajes aparecerán aquí.</p>{/if}
    {#each entries as entry (entry.key)}<div class="entry"><div class="entry-head"><span class:sent={entry.kind === "sent"}>{entry.kind === "sent" ? "Enviado" : "Recibido"}</span>{#if entry.event}<span>{entry.event}</span>{/if}{#if entry.id}<span>ID: {entry.id}</span>{/if}<time>{entry.time}</time></div><pre>{entry.data}</pre></div>{/each}
  </div>
  {#if protocol === "websocket"}<div class="compose"><textarea bind:value={message} aria-label="Mensaje WebSocket" placeholder="Escribe un mensaje de texto…" disabled={connectionState !== "open"} onkeydown={(event) => { if ((event.metaKey || event.ctrlKey) && event.key === "Enter") { event.preventDefault(); void send(); } }}></textarea><button class="btn primary" disabled={connectionState !== "open" || !message} onclick={send}>Enviar</button></div>{/if}
  {#snippet footer()}<button class="btn" onclick={() => { entries = []; }}>Limpiar</button><button class="btn" onclick={() => (store.modal = null)}>Cerrar</button>{/snippet}
</Modal>
<style>
  .top, .address, .compose { display: flex; align-items: center; gap: 10px; } .top { justify-content: space-between; } .status { color: var(--text-muted); font-size: 12px; } .status.connected { color: var(--ok); }
  .address { margin-top: 12px; } .address input { flex: 1; min-width: 0; } .hint, .error { font-size: 12px; line-height: 1.5; } .error { color: var(--err); overflow-wrap: anywhere; }
  .messages { height: min(38vh, 320px); overflow: auto; border: 1px solid var(--border); background: var(--bg); border-radius: 7px; } .empty { padding: 12px; font-size: 12px; }
  .entry { padding: 9px 12px; border-bottom: 1px solid var(--border); } .entry-head { display: flex; gap: 10px; color: var(--text-muted); font-size: 11px; } .entry-head span:first-child { color: var(--accent); } .entry-head .sent { color: var(--ok); } time { margin-left: auto; } pre { white-space: pre-wrap; overflow-wrap: anywhere; margin: 6px 0 0; font: 12px/1.5 var(--mono); }
  .compose { margin-top: 10px; align-items: stretch; } textarea { flex: 1; min-width: 0; resize: vertical; min-height: 52px; }
</style>
