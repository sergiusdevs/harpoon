<script lang="ts">
  // Documentación de la solicitud en Markdown: editor enriquecido (contenteditable),
  // editor de Markdown en texto plano y vista previa de solo lectura.
  import { onMount, tick } from "svelte";
  import { renderMarkdown, htmlToMarkdown, safeUrl } from "../../domain/markdown";
  import { store } from "../../state/store.svelte";
  import { isMock } from "../../services/api";

  interface Props { value: string; onchange: (value: string) => void; preview?: boolean }
  let { value, onchange, preview = false }: Props = $props();

  type Mode = "rich" | "markdown";
  let mode = $state<Mode>(readMode());
  function readMode(): Mode {
    try { return localStorage.getItem("harpoon.docsMode") === "markdown" ? "markdown" : "rich"; } catch { return "rich"; }
  }
  function setMode(next: Mode) {
    mode = next;
    try { localStorage.setItem("harpoon.docsMode", next); } catch { /* sin almacenamiento */ }
  }

  let rich = $state<HTMLDivElement | null>(null);
  let area = $state<HTMLTextAreaElement | null>(null);
  let block = $state("p");
  /** Último Markdown emitido por el editor enriquecido, para no reescribir su DOM mientras se escribe. */
  let emitted: string | null = null;
  let filled: HTMLElement | null = null;

  const editableHtml = (md: string) => renderMarkdown(md).replace(/<input type="checkbox" disabled/g, '<input type="checkbox"');

  $effect(() => {
    const md = value;
    if (mode !== "rich" || preview || !rich) return;
    if (md === emitted && filled === rich) return;
    rich.innerHTML = editableHtml(md);
    emitted = md;
    filled = rich;
  });

  function syncRich() {
    if (!rich) return;
    const md = htmlToMarkdown(rich);
    emitted = md;
    if (md !== value) onchange(md);
  }

  function exec(command: string, arg?: string) {
    rich?.focus();
    document.execCommand(command, false, arg);
    syncRich();
    updateBlock();
  }

  const escapeHtml = (text: string) => text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
  function selectedText() { return window.getSelection()?.toString() ?? ""; }

  function updateBlock() {
    if (mode === "markdown") {
      if (!area) return;
      const start = area.value.lastIndexOf("\n", area.selectionStart - 1) + 1;
      const m = area.value.slice(start).match(/^(#{1,3})\s/);
      block = m ? "h" + m[1].length : "p";
      return;
    }
    let node: Node | null = window.getSelection()?.anchorNode ?? null;
    while (node && node !== rich) {
      if (node.nodeType === 1 && /^H[1-3]$/.test(node.nodeName)) { block = node.nodeName.toLowerCase(); return; }
      node = node.parentNode;
    }
    block = "p";
  }

  onMount(() => {
    const listener = () => {
      const anchor = window.getSelection()?.anchorNode;
      if (anchor && rich?.contains(anchor)) updateBlock();
    };
    document.addEventListener("selectionchange", listener);
    return () => document.removeEventListener("selectionchange", listener);
  });

  // ---- Markdown en texto plano
  function replaceRange(start: number, end: number, text: string, selStart?: number, selEnd?: number) {
    if (!area) return;
    area.focus();
    area.setSelectionRange(start, end);
    if (!document.execCommand("insertText", false, text)) area.setRangeText(text, start, end, "end");
    if (selStart !== undefined) area.setSelectionRange(selStart, selEnd ?? selStart);
    onchange(area.value);
  }
  function wrapMd(mark: string, placeholder: string) {
    if (!area) return;
    const { selectionStart: s, selectionEnd: e, value: v } = area;
    const inner = v.slice(s, e) || placeholder;
    replaceRange(s, e, mark + inner + mark, s + mark.length, s + mark.length + inner.length);
  }
  function prefixLines(make: (index: number) => string, strip = /^(\s*)([-*+]\s\[[ xX]\]\s|[-*+]\s|\d+[.)]\s|#{1,6}\s)?/) {
    if (!area) return;
    const v = area.value;
    const start = v.lastIndexOf("\n", area.selectionStart - 1) + 1;
    let end = v.indexOf("\n", Math.max(area.selectionEnd - (area.selectionEnd > area.selectionStart ? 1 : 0), area.selectionStart));
    if (end < 0) end = v.length;
    const lines = v.slice(start, end).split("\n").map((line, i) => line.replace(strip, (_m, indent: string) => indent + make(i)));
    const text = lines.join("\n");
    replaceRange(start, end, text, start + text.length);
  }
  function insertMd(before: string, inner: string, after: string, ownLine = false) {
    if (!area) return;
    const { selectionStart: s, selectionEnd: e, value: v } = area;
    const sel = v.slice(s, e) || inner;
    const lead = ownLine && s > 0 && v[s - 1] !== "\n" ? "\n" : "";
    replaceRange(s, e, lead + before + sel + after, s + lead.length + before.length, s + lead.length + before.length + sel.length);
  }

  /** Inserta un bloque al nivel superior (fuera de listas, tablas o párrafos), tras el bloque actual. */
  function insertBlock(html: string) {
    if (!rich) return;
    rich.focus();
    const sel = window.getSelection();
    let top: Node | null = sel?.anchorNode ?? null;
    while (top && top.parentNode !== rich) top = top.parentNode;
    if (sel && top && rich.contains(top)) {
      const range = document.createRange();
      if (!(top.textContent ?? "").trim() && top.nodeName === "P") range.selectNode(top);
      else { range.setStartAfter(top); range.collapse(true); }
      sel.removeAllRanges();
      sel.addRange(range);
    }
    exec("insertHTML", html);
  }

  // ---- Acciones de la barra
  let savedRange: Range | null = null;
  function action(kind: string) {
    if (mode === "markdown") {
      switch (kind) {
        case "bold": return wrapMd("**", "negrita");
        case "italic": return wrapMd("*", "cursiva");
        case "strike": return wrapMd("~~", "tachado");
        case "code": return wrapMd("`", "código");
        case "ul": return prefixLines(() => "- ");
        case "ol": return prefixLines((i) => `${i + 1}. `);
        case "task": return prefixLines(() => "- [ ] ");
        case "codeblock": return insertMd("```\n", "código", "\n```\n", true);
        case "table": return insertMd("| Columna 1 | Columna 2 |\n| --- | --- |\n| ", "valor", " |  |\n", true);
        case "link": {
          if (!area) return;
          const { selectionStart: s, selectionEnd: e, value: v } = area;
          const label = v.slice(s, e) || "texto";
          const text = `[${label}](https://)`;
          return replaceRange(s, e, text, s + label.length + 3, s + text.length - 1);
        }
      }
      return;
    }
    switch (kind) {
      case "bold": return exec("bold");
      case "italic": return exec("italic");
      case "strike": return exec("strikeThrough");
      case "ul": return exec("insertUnorderedList");
      case "ol": return exec("insertOrderedList");
      case "code": return exec("insertHTML", `<code>${escapeHtml(selectedText()) || "código"}</code>`);
      case "codeblock": { const text = escapeHtml(selectedText()) || "código"; return insertBlock(`<pre><code>${text}</code></pre><p><br></p>`); }
      case "task": { const text = escapeHtml(selectedText()) || "Tarea"; return insertBlock(`<ul class="task-list"><li class="task"><input type="checkbox"> ${text}</li></ul><p><br></p>`); }
      case "table": return insertBlock(`<table><thead><tr><th>Columna 1</th><th>Columna 2</th></tr></thead><tbody><tr><td>valor</td><td><br></td></tr></tbody></table><p><br></p>`);
      case "link": {
        const selection = window.getSelection();
        savedRange = selection && selection.rangeCount && rich?.contains(selection.anchorNode) ? selection.getRangeAt(0).cloneRange() : null;
        store.dialog = {
          kind: "prompt", title: "Insertar enlace", label: "URL (http, https o mailto)", value: "https://", ok: "Insertar",
          onOk: (url) => {
            const href = safeUrl(url);
            if (!href) throw new Error("Solo se admiten enlaces http(s) o mailto.");
            // Se inserta cuando el diálogo ya se ha cerrado y el editor vuelve a ser editable.
            setTimeout(async () => {
              await tick();
              rich?.focus();
              const sel = window.getSelection();
              if (savedRange && sel) { sel.removeAllRanges(); sel.addRange(savedRange); }
              if (!sel || sel.isCollapsed) exec("insertHTML", `<a href="${escapeHtml(href).replace(/"/g, "&quot;")}">${escapeHtml(href)}</a>`);
              else exec("createLink", href);
            }, 60);
          },
        };
        return;
      }
    }
  }

  function setBlock(tag: string) {
    block = tag;
    if (mode === "markdown") {
      const level = tag === "p" ? 0 : Number(tag[1]);
      return prefixLines(() => (level ? "#".repeat(level) + " " : ""), /^(\s*)(#{1,6}\s)?/);
    }
    rich?.focus();
    const sel = window.getSelection();
    if (savedRange && sel) { sel.removeAllRanges(); sel.addRange(savedRange); }
    exec("formatBlock", `<${tag}>`);
  }

  function richKey(e: KeyboardEvent) {
    const meta = e.metaKey || e.ctrlKey;
    if (meta && e.key.toLowerCase() === "b") { e.preventDefault(); action("bold"); return; }
    if (meta && e.key.toLowerCase() === "i") { e.preventDefault(); action("italic"); return; }
    if (e.key === "Enter" && !e.shiftKey) {
      let node: Node | null = window.getSelection()?.anchorNode ?? null;
      while (node && node !== rich && node.nodeName !== "PRE") node = node.parentNode;
      if (node && node.nodeName === "PRE") { e.preventDefault(); document.execCommand("insertText", false, "\n"); syncRich(); }
    }
  }
  function mdKey(e: KeyboardEvent) {
    const meta = e.metaKey || e.ctrlKey;
    if (meta && e.key.toLowerCase() === "b") { e.preventDefault(); action("bold"); }
    else if (meta && e.key.toLowerCase() === "i") { e.preventDefault(); action("italic"); }
  }
  function richClick(e: MouseEvent) {
    const target = e.target as HTMLElement;
    if (target instanceof HTMLInputElement && target.type === "checkbox") {
      if (target.checked) target.setAttribute("checked", ""); else target.removeAttribute("checked");
      syncRich();
    }
  }
  function richPaste(e: ClipboardEvent) {
    // Pega como texto para no arrastrar estilos ni HTML ajeno.
    const text = e.clipboardData?.getData("text/plain");
    if (text === undefined) return;
    e.preventDefault();
    document.execCommand("insertText", false, text);
    syncRich();
  }

  async function openLink(e: MouseEvent) {
    const link = (e.target as HTMLElement).closest("a");
    if (!link) return;
    e.preventDefault();
    const href = safeUrl(link.getAttribute("href") ?? "");
    if (!href) return;
    if (isMock) { window.open(href, "_blank", "noopener"); return; }
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke("plugin:opener|open_url", { url: href });
    } catch (error) {
      store.status = { kind: "err", text: `No se pudo abrir el enlace: ${String(error)}` };
    }
  }

  const tools: { id: string; label: string; icon: string }[] = [
    { id: "bold", label: "Negrita (⌘B)", icon: '<path d="M7 5h6a3.5 3.5 0 0 1 0 7H7zM7 12h7a3.5 3.5 0 0 1 0 7H7z"/>' },
    { id: "italic", label: "Cursiva (⌘I)", icon: '<path d="M11 5h6M7 19h6M14 5l-4 14"/>' },
    { id: "strike", label: "Tachado", icon: '<path d="M5 12h14M16 7.5C15.4 6 13.9 5 12 5c-2.5 0-4 1.3-4 3 0 1.2.7 2.2 2.3 2.8M8 16.5c.6 1.5 2.1 2.5 4 2.5 2.5 0 4-1.3 4-3 0-.7-.2-1.3-.7-1.8"/>' },
    { id: "link", label: "Enlace", icon: '<path d="M10 14a4 4 0 0 0 5.7 0l3-3a4 4 0 0 0-5.7-5.7l-1 1M14 10a4 4 0 0 0-5.7 0l-3 3a4 4 0 0 0 5.7 5.7l1-1"/>' },
    { id: "ul", label: "Lista", icon: '<path d="M9 6h11M9 12h11M9 18h11"/><circle cx="4.5" cy="6" r="1" fill="currentColor"/><circle cx="4.5" cy="12" r="1" fill="currentColor"/><circle cx="4.5" cy="18" r="1" fill="currentColor"/>' },
    { id: "ol", label: "Lista numerada", icon: '<path d="M10 6h10M10 12h10M10 18h10M4 4.5l1.5-.5v4M3.5 14.5c0-1 1-1.5 1.7-1.5s1.3.5 1.3 1.2c0 1.3-3 2.3-3 3.8h3"/>' },
    { id: "task", label: "Lista de tareas", icon: '<rect x="3" y="4" width="6" height="6" rx="1"/><path d="m4.5 7 1 1 2-2M12 7h9M12 17h9"/><rect x="3" y="14" width="6" height="6" rx="1"/>' },
    { id: "table", label: "Tabla", icon: '<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M3 10h18M3 15h18M10 4v16"/>' },
    { id: "code", label: "Código", icon: '<path d="m8 8-4 4 4 4M16 8l4 4-4 4"/>' },
    { id: "codeblock", label: "Bloque de código", icon: '<rect x="3" y="4" width="18" height="16" rx="2"/><path d="m9 10-2 2 2 2M15 10l2 2-2 2"/>' },
  ];
  const html = $derived(renderMarkdown(value));
</script>

<div class="docs">
  {#if preview}
    <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
    <div class="markdown-body preview" onclick={openLink}>
      {#if value.trim()}{@html html}{:else}<p class="empty">Esta solicitud aún no tiene documentación.</p>{/if}
    </div>
  {:else}
    <div class="toolbar" role="toolbar" aria-label="Formato de la documentación">
      <select class="block" aria-label="Tipo de bloque" value={block} onchange={(e) => setBlock(e.currentTarget.value)} onmousedown={() => { const s = window.getSelection(); if (mode === "rich" && s?.rangeCount && rich?.contains(s.anchorNode)) savedRange = s.getRangeAt(0).cloneRange(); }}>
        <option value="p">Normal</option>
        <option value="h1">Título 1</option>
        <option value="h2">Título 2</option>
        <option value="h3">Título 3</option>
      </select>
      <span class="sep" aria-hidden="true"></span>
      {#each tools as tool, index (tool.id)}
        {#if index === 3 || index === 7}<span class="sep" aria-hidden="true"></span>{/if}
        <button type="button" class="tool" title={tool.label} aria-label={tool.label} onmousedown={(e) => e.preventDefault()} onclick={() => action(tool.id)}>
          <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">{@html tool.icon}</svg>
        </button>
      {/each}
      <div class="modes" role="radiogroup" aria-label="Modo de edición">
        <button type="button" role="radio" aria-checked={mode === "rich"} class:on={mode === "rich"} onclick={() => setMode("rich")}>Texto enriquecido</button>
        <button type="button" role="radio" aria-checked={mode === "markdown"} class:on={mode === "markdown"} onclick={() => setMode("markdown")}>Markdown</button>
      </div>
    </div>
    {#if mode === "rich"}
      <div
        class="markdown-body rich"
        bind:this={rich}
        contenteditable="true"
        role="textbox"
        aria-multiline="true"
        aria-label="Documentación"
        tabindex="0"
        data-placeholder="Describe esta solicitud: propósito, parámetros, ejemplos…"
        class:is-empty={!value.trim()}
        oninput={syncRich}
        onkeydown={richKey}
        onclick={richClick}
        onpaste={richPaste}
        onfocus={() => { try { document.execCommand("defaultParagraphSeparator", false, "p"); document.execCommand("styleWithCSS", false, "false"); } catch { /* sin soporte */ } }}
      ></div>
    {:else}
      <textarea
        class="md"
        bind:this={area}
        {value}
        spellcheck="false"
        aria-label="Documentación en Markdown"
        placeholder="# Título&#10;&#10;Describe esta solicitud en **Markdown**…"
        oninput={(e) => onchange(e.currentTarget.value)}
        onkeydown={mdKey}
        onkeyup={updateBlock}
        onclick={updateBlock}
      ></textarea>
    {/if}
  {/if}
</div>

<style>
  .docs { display: flex; flex-direction: column; height: 100%; min-height: 0; border: 1px solid var(--border-strong); border-radius: 7px; overflow: hidden; background: var(--bg); }
  .toolbar { display: flex; align-items: center; gap: 2px; padding: 5px 6px; border-bottom: 1px solid var(--border); background: var(--bg-panel); flex-wrap: wrap; flex: 0 0 auto; }
  .block { font-size: 12px; padding: 3px 6px; background: transparent; border-color: transparent; min-width: 92px; }
  .block:hover { border-color: var(--border); }
  .sep { width: 1px; height: 16px; background: var(--border); margin: 0 4px; }
  .tool { display: inline-grid; place-items: center; width: 26px; height: 26px; border-radius: 5px; color: var(--text-muted); }
  .tool:hover { color: var(--text); background: var(--bg-hover); }
  .modes { margin-left: auto; display: inline-flex; border: 1px solid var(--border); border-radius: 6px; padding: 2px; gap: 2px; }
  .modes button { font-size: 11.5px; padding: 3px 9px; border-radius: 4px; color: var(--text-muted); }
  .modes button:hover { color: var(--text); }
  .modes button.on { background: var(--bg-active); color: var(--text); }
  .rich, .md, .preview { flex: 1; min-height: 0; overflow: auto; padding: 14px 18px; }
  .rich { outline: none; cursor: text; user-select: text; -webkit-user-select: text; }
  .rich.is-empty:empty::before { content: attr(data-placeholder); color: var(--text-faint); pointer-events: none; }
  .md { width: 100%; resize: none; border: 0; border-radius: 0; background: transparent; font: var(--editor-font-size)/1.6 var(--mono); color: var(--text); outline: none; }
  .preview { user-select: text; -webkit-user-select: text; }
  .empty { color: var(--text-faint); }

  .markdown-body { font-size: 13px; line-height: 1.6; color: var(--text); overflow-wrap: anywhere; }
  .markdown-body :global(> :first-child) { margin-top: 0; }
  .markdown-body :global(h1), .markdown-body :global(h2), .markdown-body :global(h3), .markdown-body :global(h4) { margin: 1.1em 0 0.5em; line-height: 1.3; font-weight: 600; }
  .markdown-body :global(h1) { font-size: 1.6em; padding-bottom: 0.25em; border-bottom: 1px solid var(--border); }
  .markdown-body :global(h2) { font-size: 1.3em; padding-bottom: 0.2em; border-bottom: 1px solid var(--border); }
  .markdown-body :global(h3) { font-size: 1.1em; }
  .markdown-body :global(p) { margin: 0 0 0.8em; }
  .markdown-body :global(a) { color: var(--accent); text-decoration: underline; text-underline-offset: 2px; }
  .markdown-body :global(code) { font: 0.9em var(--mono); background: var(--bg-raised); border: 1px solid var(--border); border-radius: 4px; padding: 0.05em 0.35em; }
  .markdown-body :global(pre) { font: 0.9em/1.5 var(--mono); background: var(--bg-raised); border: 1px solid var(--border); border-radius: 6px; padding: 10px 12px; overflow: auto; margin: 0 0 0.9em; white-space: pre; }
  .markdown-body :global(pre code) { background: none; border: 0; padding: 0; font-size: 1em; }
  .markdown-body :global(ul), .markdown-body :global(ol) { margin: 0 0 0.8em; padding-left: 1.6em; }
  .markdown-body :global(li) { margin: 0.15em 0; }
  .markdown-body :global(ul.task-list) { list-style: none; padding-left: 0.4em; }
  .markdown-body :global(li.task input) { margin: 0 6px 0 0; vertical-align: -2px; accent-color: var(--accent); }
  .markdown-body :global(blockquote) { margin: 0 0 0.8em; padding: 0 0 0 12px; border-left: 3px solid var(--border-strong); color: var(--text-muted); }
  .markdown-body :global(hr) { border: 0; border-top: 1px solid var(--border); margin: 1em 0; }
  .markdown-body :global(table) { border-collapse: collapse; margin: 0 0 0.9em; }
  .markdown-body :global(th), .markdown-body :global(td) { border: 1px solid var(--border-strong); padding: 5px 10px; text-align: left; min-width: 60px; }
  .markdown-body :global(th) { background: var(--bg-panel); font-weight: 600; }
</style>
