<script lang="ts">
  import { store } from "./store.svelte";
  import { METHODS, type Auth, type Body } from "./types";
  import CodeEditor from "./CodeEditor.svelte";
  import KeyValue from "./KeyValue.svelte";
  import AuthForm from "./AuthForm.svelte";
  import MethodBadge from "./MethodBadge.svelte";

  type Tab = "params" | "headers" | "body" | "auth" | "expect" | "scripts" | "tests" | "docs";
  let tab = $state<Tab>("params");

  const req = $derived(store.draft);
  const count = (o: Record<string, unknown>) => Object.keys(o).length;

  function set<K extends keyof NonNullable<typeof req>>(k: K, v: NonNullable<typeof req>[K]) {
    if (!store.draft) return;
    store.draft[k] = v;
    store.touch();
  }

  const bodyText = $derived(
    !req ? "" : req.body.type === "json" ? req.body.json : req.body.type === "text" ? req.body.text : "",
  );
  const bodyKind = $derived(req?.body.type ?? "none");

  function setBodyKind(kind: Body["type"]) {
    const text = bodyText;
    const next: Body =
      kind === "json" ? { type: "json", json: text } : kind === "text" ? { type: "text", text } : kind === "form" ? { type: "form", fields: {} } : { type: "none" };
    set("body", next);
  }

  let jsonError = $state<string | null>(null);
  $effect(() => {
    if (bodyKind !== "json" || !bodyText.trim()) { jsonError = null; return; }
    try { JSON.parse(bodyText); jsonError = null; } catch (e) { jsonError = String(e).replace("SyntaxError: ", ""); }
  });

  function formatJson() {
    try {
      setBodyText(JSON.stringify(JSON.parse(bodyText), null, 2));
    } catch {
      /* el aviso ya se muestra */
    }
  }

  function setBodyText(text: string) {
    if (!req) return;
    if (req.body.type === "json") set("body", { type: "json", json: text });
    else if (req.body.type === "text") set("body", { type: "text", text });
  }

  function onKey(e: KeyboardEvent) {
    if ((e.metaKey || e.ctrlKey) && e.key === "Enter") {
      e.preventDefault();
      store.send();
    } else if ((e.metaKey || e.ctrlKey) && e.key === "s") {
      e.preventDefault();
      store.save();
    }
  }
</script>

<svelte:window onkeydown={onKey} />

