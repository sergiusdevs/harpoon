<script lang="ts">
  import { store, fmtSize } from "../../state/store.svelte";
  import CodeEditor from "../common/CodeEditor.svelte";
  import { formatHtml } from "../../domain/formatHtml";
  import { api } from "../../services/api";
  import HistoryComparison from "./HistoryComparison.svelte";

  import type { Exchange } from "../../domain/types";

  type Tab = "body" | "preview" | "headers" | "cookies" | "tests" | "console" | "sent" | "history";
  let tab = $state<Tab>("body");
  let formatted = $state(true);
  let downloadError = $state("");
  let compareItems = $state<import("../../domain/types").HistoryEntry[]>([]);
  let comparing = $state(false);
  $effect(() => { void store.selected; void store.history; compareItems = []; comparing = false; });
  function toggleCompare(item: import("../../domain/types").HistoryEntry) {
    compareItems = compareItems.includes(item) ? compareItems.filter((entry) => entry !== item) : [...compareItems.slice(-1), item];
  }

  /** Entrada del historial convertida a Exchange para reutilizar la vista. */
  const fromHistory = $derived.by((): Exchange | null => {
    const h = store.viewingHistory;
    if (!h) return null;
    const ct = (h.headers["content-type"] ?? "").toLowerCase();
    return {
      sent: { method: h.method, url: h.url, headers: {}, query: {} },
      response: { status: h.status, headers: h.headers, body: h.body, is_json: ct.includes("json"), elapsed_ms: h.elapsed_ms, size: h.size },
      assertions: [],
      tests: [],
      logs: [],
      vars: {},
      passed: h.checks_total === 0 || h.checks_passed === h.checks_total,
    };
  });
  const x = $derived(fromHistory ?? store.exchange);
  const statusClass = (s: number) => (s < 300 ? "ok" : s < 400 ? "info" : s < 500 ? "warn" : "err");
  const pretty = $derived.by(() => {
    if (!x) return "";
    if (!x.response.is_json) {
      return /text\/html|application\/xhtml\+xml/i.test(x.response.headers["content-type"] ?? "")
        ? formatHtml(x.response.body)
        : x.response.body;
    }
    try {
      return JSON.stringify(JSON.parse(x.response.body), null, 2);
    } catch {
      return x.response.body;
    }
  });
  const shownBody = $derived(x ? (formatted ? pretty : x.response.body) : "");
  const checks = $derived(x ? [...x.assertions, ...x.tests] : []);
  const failed = $derived(checks.filter((a) => !a.passed).length);
  const isHtml = $derived(!!x && /text\/html/i.test(x.response.headers["content-type"] ?? ""));
  const isImage = $derived(!!x && /^image\//i.test(x.response.headers["content-type"] ?? ""));
  const imageSrc = $derived(x && isImage && x.response.body_base64 ? `data:${x.response.headers["content-type"]};base64,${x.response.body_base64}` : null);
  const cookies = $derived.by(() => {
    const raw = x?.response.headers["set-cookie"];
    if (!raw) return [] as { name: string; value: string; attrs: string }[];
    return raw.split("\n").map((line) => {
      const [pair, ...rest] = line.split(";");
      const eq = pair.indexOf("=");
      return { name: pair.slice(0, eq).trim(), value: pair.slice(eq + 1).trim(), attrs: rest.map((a) => a.trim()).join(" · ") };
    });
  });
  const fmtTime = (iso: string) => iso.replace("T", " ").replace("Z", "");
  let copied = $state(false);
  let bodyEditor = $state<CodeEditor | null>(null);
  async function download() {
    if (!x) return;
    try {
      downloadError = "";
      let bytes: Uint8Array;
      if (x.response.is_binary) {
        if (!x.response.body_base64) throw new Error("El contenido binario no está disponible en el historial");
        const decoded = atob(x.response.body_base64);
        bytes = Uint8Array.from(decoded, (character) => character.charCodeAt(0));
      } else bytes = new TextEncoder().encode(x.response.body);
      const ct = (x.response.headers["content-type"] ?? "").toLowerCase();
      const extension = ct.includes("json") ? "json" : ct.includes("html") ? "html" : ct.startsWith("text/") ? "txt" : "bin";
      await (await api).saveResponse(`respuesta.${extension}`, bytes);
    } catch (error) { downloadError = String(error); }
  }
  async function copy(text: string) {
    try {
      await navigator.clipboard.writeText(text);
      copied = true;
      setTimeout(() => (copied = false), 1200);
    } catch {
      /* portapapeles no disponible */
    }
  }
</script>

{#snippet historyList()}
  {#if store.history.length === 0}
    <p class="muted pad">Sin historial para esta solicitud. Se guardan las últimas 50 respuestas en <code class="mono">.harpoon/history/</code> (fuera de Git).</p>
  {:else}
    <div class="hist-tools">
      <span class="muted hint">{store.history.length} respuestas · clic para ver</span>
      <div class="history-actions"><button class="btn small" disabled={compareItems.length !== 2} onclick={() => (comparing = true)}>Comparar ({compareItems.length}/2)</button><button class="btn small" onclick={() => { compareItems = []; store.clearHistory(); }}>Vaciar</button></div>
    </div>
    <ul class="hist">
      {#each store.history as h, i (i + ":" + h.at)}
        <li class="hist-row">
          <input type="checkbox" aria-label={`Seleccionar respuesta ${fmtTime(h.at)} para comparar`} checked={compareItems.includes(h)} onchange={() => toggleCompare(h)} />
          <button class="hist-item" class:active={store.viewingHistory === h} onclick={() => store.showHistory(h)}>
            <span class="status small {statusClass(h.status)}">{h.status}</span>
            <span class="mono when">{fmtTime(h.at)}</span>
            <span class="muted">{h.elapsed_ms} ms · {fmtSize(h.size)}</span>
            {#if h.checks_total}
              <span class="checks" class:bad={h.checks_passed < h.checks_total}>{h.checks_passed}/{h.checks_total}</span>
            {/if}
            {#if h.env}<span class="env-tag">{h.env}</span>{/if}
          </button>
        </li>
      {/each}
    </ul>
  {/if}
{/snippet}

<section class="pane">
  {#if store.sending}
    <div class="empty">
      <div class="spinner"></div>
      <p class="muted">Enviando…</p>
    </div>
  {:else if store.error}
    <div class="empty fade-in">
      <p class="err-title">No se pudo enviar</p>
      <p class="err mono">{store.error}</p>
    </div>
  {:else if x}
    {#if store.viewingHistory}
      <div class="history-banner">
        <span>Viendo respuesta del historial · {fmtTime(store.viewingHistory.at)}{#if store.viewingHistory.env} · {store.viewingHistory.env}{/if}</span>
        <button class="btn small" onclick={() => store.showHistory(null)}>Volver</button>
      </div>
    {/if}
    <div class="response-toolbar">
    <div class="tabs" role="tablist" aria-label="Secciones de la respuesta">
      <button class="tab" role="tab" aria-selected={tab === "body"} class:active={tab === "body"} onclick={() => (tab = "body")}>Respuesta</button>
      {#if isHtml}
        <button class="tab" role="tab" aria-selected={tab === "preview"} class:active={tab === "preview"} onclick={() => (tab = "preview")}>Vista</button>
      {/if}
      <button class="tab" role="tab" aria-selected={tab === "headers"} class:active={tab === "headers"} onclick={() => (tab = "headers")}>Headers <span class="count">{Object.keys(x.response.headers).length}</span></button>
      {#if cookies.length}
        <button class="tab" role="tab" aria-selected={tab === "cookies"} class:active={tab === "cookies"} onclick={() => (tab = "cookies")}>Cookies <span class="count">{cookies.length}</span></button>
      {/if}
      <button class="tab" role="tab" aria-selected={tab === "tests"} class:active={tab === "tests"} onclick={() => (tab = "tests")}>Tests {#if checks.length}<span class="count" class:bad={failed > 0}>{checks.length}</span>{/if}</button>
      <button class="tab" role="tab" aria-selected={tab === "console"} class:active={tab === "console"} onclick={() => (tab = "console")}>Consola {#if x.logs.length}<span class="count">{x.logs.length}</span>{/if}</button>
      <button class="tab" role="tab" aria-selected={tab === "sent"} class:active={tab === "sent"} onclick={() => (tab = "sent")}>Enviado</button>
      <button class="tab" role="tab" aria-selected={tab === "history"} class:active={tab === "history"} onclick={() => (tab = "history")}>Historial {#if store.history.length}<span class="count">{store.history.length}</span>{/if}</button>
    </div>
    <div class="summary fade-in">
      <span class="status {statusClass(x.response.status)}">{x.response.status}</span>
      <span class="meta"><b>{x.response.elapsed_ms}</b> ms</span>
      <span class="meta"><b>{fmtSize(x.response.size)}</b></span>
      {#if checks.length}
        <span class="meta tests" class:bad={failed > 0}>
          {#if failed}✗ {failed}/{checks.length} tests{:else}✓ {checks.length} tests{/if}
        </span>
      {/if}
    </div>
    </div>
    <div class="content">
      {#if tab === "body"}
        <div class="body-tools">
          {#if !x.response.is_binary}
            <div class="segmented" aria-label="Formato de respuesta"><button class:active={formatted} aria-pressed={formatted} onclick={() => (formatted = true)}>Formateado</button><button class:active={!formatted} aria-pressed={!formatted} onclick={() => (formatted = false)}>Original</button></div>
            <button class="btn small" onclick={() => copy(shownBody)}>{copied ? "Copiado ✓" : "Copiar"}</button>
            <button class="btn small" onclick={() => bodyEditor?.openSearch()} title="Buscar en la respuesta (⌘F)">Buscar</button>
          {/if}
          <button class="btn small" onclick={download} disabled={!!store.viewingHistory?.truncated}>Descargar</button>
          {#if store.viewingHistory?.truncated}<span class="muted hint">Contenido incompleto en el historial</span>{/if}
          {#if downloadError}<span class="download-error" role="alert">{downloadError}</span>{/if}
        </div>
        {#if imageSrc}
          <div class="image-wrap">
            <img src={imageSrc} alt="Respuesta ({x.response.headers['content-type']})" />
            <span class="muted hint">{x.response.headers["content-type"]} · {fmtSize(x.response.size)}</span>
          </div>
        {:else if x.response.is_binary}
          <div class="empty">
            <p class="muted">Respuesta binaria ({x.response.headers["content-type"] ?? "tipo desconocido"}, {fmtSize(x.response.size)}).</p>
          </div>
        {:else}
          <div class="body-editor">
            <CodeEditor bind:this={bodyEditor} value={shownBody} language={x.response.is_json ? "json" : "text"} readonly />
          </div>
        {/if}
      {:else if tab === "preview"}
        <!-- sandbox vacío: sin scripts, sin same-origin, sin formularios; nada sale de la app -->
        <iframe class="preview" title="Vista previa HTML" sandbox="" srcdoc={x.response.body}></iframe>
      {:else if tab === "headers"}
        <table class="hdrs">
          <tbody>
            {#each Object.entries(x.response.headers) as [k, v]}
              <tr><td class="k mono">{k}</td><td class="v mono">{v}</td></tr>
            {/each}
          </tbody>
        </table>
      {:else if tab === "cookies"}
        <table class="hdrs">
          <thead><tr><th>Nombre</th><th>Valor</th><th>Atributos</th></tr></thead>
          <tbody>
            {#each cookies as c}
              <tr><td class="k mono">{c.name}</td><td class="v mono">{c.value}</td><td class="v muted">{c.attrs}</td></tr>
            {/each}
          </tbody>
        </table>
      {:else if tab === "history"}
        {@render historyList()}
      {:else if tab === "tests"}
        {#if checks.length === 0}
          <p class="muted pad">Sin comprobaciones. Añade aserciones en <b>Expect</b> o scripts en <b>Tests</b>.</p>
        {:else}
          {#if x.assertions.length}<div class="group muted">EXPECT</div>{/if}
          <ul class="tests-list">
            {#each x.assertions as a}
              <li class:pass={a.passed} class:fail={!a.passed}>
                <span class="mark">{a.passed ? "✓" : "✗"}</span>
                <span class="mono">{a.name}</span>
                {#if a.detail}<span class="detail muted">— {a.detail}</span>{/if}
              </li>
            {/each}
          </ul>
          {#if x.tests.length}<div class="group muted">SCRIPTS</div>{/if}
          <ul class="tests-list">
            {#each x.tests as a}
              <li class:pass={a.passed} class:fail={!a.passed}>
                <span class="mark">{a.passed ? "✓" : "✗"}</span>
                <span>{a.name}</span>
                {#if a.detail}<span class="detail muted mono">— {a.detail}</span>{/if}
              </li>
            {/each}
          </ul>
        {/if}
      {:else if tab === "console"}
        {#if x.logs.length === 0}
          <p class="muted pad">Nada en la consola. Usa <code class="mono">console.log()</code> en los scripts.</p>
        {:else}
          <pre class="console mono">{x.logs.join("\n")}</pre>
        {/if}
      {:else}
        <div class="sent">
          <div class="line mono"><span class="method {x.sent.method}">{x.sent.method}</span> {x.sent.url}{#if Object.keys(x.sent.query).length}?{Object.entries(x.sent.query).map(([k, v]) => `${k}=${v}`).join("&")}{/if}</div>
          <table class="hdrs">
            <tbody>
              {#each Object.entries(x.sent.headers) as [k, v]}
                <tr><td class="k mono">{k}</td><td class="v mono">{v}</td></tr>
              {/each}
            </tbody>
          </table>
          {#if x.sent.body}<pre class="mono body">{x.sent.body}</pre>{/if}
          {#if x.sent.binary_path}<div class="multipart-row mono">Archivo binario: {x.sent.binary_path}</div>{/if}
          {#if x.sent.multipart}
            <div class="multipart-sent"><div class="group muted">MULTIPART</div>{#each x.sent.multipart as part}<div class="multipart-row mono"><span>{part.name}</span><span>{part.type === "file" ? `Archivo: ${part.path}` : part.value}</span></div>{/each}</div>
          {/if}
        </div>
      {/if}
    </div>
  {:else if store.history.length}
    <div class="tabs" role="tablist">
      <button class="tab active" role="tab" aria-selected="true">Historial <span class="count">{store.history.length}</span></button>
    </div>
    <div class="content">{@render historyList()}</div>
  {:else}
    <div class="empty">
      <svg class="empty-plane" width="82" height="82" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.15" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="m22 2-7 20-4-9-9-4 20-7Z"/><path d="M22 2 11 13"/></svg>
      <p class="empty-title">Envía una solicitud para ver la respuesta</p>
      <div class="shortcuts"><span>Enviar solicitud</span><kbd>⌘ ↵</kbd><span>Nueva solicitud</span><kbd>⌘ N</kbd><span>Editar entornos</span><kbd>⌘ E</kbd></div>
    </div>
  {/if}
</section>
{#if comparing && compareItems.length === 2}
  {@const firstIndex = store.history.indexOf(compareItems[0])}
  {@const secondIndex = store.history.indexOf(compareItems[1])}
  <HistoryComparison before={firstIndex > secondIndex ? compareItems[0] : compareItems[1]} after={firstIndex > secondIndex ? compareItems[1] : compareItems[0]} onclose={() => (comparing = false)} />
{/if}

<style>
  .pane {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-width: 0;
    background: var(--bg);
  }
  .response-toolbar { display: flex; align-items: center; min-width: 0; border-bottom: 1px solid var(--border); flex: 0 0 auto; }
  .response-toolbar .tabs { flex: 1; min-width: 0; border-bottom: 0; }
  .response-toolbar .tab { margin-bottom: 0; }
  .empty-heading { padding: 10px 20px; border-bottom: 1px solid var(--border); color: var(--text); font-weight: 600; }
  .summary {
    display: flex;
    align-items: center;
    gap: 14px;
    flex: 0 0 auto;
    min-height: 40px;
    padding: 4px 16px;
    font-size: 12px;
  }
  .status {
    font-family: var(--mono);
    font-weight: 700;
    font-size: 13px;
    padding: 2px 8px;
    border-radius: 4px;
  }
  .status.ok {
    color: var(--ok);
    background: color-mix(in srgb, var(--ok) 14%, transparent);
  }
  .status.info {
    color: var(--accent);
    background: var(--accent-soft);
  }
  .status.warn {
    color: var(--warn);
    background: color-mix(in srgb, var(--warn) 14%, transparent);
  }
  .status.err {
    color: var(--err);
    background: color-mix(in srgb, var(--err) 14%, transparent);
  }
  .meta {
    color: var(--text-muted);
  }
  .meta b {
    color: var(--text);
    font-weight: 500;
  }
  .tests {
    color: var(--ok);
  }
  .tests.bad {
    color: var(--err);
  }
  .count.bad {
    background: color-mix(in srgb, var(--err) 20%, transparent);
    color: var(--err);
  }
  .content {
    flex: 1;
    min-height: 0;
    overflow: auto;
    display: flex;
    flex-direction: column;
  }
  .history-banner {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 6px 14px;
    font-size: 12px;
    background: var(--accent-soft);
    border-bottom: 1px solid var(--border);
  }
  .image-wrap {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 14px;
    background-image: linear-gradient(45deg, var(--bg-active) 25%, transparent 25%), linear-gradient(-45deg, var(--bg-active) 25%, transparent 25%), linear-gradient(45deg, transparent 75%, var(--bg-active) 75%), linear-gradient(-45deg, transparent 75%, var(--bg-active) 75%);
    background-size: 16px 16px;
    background-position: 0 0, 0 8px, 8px -8px, -8px 0;
  }
  .image-wrap img {
    max-width: 100%;
    max-height: 80%;
    object-fit: contain;
    border: 1px solid var(--border);
    background: #fff;
  }
  .hist-tools {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 14px 4px;
  }
  .hist {
    list-style: none;
    margin: 0;
    padding: 4px 8px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .history-actions { display: flex; gap: 6px; }
  .hist-row { display: flex; align-items: center; gap: 4px; }
  .hist-row > input { flex: 0 0 auto; }
  .hist-item {
    display: flex;
    gap: 10px;
    align-items: center;
    width: 100%;
    text-align: left;
    padding: 6px 8px;
    border-radius: var(--radius);
    font-size: 12px;
  }
  .hist-item:hover {
    background: var(--bg-hover);
  }
  .hist-item.active {
    background: var(--accent-soft);
  }
  .status.small {
    font-size: 11px;
    padding: 1px 6px;
  }
  .when {
    color: var(--text);
  }
  .checks {
    color: var(--ok);
    font-size: 11px;
  }
  .checks.bad {
    color: var(--err);
  }
  .env-tag {
    margin-left: auto;
    font-size: 10.5px;
    color: var(--text-muted);
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 0 7px;
  }
  thead th {
    text-align: left;
    font-weight: 500;
    color: var(--text-muted);
    font-size: 11px;
    padding: 6px 14px 2px;
  }
  .body-tools {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    align-items: center;
    padding: 8px 10px;
  }
  .download-error { color: var(--err); font-size: 11px; }
  .body-editor {
    flex: 1;
    min-height: 0;
  }
  .hint {
    font-size: 11.5px;
  }
  .preview {
    flex: 1;
    border: 0;
    background: #fff;
    width: 100%;
    height: 100%;
  }
  .hdrs {
    border-collapse: collapse;
    width: 100%;
    font-size: 12px;
  }
  .hdrs td {
    padding: 5px 14px;
    border-bottom: 1px solid var(--border);
    vertical-align: top;
    user-select: text;
  }
  .hdrs .k {
    color: var(--text-muted);
    width: 34%;
    overflow-wrap: anywhere;
  }
  .hdrs .v {
    word-break: break-all;
  }
  .group {
    font-size: 10.5px;
    letter-spacing: 0.05em;
    padding: 10px 14px 0;
  }
  .console {
    margin: 0;
    padding: 10px 14px;
    font-size: 12px;
    white-space: pre-wrap;
    user-select: text;
  }
  .tests-list {
    list-style: none;
    margin: 0;
    padding: 8px 14px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 12px;
  }
  .tests-list li {
    display: flex;
    gap: 8px;
    align-items: baseline;
  }
  .mark {
    font-weight: 700;
    width: 12px;
  }
  .pass .mark {
    color: var(--ok);
  }
  .fail .mark,
  .fail .mono {
    color: var(--err);
  }
  .sent {
    padding: 10px 0;
    font-size: 12px;
  }
  .multipart-row { display: flex; gap: 14px; padding: 6px 14px; overflow-wrap: anywhere; user-select: text; }
  .sent .line {
    padding: 0 14px 10px;
    word-break: break-all;
    user-select: text;
  }
  .sent .body {
    margin: 10px 14px 0;
    padding: 10px;
    background: var(--bg-raised);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    white-space: pre-wrap;
    user-select: text;
  }
  .empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 24px;
    text-align: center;
  }
  .empty p {
    margin: 0;
  }
  .empty-plane { color: var(--text-faint); opacity: .65; margin-bottom: 18px; }
  .shortcuts { display: grid; grid-template-columns: auto auto; text-align: left; gap: 8px 24px; margin-top: 14px; color: var(--text-muted); font-size: 12px; }
  .shortcuts kbd { font: inherit; color: var(--text); }
  .empty-title { color: var(--text); font-size: 14px; font-weight: 600; }
  .small {
    font-size: 11px;
    color: var(--text-faint);
  }
  .err-title {
    color: var(--err);
    font-weight: 600;
  }
  .err {
    font-size: 12px;
    color: var(--text-muted);
    user-select: text;
    max-width: 90%;
    word-break: break-word;
  }
  .pad {
    padding: 14px;
  }
  .spinner {
    width: 22px;
    height: 22px;
    border: 2px solid var(--border-strong);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
