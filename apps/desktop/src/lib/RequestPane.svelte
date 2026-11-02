<script lang="ts">
  import { store } from "./store.svelte";
  import { METHODS, type Auth, type Body } from "./types";
  import CodeEditor from "./CodeEditor.svelte";
  import KeyValue from "./KeyValue.svelte";
  import AuthForm from "./AuthForm.svelte";

  type Tab = "params" | "headers" | "body" | "auth" | "expect" | "docs";
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
  <section class="pane">
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

    <div class="tabs">
      <button class="tab" class:active={tab === "params"} onclick={() => (tab = "params")}>Params {#if count(req.query)}<span class="count">{count(req.query)}</span>{/if}</button>
      <button class="tab" class:active={tab === "headers"} onclick={() => (tab = "headers")}>Headers {#if count(req.headers)}<span class="count">{count(req.headers)}</span>{/if}</button>
      <button class="tab" class:active={tab === "body"} onclick={() => (tab = "body")}>Body {#if bodyKind !== "none"}<span class="count">{bodyKind}</span>{/if}</button>
      <button class="tab" class:active={tab === "auth"} onclick={() => (tab = "auth")}>Auth {#if req.auth.type !== "inherit"}<span class="count">{req.auth.type}</span>{/if}</button>
      <button class="tab" class:active={tab === "expect"} onclick={() => (tab = "expect")}>Expect</button>
      <button class="tab" class:active={tab === "docs"} onclick={() => (tab = "docs")}>Docs</button>
    </div>

    <div class="content">
      {#if tab === "params"}
        <KeyValue entries={req.query} onchange={(v) => set("query", v)} keyPlaceholder="parámetro" />
      {:else if tab === "headers"}
        <KeyValue entries={req.headers} onchange={(v) => set("headers", v)} keyPlaceholder="Cabecera" />
      {:else if tab === "body"}
        <div class="body">
          <div class="body-kind">
            {#each ["none", "json", "text", "form"] as k}
              <button class="chip" class:active={bodyKind === k} onclick={() => setBodyKind(k as Body["type"])}>{k}</button>
            {/each}
          </div>
          {#if bodyKind === "json" || bodyKind === "text"}
            <div class="editor-wrap">
              <CodeEditor value={bodyText} language={bodyKind === "json" ? "json" : "text"} onchange={setBodyText} />
            </div>
          {:else if bodyKind === "form"}
            <KeyValue entries={req.body.type === "form" ? req.body.fields : {}} onchange={(fields) => set("body", { type: "form", fields })} keyPlaceholder="campo" />
          {:else}
            <p class="muted hint">Esta solicitud no envía cuerpo.</p>
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
  .urlbar {
    display: flex;
    gap: 8px;
    padding: 10px 12px;
    align-items: center;
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
  .body-kind {
    display: flex;
    gap: 4px;
  }
  .chip {
    padding: 2px 10px;
    border-radius: 999px;
    font-size: 11.5px;
    color: var(--text-muted);
    border: 1px solid var(--border);
    transition: all var(--t);
  }
  .chip:hover {
    color: var(--text);
    border-color: var(--border-strong);
  }
  .chip.active {
    color: var(--accent-strong);
    border-color: var(--accent);
    background: var(--accent-soft);
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