{#if req}
  <section class="pane" aria-label="Solicitud">
    <div class="namebar">
      <input
        class="name-input"
        value={req.name}
        oninput={(e) => set("name", e.currentTarget.value)}
        placeholder="Nombre de la solicitud"
        aria-label="Nombre de la solicitud"
        spellcheck="false"
      />
      <span class="key mono muted" title="Archivo"><MethodBadge method={req.method} compact={false} /> {store.selected}.toml</span>
    </div>
    <div class="urlbar">
      <select class="method-select method {req.method}" value={req.method} onchange={(e) => set("method", e.currentTarget.value as typeof req.method)}>
        {#each METHODS as m}<option value={m}>{m}</option>{/each}
      </select>
      <input
        class="url mono"
        value={req.url}
        oninput={(e) => set("url", e.currentTarget.value)}
        title={req.name}
        onkeydown={(e) => e.key === "Enter" && store.send()}
        placeholder="{'{{base_url}}'}/ruta"
        spellcheck="false"
      />
      <button class="btn primary" onclick={() => store.send()} disabled={store.sending}>
        {#if store.sending}<span class="pulse">Enviando</span>{:else}Enviar{/if}
        <kbd>⌘↩</kbd>
      </button>
      <button class="btn" onclick={() => store.save()} disabled={!store.dirty} title="Guardar (⌘S)">Guardar</button>
    </div>

    <div class="tabs" role="tablist" aria-label="Secciones de la solicitud">
      <button class="tab" role="tab" aria-selected={tab === "params"} class:active={tab === "params"} onclick={() => (tab = "params")}>Params {#if count(req.query)}<span class="count">{count(req.query)}</span>{/if}</button>
      <button class="tab" role="tab" aria-selected={tab === "headers"} class:active={tab === "headers"} onclick={() => (tab = "headers")}>Headers {#if count(req.headers)}<span class="count">{count(req.headers)}</span>{/if}</button>
      <button class="tab" role="tab" aria-selected={tab === "body"} class:active={tab === "body"} onclick={() => (tab = "body")}>Body {#if bodyKind !== "none"}<span class="count">{bodyKind}</span>{/if}</button>
      <button class="tab" role="tab" aria-selected={tab === "auth"} class:active={tab === "auth"} onclick={() => (tab = "auth")}>Auth {#if req.auth.type !== "inherit"}<span class="count">{req.auth.type}</span>{/if}</button>
      <button class="tab" role="tab" aria-selected={tab === "expect"} class:active={tab === "expect"} onclick={() => (tab = "expect")}>Expect</button>
      <button class="tab" role="tab" aria-selected={tab === "scripts"} class:active={tab === "scripts"} onclick={() => (tab = "scripts")}>Scripts {#if req.scripts.pre || req.scripts.post}<span class="count">js</span>{/if}</button>
      <button class="tab" role="tab" aria-selected={tab === "tests"} class:active={tab === "tests"} onclick={() => (tab = "tests")}>Tests {#if req.scripts.test}<span class="count">js</span>{/if}</button>
      <button class="tab" role="tab" aria-selected={tab === "docs"} class:active={tab === "docs"} onclick={() => (tab = "docs")}>Docs</button>
    </div>

    <div class="content">
      {#if tab === "params"}
        <KeyValue entries={req.query} onchange={(v) => set("query", v)} keyPlaceholder="parámetro" addLabel="Añadir parámetro" />
      {:else if tab === "headers"}
        <KeyValue entries={req.headers} onchange={(v) => set("headers", v)} keyPlaceholder="Cabecera" valuePlaceholder="valor" addLabel="Añadir cabecera" />
      {:else if tab === "body"}
        <div class="body">
          <div class="body-bar">
            <div class="segmented" role="radiogroup" aria-label="Tipo de cuerpo">
              {#each [["none", "Sin cuerpo"], ["json", "JSON"], ["text", "Texto"], ["form", "Formulario"]] as [k, label]}
                <button role="radio" aria-checked={bodyKind === k} class:active={bodyKind === k} onclick={() => setBodyKind(k as Body["type"])}>{label}</button>
              {/each}
            </div>
            {#if bodyKind === "json"}
              <button class="btn small" onclick={formatJson} title="Reindentar el JSON">Formatear</button>
              {#if jsonError}<span class="json-error" role="alert">{jsonError}</span>{/if}
            {/if}
          </div>
          {#if bodyKind === "json" || bodyKind === "text"}
            <div class="editor-wrap">
              <CodeEditor value={bodyText} language={bodyKind === "json" ? "json" : "text"} onchange={setBodyText} />
            </div>
          {:else if bodyKind === "form"}
            <KeyValue entries={req.body.type === "form" ? req.body.fields : {}} onchange={(fields) => set("body", { type: "form", fields })} keyPlaceholder="campo" addLabel="Añadir campo" />
          {:else}
            <div class="nobody">
              <p class="muted">Esta solicitud no envía cuerpo.</p>
              <button class="btn" onclick={() => { setBodyKind("json"); }}>Añadir cuerpo JSON</button>
            </div>
          {/if}
        </div>
      {:else if tab === "auth"}
        <AuthForm
          auth={req.auth}
          onchange={(a: Auth) => set("auth", a)}
          inheritHint={`Usa la auth definida en harpoon.toml (${store.collection?.meta.auth.type ?? "none"}).`}
        />
      {:else if tab === "expect"}
        <div class="form">
          <label><span>Status</span><input class="mono" type="number" value={req.expect.status ?? ""} oninput={(e) => set("expect", { ...req.expect, status: e.currentTarget.value ? Number(e.currentTarget.value) : undefined })} placeholder="200" /></label>
          <label><span>Tiempo máx. (ms)</span><input class="mono" type="number" value={req.expect.max_time_ms ?? ""} oninput={(e) => set("expect", { ...req.expect, max_time_ms: e.currentTarget.value ? Number(e.currentTarget.value) : undefined })} placeholder="500" /></label>
          <label><span>El cuerpo contiene</span><input class="mono" value={req.expect.body_contains.join(" | ")} oninput={(e) => set("expect", { ...req.expect, body_contains: e.currentTarget.value.split("|").map((s) => s.trim()).filter(Boolean) })} placeholder={'"id" | "name"  (separar con |)'} /></label>
          <div class="sub">Cabeceras esperadas</div>
          <KeyValue entries={req.expect.headers} onchange={(headers) => set("expect", { ...req.expect, headers })} keyPlaceholder="Cabecera" />
        </div>
      {:else if tab === "scripts"}
        <div class="scripts">
          <div class="script">
            <div class="script-head"><span>Pre-request</span><span class="muted">se ejecuta antes de enviar · <code>harpoon.req</code>, <code>harpoon.vars</code></span></div>
            <div class="editor-wrap"><CodeEditor value={req.scripts.pre} language="javascript" onchange={(v) => set("scripts", { ...req.scripts, pre: v })} /></div>
          </div>
          <div class="script">
            <div class="script-head"><span>Post-response</span><span class="muted">después de recibir · <code>harpoon.res.json()</code>, <code>harpoon.vars.set()</code></span></div>
            <div class="editor-wrap"><CodeEditor value={req.scripts.post} language="javascript" onchange={(v) => set("scripts", { ...req.scripts, post: v })} /></div>
          </div>
        </div>
      {:else if tab === "tests"}
        <div class="scripts">
          <div class="script grow">
            <div class="script-head"><span>Tests</span><span class="muted"><code>test("nombre", () =&gt; expect(x).to.equal(y))</code></span></div>
            <div class="editor-wrap"><CodeEditor value={req.scripts.test} language="javascript" onchange={(v) => set("scripts", { ...req.scripts, test: v })} placeholder="test('status 200', () => expect(harpoon.res.status).to.equal(200));" /></div>
          </div>
        </div>
      {:else if tab === "docs"}
        <div class="editor-wrap">
          <CodeEditor value={req.docs} onchange={(v) => set("docs", v)} placeholder="Notas de esta solicitud" />
        </div>
      {/if}
    </div>
  </section>
{:else}
  <section class="pane center">
    <p class="muted">Selecciona una solicitud</p>
  </section>
{/if}

<style>
  .pane {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-width: 0;
  }
  .center {
    align-items: center;
    justify-content: center;
  }
  .namebar {
    display: flex;
    align-items: baseline;
    gap: 10px;
    padding: 10px 12px 0;
  }
  .name-input {
    flex: 1;
    min-width: 0;
    background: transparent;
    border-color: transparent;
    font-size: 14px;
    font-weight: 600;
    padding: 2px 6px;
    margin-left: -6px;
  }
  .name-input:hover {
    border-color: var(--border);
  }
  .key {
    font-size: 11px;
    white-space: nowrap;
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }
  .urlbar {
    display: flex;
    gap: 8px;
    padding: 8px 12px 10px;
    align-items: center;
  }
  .body-bar {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .json-error {
    color: var(--err);
    font-size: 11.5px;
  }
  .nobody {
    display: flex;
    flex-direction: column;
    gap: 8px;
    align-items: flex-start;
    padding: 8px 0;
  }
  .nobody p {
    margin: 0;
  }
  .method-select {
    width: 100px;
    padding: 6px 8px;
    font-weight: 700;
  }
  .url {
    flex: 1;
    min-width: 0;
    padding: 6px 10px;
    font-size: 12.5px;
  }
  kbd {
    font: 10px var(--mono);
    opacity: 0.6;
    margin-left: 2px;
  }
  .content {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 8px 12px 12px;
  }
  .body {
    display: flex;
    flex-direction: column;
    height: 100%;
    gap: 8px;
  }
  .editor-wrap {
    flex: 1;
    min-height: 160px;
    height: 100%;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
    background: var(--bg-raised);
  }
  .scripts {
    display: flex;
    flex-direction: column;
    gap: 10px;
    height: 100%;
  }
  .script {
    display: flex;
    flex-direction: column;
    gap: 4px;
    flex: 1;
    min-height: 120px;
  }
  .script.grow {
    flex: 1;
  }
  .script-head {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    font-size: 12px;
  }
  .script-head .muted {
    font-size: 11px;
  }
  .script-head code {
    font-family: var(--mono);
    font-size: 10.5px;
  }
  .form {
    display: flex;
    flex-direction: column;
    gap: 10px;
    max-width: 560px;
  }
  .form label {
    display: grid;
    grid-template-columns: 140px 1fr;
    align-items: center;
    gap: 10px;
  }
  .form label span {
    color: var(--text-muted);
    font-size: 12px;
  }
  .form input {
    font-size: 12px;
  }
  .sub {
    color: var(--text-muted);
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    margin-top: 6px;
  }
  .hint {
    font-size: 12px;
    margin: 4px 0;
  }
</style>
