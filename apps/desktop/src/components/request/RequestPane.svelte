<script lang="ts">
  import { store } from "../../state/store.svelte";
  import { METHODS, type Auth, type Body } from "../../domain/types";
  import CodeEditor from "../common/CodeEditor.svelte";
  import KeyValue from "../common/KeyValue.svelte";
  import AuthForm from "./AuthForm.svelte";
  import MethodBadge from "../common/MethodBadge.svelte";
  import VariableInput from "../common/VariableInput.svelte";
  import { api, isMock } from "../../services/api";
  import ResponsePane from "../response/ResponsePane.svelte";
  import Splitter from "../common/Splitter.svelte";
  import { folderChain } from "../../domain/paths";
  import { variableSuggestions } from "../../domain/variables";
  import { parseCurl } from "../../domain/curl";
  import CurlExport from "./CurlExport.svelte";
  import MultipartEditor from "./MultipartEditor.svelte";
  let exportingCurl = $state(false);

  let editorWidth = $state(0.48);
  let panels = $state<HTMLDivElement>();
  function resizePanels(delta: number) {
    if (panels?.clientWidth) editorWidth = Math.min(0.8, Math.max(0.2, editorWidth + delta / panels.clientWidth));
  }

  const headerSuggestions = [
    "Accept", "Accept-Charset", "Accept-Encoding", "Accept-Language", "Authorization",
    "Cache-Control", "Connection", "Content-Encoding", "Content-Language", "Content-Length",
    "Content-Type", "Cookie", "Date", "ETag", "Expect", "Host", "If-Match",
    "If-Modified-Since", "If-None-Match", "If-Unmodified-Since", "Origin", "Pragma",
    "Range", "Referer", "User-Agent", "X-API-Key", "X-Request-ID", "X-Requested-With",
  ];

  type Tab = "params" | "headers" | "body" | "auth" | "expect" | "scripts" | "tests" | "docs";
  let tab = $state<Tab>("params");

  const req = $derived(store.draft);
  const variables = $derived.by(() => {
    const col = store.collection;
    if (!col) return [];
    const layers = [{ source: "Colección", names: Object.keys(col.meta.vars) }];
    for (const folder of folderChain(store.selected ?? "")) {
      layers.push({ source: `Carpeta · ${folder}`, names: Object.keys(col.folder_configs[folder]?.vars ?? {}) });
    }
    const env = store.env ?? col.meta.default_environment;
    if (env) layers.push({ source: `Entorno · ${env}`, names: Object.keys(col.environments[env]?.vars ?? {}) });
    layers.push({ source: "Ejecución", names: Object.keys(store.runtimeVariables) });
    return variableSuggestions(layers);
  });
  let resolvedUrl = $state("");
  let urlError = $state("");
  $effect(() => {
    const root = store.collection?.root;
    const key = store.selected;
    const env = store.env;
    const url = req?.url;
    // La colección cambia tras guardar variables o entornos.
    void store.collection?.folder_configs;
    void store.collection?.environments;
    void store.collection?.meta.vars;
    let active = true;
    if (root && key && url) {
      (async () => {
        try {
          const value = await (await api).resolveUrl(root, key, env, url);
          if (active) { resolvedUrl = value; urlError = ""; }
        } catch (e) { if (active) { resolvedUrl = ""; urlError = String(e); } }
      })();
    } else { resolvedUrl = ""; urlError = ""; }
    return () => { active = false; };
  });
  const count = (o: Record<string, unknown>) => Object.keys(o).length;

  function set<K extends keyof NonNullable<typeof req>>(k: K, v: NonNullable<typeof req>[K]) {
    if (!store.draft) return;
    store.draft[k] = v;
    store.touch();
  }

  function pasteCurl(event: ClipboardEvent) {
    const text = event.clipboardData?.getData("text") ?? "";
    if (!/^\s*curl(?:\s|$)/.test(text)) return;
    event.preventDefault();
    try {
      const imported = parseCurl(text);
      const selected = store.selected;
      const apply = () => {
        if (!store.draft || store.selected !== selected) return;
        Object.assign(store.draft, {
          method: imported.method, url: imported.url, headers: imported.headers,
          query: imported.query, disabled_query: imported.disabled_query, body: imported.body, auth: imported.auth,
        });
        store.touch();
        store.status = { kind: "ok", text: "Solicitud rellenada desde cURL · guarda los cambios" };
      };
      const current = store.draft;
      const hasContent = current && (Object.keys(current.headers).length > 0 || Object.keys(current.query).length > 0 || current.body.type !== "none" || !["none", "inherit"].includes(current.auth.type));
      if (hasContent) {
        store.dialog = {
          kind: "confirm", title: "Importar cURL en esta solicitud",
          text: "Se sustituirán el método, la URL, los parámetros, las cabeceras, el cuerpo y la autenticación actuales.",
          ok: "Sustituir", onOk: apply,
        };
      } else apply();
    } catch (error) {
      store.status = { kind: "err", text: `No se pudo importar cURL: ${error instanceof Error ? error.message : String(error)}` };
    }
  }

  const bodyText = $derived(
    !req ? "" : req.body.type === "json" ? req.body.json : req.body.type === "text" ? req.body.text : req.body.type === "xml" ? req.body.xml : "",
  );
  const bodyKind = $derived(req?.body.type ?? "none");

  function setBodyKind(kind: Body["type"]) {
    const text = bodyText;
    const next: Body =
      kind === "json" ? { type: "json", json: text } : kind === "graphql" ? { type: "graphql", query: "", variables: "{}", operation_name: "" } : kind === "text" ? { type: "text", text } : kind === "xml" ? { type: "xml", xml: text } : kind === "binary" ? { type: "binary", path: "" } : kind === "form" ? { type: "form", fields: {} } : kind === "multipart" ? { type: "multipart", parts: [] } : { type: "none" };
    set("body", next);
    if (kind === "graphql" && req?.method === "GET") set("method", "POST");
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
    else if (req.body.type === "xml") set("body", { type: "xml", xml: text });
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
      {#if store.dirty}<span class="unsaved">Sin guardar</span>{/if}
      <span class="key mono muted" title="Archivo"><MethodBadge method={req.method} compact={false} /> {store.selected}.toml</span>
    </div>
    <div class="urlbar">
      <div class="address">
        <div class="method-wrap">
          <select aria-label="Método HTTP" class="method-select method {req.method}" value={req.method} onchange={(e) => set("method", e.currentTarget.value as typeof req.method)}>
            {#each METHODS as m}<option value={m}>{m}</option>{/each}
          </select>
          <svg class="method-chevron" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="m6 9 6 6 6-6" /></svg>
        </div>
        <div class="url">
          <VariableInput
            value={req.url}
            oninput={(e) => set("url", e.currentTarget.value)}
            onkeydown={(e) => e.key === "Enter" && store.send()}
            onpaste={pasteCurl}
            placeholder={'{{base_url}}/ruta'}
            label="URL de la solicitud"
            integrated
            {variables}
          />
        </div>
        <button class="btn icon save" onclick={() => store.save()} disabled={!store.dirty} title="Guardar (⌘S)" aria-label="Guardar solicitud">
          <svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M5 3h12l4 4v14H3V3h2z"/><path d="M7 3v6h10V4M7 21v-8h10v8"/></svg>
        </button>
        <button class="btn icon save" title="Exportar como cURL" aria-label="Exportar como cURL" onclick={() => (exportingCurl = true)}>
          <svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="m8 8-4 4 4 4m8-8 4 4-4 4m-2-12-4 16" /></svg>
        </button>
      </div>
      <button class="btn variables-button" onclick={() => (store.modal = "variables")} title="Ver variables de esta solicitud">Variables</button>
      <button class="btn primary send" onclick={() => store.send()} disabled={store.sending}>
        {#if store.sending}<span class="pulse">Enviando</span>{:else}Enviar{/if}
      </button>
    </div>

    {#if req.url.includes("{{")}
      <div class="resolved-url" title={urlError || resolvedUrl}>
        <span class="muted">URL resuelta</span>
        <span class:unresolved={!!urlError} class="mono">{urlError || resolvedUrl || "Resolviendo…"}</span>
      </div>
    {/if}
    <div class="panels" bind:this={panels}>
      <div class="request-editor" style:width={`${editorWidth * 100}%`}>
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
        <KeyValue {variables} disabledKeys={req.disabled_query} onDisabledChange={(keys) => set("disabled_query", keys)} entries={req.query} onchange={(v) => set("query", v)} keyPlaceholder="parámetro" addLabel="Añadir parámetro" />
      {:else if tab === "headers"}
        <KeyValue {variables} entries={req.headers} onchange={(v) => set("headers", v)} keyPlaceholder="Cabecera" valuePlaceholder="valor" addLabel="Añadir cabecera" suggestions={headerSuggestions} />
      {:else if tab === "body"}
        <div class="body">
          <div class="body-bar">
            <div class="segmented" role="radiogroup" aria-label="Tipo de cuerpo">
              {#each [["none", "Sin cuerpo"], ["json", "JSON"], ["graphql", "GraphQL"], ["text", "Texto"], ["xml", "XML"], ["binary", "Binario"], ["form", "Formulario"], ["multipart", "Multipart"]] as [k, label]}
                <button role="radio" aria-checked={bodyKind === k} class:active={bodyKind === k} onclick={() => setBodyKind(k as Body["type"])}>{label}</button>
              {/each}
            </div>
            {#if bodyKind === "json"}
              <button class="btn small" onclick={formatJson} title="Reindentar el JSON">Formatear</button>
              {#if jsonError}<span class="json-error" role="alert">{jsonError}</span>{/if}
            {/if}
          </div>
          {#if bodyKind === "json" || bodyKind === "text" || bodyKind === "xml"}
            <div class="editor-wrap">
              <CodeEditor value={bodyText} language={bodyKind === "json" ? "json" : "text"} onchange={setBodyText} />
            </div>
          {:else if req.body.type === "graphql"}
            <div class="graphql-editor">
              <label>Nombre de operación <span class="muted">opcional</span><input value={req.body.operation_name ?? ""} placeholder="Nombre de operación" oninput={(e) => { if (req.body.type === "graphql") set("body", { ...req.body, operation_name: e.currentTarget.value }); }} /></label>
              <div class="graphql-label">Consulta</div>
              <div class="graphql-query"><CodeEditor value={req.body.query} language="text" onchange={(value) => { if (req.body.type === "graphql") set("body", { ...req.body, query: value }); }} /></div>
              <div class="graphql-label">Variables <span class="muted">JSON</span></div>
              <div class="graphql-vars"><CodeEditor value={req.body.variables ?? ""} language="json" onchange={(value) => { if (req.body.type === "graphql") set("body", { ...req.body, variables: value }); }} /></div>
            </div>
          {:else if bodyKind === "binary"}
            <div class="binary-editor">
              <p class="muted">Selecciona un archivo o escribe una ruta relativa a la colección.</p>
              <div class="binary-row"><VariableInput value={req.body.type === "binary" ? req.body.path : ""} {variables} label="Ruta del archivo binario" placeholder="archivos/datos.bin" oninput={(e) => set("body", { type: "binary", path: e.currentTarget.value })} /><button class="btn small" disabled={isMock} onclick={async () => { const path = await (await api).pickFile(); if (path && store.draft?.body.type === "binary") set("body", { type: "binary", path }); }}>Elegir…</button></div>
            </div>
          {:else if bodyKind === "form"}
            <KeyValue {variables} entries={req.body.type === "form" ? req.body.fields : {}} onchange={(fields) => set("body", { type: "form", fields })} keyPlaceholder="campo" addLabel="Añadir campo" />
          {:else if bodyKind === "multipart"}
            <MultipartEditor parts={req.body.type === "multipart" ? req.body.parts : []} {variables} onchange={(parts) => set("body", { type: "multipart", parts })} />
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
          <KeyValue entries={req.expect.headers} onchange={(headers) => set("expect", { ...req.expect, headers })} keyPlaceholder="Cabecera" suggestions={headerSuggestions} />
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
      </div>
      <Splitter onresize={resizePanels} />
      <div class="response-panel"><ResponsePane /></div>
    </div>
  </section>
{:else}
  <section class="pane center">
    <div class="empty-icon" aria-hidden="true">↗</div>
    <p class="empty-title">Tu espacio de trabajo</p>
    <p class="muted empty-hint">Selecciona una solicitud de la colección para empezar.</p>
  </section>
{/if}

{#if exportingCurl}<CurlExport onclose={() => (exportingCurl = false)} />{/if}

<style>
  .pane {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-width: 0;
  }
  .panels { flex: 1; display: flex; min-height: 0; min-width: 0; padding: 0 20px 16px; }
  .request-editor { display: flex; flex-direction: column; flex: 0 0 auto; min-height: 0; min-width: 0; overflow: hidden; }
  .response-panel { flex: 1; min-height: 0; min-width: 0; }
  .center {
    align-items: center;
    justify-content: center;
  }
  .empty-icon {
    display: grid; place-items: center; width: 42px; height: 42px;
    border: 1px solid var(--border-strong); border-radius: 12px;
    color: var(--accent); font-size: 22px; margin-bottom: 10px;
  }
  .empty-title { margin: 0; font-size: 14px; font-weight: 600; }
  .empty-hint { margin: 6px 24px 0; text-align: center; }
  .namebar {
    flex: 0 0 auto;
    display: flex;
    align-items: baseline;
    gap: 10px;
    padding: 12px 20px 0;
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
  .unsaved { color: var(--warn); font-size: 11px; white-space: nowrap; }
  .key {
    font-size: 11px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 45%;
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }
  .urlbar {
    flex: 0 0 auto;
    display: flex;
    gap: 8px;
    padding: 10px 20px 12px;
    align-items: center;
    flex-wrap: wrap;
  }
  .address { display: flex; flex: 1 1 240px; min-width: 0; align-items: center; border: 1px solid var(--border-strong); border-radius: var(--radius); background: var(--bg); }
  .address:focus-within { border-color: var(--accent); }
  .address .save { margin: 0 6px; flex: 0 0 auto; }
  .send { justify-content: center; min-width: 92px; min-height: 38px; }
  .resolved-url { flex: 0 0 auto; display: flex; gap: 8px; padding: 0 20px 9px; font-size: 11px; min-width: 0; }
  .resolved-url span:last-child { color: var(--text-muted); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; user-select: text; }
  .resolved-url span.unresolved { color: var(--warn); }
  .binary-editor p { font-size: 12px; margin: 4px 0 10px; }
  .binary-row { display: flex; align-items: center; gap: 8px; }
  .body-bar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
  }
  .body-bar .segmented { max-width: 100%; overflow-x: auto; }
  .body-bar .segmented button { white-space: nowrap; }
  .graphql-editor { display: flex; flex-direction: column; gap: 7px; flex: 1; min-height: 360px; }
  .graphql-editor label { display: flex; flex-direction: column; gap: 5px; font-size: 12px; }
  .graphql-label { font-size: 12px; color: var(--text-muted); }
  .graphql-query, .graphql-vars { min-height: 130px; flex: 1; border: 1px solid var(--border); border-radius: var(--radius); overflow: hidden; background: var(--bg-raised); }
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
  .method-wrap { position: relative; flex: 0 0 96px; display: flex; align-items: center; }
  .method-chevron { position: absolute; right: 10px; color: var(--text-muted); pointer-events: none; }
  .method-select {
    appearance: none;
    -webkit-appearance: none;
    width: 100%;
    padding: 10px 26px 10px 12px;
    cursor: pointer;
    border: 0;
    background: transparent;
    box-shadow: none;
    font-weight: 700;
  }
  .method-select:focus-visible { outline: none; box-shadow: none; }
  .url {
    flex: 1 1 140px;
    min-width: 100px;
  }
  .content {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 16px 20px 20px;
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
