<script lang="ts">
  import { applyUrlInput, composeUrl } from "../../domain/urlQuery";
  import MethodSelect from "./MethodSelect.svelte";
  import VariableInspector from "./VariableInspector.svelte";
  import BodyTypeSelect from "./BodyTypeSelect.svelte";
  import TabBar from "../common/TabBar.svelte";
  import { validateGraphql } from "../../domain/graphql";
  import { streams } from "../../state/streams.svelte";
  import { store } from "../../state/store.svelte";
  import { REQUEST_TYPES, requestType, type RequestType, type Auth, type Body } from "../../domain/types";
  import CodeEditor from "../common/CodeEditor.svelte";
  import KeyValue from "../common/KeyValue.svelte";
  import RuntimeHeaders from "./RuntimeHeaders.svelte";
  import { disabledRuntime, runtimeHeaders } from "../../domain/runtimeHeaders";
  import { inheritedAuth, inheritedHeaders } from "../../domain/folderInheritance";
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
  import DocsEditor from "./DocsEditor.svelte";
  let exportingCurl = $state(false);

  let editorWidth = $state(0.48);
  let panelWidth = $state(1000);
  const stacked = $derived(store.forceStacked || panelWidth < 760);
  let docsPreview = $state(false);
  function observePanels(node: HTMLDivElement) {
    const observer = new ResizeObserver(([entry]) => { panelWidth = entry.contentRect.width; });
    observer.observe(node);
    return { destroy: () => observer.disconnect() };
  }
  let panels = $state<HTMLDivElement>();
  function resizePanels(delta: number) {
    if (panels) editorWidth = Math.min(0.8, Math.max(0.2, editorWidth + delta / (stacked ? panels.clientHeight : panels.clientWidth)));
  }

  const headerSuggestions = [
    "Accept", "Accept-Charset", "Accept-Encoding", "Accept-Language", "Authorization",
    "Cache-Control", "Connection", "Content-Encoding", "Content-Language", "Content-Length",
    "Content-Type", "Cookie", "Date", "ETag", "Expect", "Host", "If-Match",
    "If-Modified-Since", "If-None-Match", "If-Unmodified-Since", "Origin", "Pragma",
    "Range", "Referer", "User-Agent", "X-API-Key", "X-Request-ID", "X-Requested-With",
  ];

  type Tab = "variables" | "params" | "headers" | "body" | "auth" | "expect" | "scripts" | "tests" | "docs";
  let tab = $state<Tab>("params");

  const req = $derived(store.draft);
  const type = $derived(req ? requestType(req) : "http");
  const folders = $derived(store.selected ? folderChain(store.selected) : []);
  const folderHeaders = $derived(store.collection ? inheritedHeaders(store.collection, folders) : []);
  const parentAuth = $derived(store.collection ? inheritedAuth(store.collection, folders) : { auth: { type: "none" } as Auth, source: "la colección" });
  const currentSession = $derived(streams.sessions.find((session) => session.root === store.collection?.root && session.key === store.selected));
  const isStream = $derived(type === "websocket" || type === "sse");
  $effect(() => {
    void store.selected;
    tab = type === "graphql" ? "body" : "params";
  });
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
    const url = composedUrl;
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
  const expectCount = $derived(req ? (req.expect.status != null ? 1 : 0) + (req.expect.max_time_ms != null ? 1 : 0) + req.expect.body_contains.length + count(req.expect.headers) : 0);

  function set<K extends keyof NonNullable<typeof req>>(k: K, v: NonNullable<typeof req>[K]) {
    if (!store.draft) return;
    store.draft[k] = v;
    store.touch();
  }

  // Barra de URL ⇄ Params (domain/urlQuery.ts). Mientras se escribe se
  // respeta el texto tal cual (p. ej. un "?" recién tecleado); si Params
  // cambia desde la tabla, la barra se recompone.
  const composedUrl = $derived(req ? composeUrl(req.url, req.query, req.disabled_query) : "");
  let typedUrl = $state<{ text: string; composed: string } | null>(null);
  const urlBarValue = $derived(typedUrl && typedUrl.composed === composedUrl ? typedUrl.text : composedUrl);
  function onUrlInput(text: string) {
    if (!store.draft) return;
    const next = applyUrlInput(text, { url: store.draft.url, query: store.draft.query, disabled_query: store.draft.disabled_query });
    store.draft.url = next.url;
    store.draft.query = next.query;
    store.draft.disabled_query = next.disabled_query;
    store.touch();
    typedUrl = { text, composed: composeUrl(next.url, next.query, next.disabled_query) };
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
          type: requestType(imported), method: imported.method, url: imported.url, headers: imported.headers,
          query: imported.query, disabled_query: imported.disabled_query, disabled_headers: imported.disabled_headers ?? [], header_descriptions: {}, query_descriptions: {}, body: imported.body, auth: imported.auth,
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
    const apply = () => {
      set("body", next);
      set("type", kind === "graphql" ? "graphql" : "http");
      if (kind === "graphql" && req?.method === "GET") set("method", "POST");
    };
    const source = req?.body;
    const raw = ["json", "text", "xml"];
    const hasData = source && (source.type === "json" ? !!source.json : source.type === "text" ? !!source.text : source.type === "xml" ? !!source.xml : source.type === "binary" ? !!source.path : source.type === "form" ? !!Object.keys(source.fields).length : source.type === "multipart" ? !!source.parts.length : source.type === "graphql" ? !!source.query : false);
    if (hasData && source?.type !== kind && !(raw.includes(source!.type) && raw.includes(kind))) {
      store.dialog = { kind: "confirm", title: "Cambiar formato del cuerpo", text: "Se sustituirá el contenido del cuerpo actual. El cambio quedará pendiente de guardar.", ok: "Cambiar formato", onOk: apply };
    } else apply();
  }

  const graphqlIssues = $derived(req?.body.type === "graphql" ? validateGraphql(req.body.query, req.body.variables, req.body.operation_name, req.body.schema) : []);
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
    if (store.modal || store.dialog || e.defaultPrevented) return;
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
      <select class="request-type" aria-label="Tipo de solicitud" value={type} disabled={store.sending} onchange={(event) => { const selected = event.currentTarget.value as RequestType; event.currentTarget.value = type; store.changeRequestType(selected); }}>
        {#each REQUEST_TYPES as option}<option value={option.value}>{option.label}</option>{/each}
      </select>
      <input
        class="name-input"
        value={req.name}
        oninput={(e) => set("name", e.currentTarget.value)}
        placeholder="Nombre de la solicitud"
        aria-label="Nombre de la solicitud"
        spellcheck="false"
      />
      {#if store.dirty}<span class="unsaved">Sin guardar</span>{/if}
      <span class="key mono muted" title="Archivo">{#if type === "http"}<MethodBadge method={req.method} compact={false} />{:else}{REQUEST_TYPES.find((item) => item.value === type)?.label}{/if} {store.selected}.toml</span>
    </div>
    <div class="urlbar">

      <div class="address">
        {#if !isStream}
        <MethodSelect value={req.method} onchange={(method) => set("method", method)} />
        {/if}
        <div class="url">
          <VariableInput
            value={urlBarValue}
            oninput={(e) => onUrlInput(e.currentTarget.value)}
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
        {#if !isStream}
        <button class="btn icon save" title="Exportar como cURL" aria-label="Exportar como cURL" onclick={() => (exportingCurl = true)}>
          <svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="m8 8-4 4 4 4m8-8 4 4-4 4m-2-12-4 16" /></svg>
        </button>
        {/if}
      </div>
      {#if store.sending}
        <button class="btn send" onclick={() => store.cancelSend()} disabled={store.cancelling}>{store.cancelling ? "Cancelando…" : "Cancelar envío"}</button>
      {:else}
        <button class="btn primary send" onclick={() => store.send()}>{#if isStream}{currentSession && currentSession.state !== "closed" ? "Ver conexión" : "Conectar"}{:else}Enviar{/if}</button>
      {/if}
    </div>

    {#if composedUrl.includes("{{")}
      <div class="resolved-url" title={urlError || resolvedUrl}>
        <span class="muted">URL resuelta</span>
        <span class:unresolved={!!urlError} class="mono">{urlError || resolvedUrl || "Resolviendo…"}</span>
      </div>
    {/if}
    <div class="panels" class:stacked use:observePanels bind:this={panels}>
      <div class="request-editor" style:width={stacked ? "100%" : `${editorWidth * 100}%`} style:height={stacked ? `${editorWidth * 100}%` : "100%"}>
    <div class="request-tabbar"><div class="section-tabs">
    <TabBar label="Secciones de la solicitud" active={tab} onselect={(id) => (tab = id as Tab)} items={[
      { id: "params", label: "Params", count: count(req.query) || undefined },
      ...(!isStream ? [{ id: "body", label: type === "graphql" ? "GraphQL" : "Body" }] : []),
      { id: "headers", label: "Headers", count: count(req.headers) || undefined },
      { id: "auth", label: "Auth" },
      { id: "variables", label: "Vars" },
      ...(!isStream ? [{ id: "scripts", label: "Script" }, { id: "expect", label: "Assert", count: expectCount || undefined }, { id: "tests", label: "Tests" }] : []),
      { id: "docs", label: "Docs", dot: !!req.docs.trim() },
    ]} />
    </div>{#if tab === "body" && type !== "graphql"}<BodyTypeSelect value={bodyKind} onchange={setBodyKind} />{/if}{#if tab === "docs"}<button class="preview-toggle" class:on={docsPreview} aria-pressed={docsPreview} onclick={() => (docsPreview = !docsPreview)} title="Alternar vista previa"><svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M2 12s3.6-7 10-7 10 7 10 7-3.6 7-10 7S2 12 2 12z"/><circle cx="12" cy="12" r="3"/></svg><span>Vista previa</span></button>{/if}</div>

    <div class="content">
      {#if tab === "params"}
        <p class="table-label">Parámetros de consulta</p>
        <KeyValue descriptions={req.query_descriptions ?? {}} onDescriptionsChange={(values) => set("query_descriptions", values)} {variables} disabledKeys={req.disabled_query} onDisabledChange={(keys) => set("disabled_query", keys)} entries={req.query} onchange={(v) => set("query", v)} keyPlaceholder="parámetro" addLabel="Añadir parámetro" secretLocation={(name) => `parámetro ${name}`} />
      {:else if tab === "headers"}
        {#if type === "http" || type === "graphql"}
          <RuntimeHeaders headers={runtimeHeaders(req, parentAuth, resolvedUrl, folderHeaders)} disabled={disabledRuntime(req, folderHeaders)}
            ontoggle={(name, enabled) => set("disabled_headers", enabled ? (req.disabled_headers ?? []).filter((k) => k.toLowerCase() !== name.toLowerCase()) : [...(req.disabled_headers ?? []), name])} />
        {/if}
        <p class="table-label">Cabeceras de la solicitud <span class="muted">({count(req.headers)})</span></p>
        <KeyValue disabledKeys={req.disabled_headers ?? []} onDisabledChange={(keys) => set("disabled_headers", [...keys, ...disabledRuntime(req, folderHeaders).filter((name) => !keys.some((k) => k.toLowerCase() === name.toLowerCase()))])} caseInsensitive descriptions={req.header_descriptions ?? {}} onDescriptionsChange={(values) => set("header_descriptions", values)} {variables} entries={req.headers} onchange={(v) => set("headers", v)} keyPlaceholder="Cabecera" valuePlaceholder="valor" addLabel="Añadir cabecera" suggestions={headerSuggestions} secretLocation={(name) => `header ${name}`} />
      {:else if tab === "body"}
        <div class="body">
          {#if bodyKind === "json"}<div class="body-bar"><button class="btn small" onclick={formatJson}>Formatear JSON</button>{#if jsonError}<span class="json-error" role="alert">{jsonError}</span>{/if}</div>{/if}
          {#if bodyKind === "json" || bodyKind === "text" || bodyKind === "xml"}
            <div class="editor-wrap">
              <CodeEditor value={bodyText} language={bodyKind === "json" ? "json" : "text"} onchange={setBodyText} />
            </div>
          {:else if req.body.type === "graphql"}
            <div class="graphql-editor">
              {#if graphqlIssues.length}<div class="graphql-errors" role="status">{#each graphqlIssues as issue}<p>{issue.line ? `Línea ${issue.line}, columna ${issue.column}: ` : ""}{issue.message}</p>{/each}</div>{/if}
              <label>Nombre de operación <span class="muted">opcional</span><input value={req.body.operation_name ?? ""} placeholder="Nombre de operación" oninput={(e) => { if (req.body.type === "graphql") set("body", { ...req.body, operation_name: e.currentTarget.value }); }} /></label>
              <div class="graphql-label">Consulta</div>
              <div class="graphql-query"><CodeEditor value={req.body.query} language="graphql" schema={req.body.schema ?? ""} label="Consulta GraphQL" onchange={(value) => { if (req.body.type === "graphql") set("body", { ...req.body, query: value }); }} /></div>
              <div class="graphql-label">Variables <span class="muted">JSON</span></div>
              <div class="graphql-vars"><CodeEditor value={req.body.variables ?? ""} language="json" label="Variables GraphQL" onchange={(value) => { if (req.body.type === "graphql") set("body", { ...req.body, variables: value }); }} /></div>
              <details class="schema-details"><summary>Esquema SDL local · validación y autocompletado</summary>
                <p class="muted">Pega el esquema de la API. Se guarda en la colección y no se envía al servidor. Usa Ctrl+Espacio para ver sugerencias.</p>
                <textarea aria-label="Esquema GraphQL SDL" value={req.body.schema ?? ""} spellcheck="false" placeholder={"type Query { user(id: ID!): User }"} oninput={(event) => { if (req.body.type === "graphql") set("body", { ...req.body, schema: event.currentTarget.value }); }}></textarea>
              </details>
            </div>
          {:else if bodyKind === "binary"}
            <div class="binary-editor">
              <p class="muted">Selecciona un archivo o escribe una ruta relativa a la colección.</p>
              <div class="binary-row"><VariableInput value={req.body.type === "binary" ? req.body.path : ""} {variables} label="Ruta del archivo binario" placeholder="archivos/datos.bin" oninput={(e) => set("body", { type: "binary", path: e.currentTarget.value })} /><button class="btn small" disabled={isMock} onclick={async () => { const path = await (await api).pickFile(); if (path && store.draft?.body.type === "binary") set("body", { type: "binary", path }); }}>Elegir…</button></div>
            </div>
          {:else if bodyKind === "form"}
            <KeyValue {variables} entries={req.body.type === "form" ? req.body.fields : {}} onchange={(fields) => set("body", { type: "form", fields })} keyPlaceholder="campo" addLabel="Añadir campo" secretLocation={(name) => `campo del formulario ${name}`} />
          {:else if bodyKind === "multipart"}
            <MultipartEditor parts={req.body.type === "multipart" ? req.body.parts : []} {variables} onchange={(parts) => set("body", { type: "multipart", parts })} />
          {:else}
            <div class="nobody">
              <p class="muted">Esta solicitud no envía cuerpo.</p>
              <button class="btn" onclick={() => { setBodyKind("json"); }}>Añadir cuerpo JSON</button>
            </div>
          {/if}
        </div>
      {:else if tab === "variables"}
        <VariableInspector embedded />
      {:else if tab === "auth"}
        <AuthForm
          auth={req.auth}
          onchange={(a: Auth) => set("auth", a)}
          secretActions
          inheritHint={`Usa la auth de ${parentAuth.source} (${parentAuth.auth.type}).`}
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
        <DocsEditor value={req.docs} preview={docsPreview} onchange={(v) => set("docs", v)} />
      {/if}
    </div>
      </div>
      <Splitter direction={stacked ? "vertical" : "horizontal"} onresize={resizePanels} />
      <div class="response-panel">
        {#if isStream}
          <div class="stream-summary">
            <h3>{type === "websocket" ? "WebSocket" : "Eventos SSE"}</h3>
            <p class="muted">{currentSession?.state === "open" ? "Conexión activa" : currentSession?.state === "connecting" ? "Conectando…" : currentSession?.state === "closing" ? "Desconectando…" : "Sin conexión activa"}</p>
            <p class="muted">{type === "websocket" ? "Envía y recibe mensajes desde el panel de conexiones." : "Consulta los eventos del servidor desde el panel de conexiones."} Puedes seguir trabajando con otras solicitudes mientras está conectado.</p>
            <button class="btn" onclick={() => store.openStream()}>Abrir conexión</button>
          </div>
        {:else}<ResponsePane />{/if}
      </div>
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
  .stream-summary { padding: 24px; max-width: 480px; }
  .stream-summary h3 { font-size: 14px; margin-top: 0; }
  .pane {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-width: 0;
  }
  .panels { flex: 1; display: flex; min-height: 0; min-width: 0; padding: 0 0 0; border-top: 1px solid var(--border); }
  .panels.stacked { flex-direction: column; }
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
    flex-wrap: wrap;
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
    min-width: 0;
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
    padding: 10px 18px 12px;
    align-items: center;
    flex-wrap: nowrap;
  }
  .address { display: flex; flex: 1 1 0; min-width: 0; align-items: center; border: 1px solid var(--border-strong); border-radius: 8px; background: var(--bg-panel); min-height: 36px; transition: border-color var(--t), box-shadow var(--t), background var(--t); }
  .address:hover { border-color: color-mix(in srgb, var(--accent) 45%, var(--border-strong)); }
  .url :global(.integrated input) { height: 34px; padding: 6px 10px; }
  .address:focus-within { border-color: var(--accent); box-shadow: 0 0 0 3px var(--accent-soft); background: var(--bg); }
  .address .save { margin: 0 6px; flex: 0 0 auto; }
  .request-type { flex: 0 0 auto; min-height: 38px; max-width: 130px; font-size: 11px; min-height: 26px; padding: 3px 7px; background: transparent; border-color: transparent; }
  .send { border-radius: 7px; padding: 6px 15px; font-size: 12px; justify-content: center; min-width: 76px; min-height: 36px; flex: 0 0 auto; }
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
  .request-tabbar { display: flex; align-items: stretch; min-width: 0; border-bottom: 1px solid var(--border); flex: 0 0 auto; }
  .section-tabs { flex: 1; min-width: 0; }
  .section-tabs :global(.tabbar) { border-bottom: 0; }
  .preview-toggle { display: inline-flex; align-items: center; gap: 6px; padding: 0 14px; font-size: 12px; color: var(--text-muted); flex: 0 0 auto; }
  .preview-toggle:hover { color: var(--text); }
  .preview-toggle.on { color: var(--accent); }
  .table-label { color: var(--text-muted); font-size: 12px; margin: 0 0 14px; }
  .graphql-errors { color: var(--err); font-size: 12px; overflow-wrap: anywhere; }
  .graphql-errors p { margin: 2px 0; }
  .schema-details { font-size: 12px; }
  .schema-details textarea { width: 100%; min-height: 120px; resize: vertical; font-family: var(--mono); background: var(--bg); border: 1px solid var(--border); border-radius: var(--radius); padding: 8px; }
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
</style>
