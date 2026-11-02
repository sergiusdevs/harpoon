<script lang="ts">
  import { store, fmtSize } from "./store.svelte";
  import CodeEditor from "./CodeEditor.svelte";

  type Tab = "body" | "preview" | "headers" | "tests" | "console" | "sent";
  let tab = $state<Tab>("body");

  const x = $derived(store.exchange);
  const statusClass = (s: number) => (s < 300 ? "ok" : s < 400 ? "info" : s < 500 ? "warn" : "err");
  const pretty = $derived.by(() => {
    if (!x) return "";
    if (!x.response.is_json) return x.response.body;
    try {
      return JSON.stringify(JSON.parse(x.response.body), null, 2);
    } catch {
      return x.response.body;
    }
  });
  const checks = $derived(x ? [...x.assertions, ...x.tests] : []);
  const failed = $derived(checks.filter((a) => !a.passed).length);
  const isHtml = $derived(!!x && /text\/html/i.test(x.response.headers["content-type"] ?? ""));
  const isImage = $derived(!!x && /^image\//i.test(x.response.headers["content-type"] ?? ""));
  let copied = $state(false);
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
    <div class="tabs" role="tablist" aria-label="Secciones de la respuesta">
      <button class="tab" role="tab" aria-selected={tab === "body"} class:active={tab === "body"} onclick={() => (tab = "body")}>Respuesta</button>
      {#if isHtml}
        <button class="tab" role="tab" aria-selected={tab === "preview"} class:active={tab === "preview"} onclick={() => (tab = "preview")}>Vista</button>
      {/if}
      <button class="tab" role="tab" aria-selected={tab === "headers"} class:active={tab === "headers"} onclick={() => (tab = "headers")}>Headers <span class="count">{Object.keys(x.response.headers).length}</span></button>
      <button class="tab" role="tab" aria-selected={tab === "tests"} class:active={tab === "tests"} onclick={() => (tab = "tests")}>Tests {#if checks.length}<span class="count" class:bad={failed > 0}>{checks.length}</span>{/if}</button>
      <button class="tab" role="tab" aria-selected={tab === "console"} class:active={tab === "console"} onclick={() => (tab = "console")}>Consola {#if x.logs.length}<span class="count">{x.logs.length}</span>{/if}</button>
      <button class="tab" role="tab" aria-selected={tab === "sent"} class:active={tab === "sent"} onclick={() => (tab = "sent")}>Enviado</button>
    </div>
    <div class="content">
      {#if tab === "body"}
        <div class="body-tools">
          <button class="btn small" onclick={() => copy(pretty)}>{copied ? "Copiado ✓" : "Copiar"}</button>
          {#if isImage}<span class="muted hint">imagen de {fmtSize(x.response.size)} (vista no disponible todavía)</span>{/if}
        </div>
        <div class="body-editor">
          <CodeEditor value={pretty} language={x.response.is_json ? "json" : "text"} readonly />
        </div>
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
        </div>
      {/if}
    </div>
  {:else}
    <div class="empty">
      <svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="var(--text-faint)" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><path d="M4 20L18 6" /><path d="M20 4l-6 1.5 4.5 4.5L20 4z" /></svg>
      <p class="muted">Envía la solicitud para ver la respuesta</p>
      <p class="muted small">⌘↩ para enviar · ⌘S para guardar</p>
    </div>
  {/if}
</section>

<style>
  .pane {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-width: 0;
    background: var(--bg-panel);
    border-left: 1px solid var(--border);
  }
  .summary {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 10px 14px;
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
  .body-tools {
    display: flex;
    gap: 10px;
    align-items: center;
    padding: 6px 10px 0;
  }
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
    white-space: nowrap;
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
