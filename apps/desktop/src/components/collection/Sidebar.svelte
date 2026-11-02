<script lang="ts">
  import { store, basename } from "../../state/store.svelte";
  import ContextMenu, { type Item } from "../common/ContextMenu.svelte";
  import Icon from "../common/Icon.svelte";
  import MethodBadge from "../common/MethodBadge.svelte";

  let filter = $state("");
  let filterInput = $state<HTMLInputElement | null>(null);
  let newName = $state("");
  $effect(() => {
    if (store.pendingInit) newName = basename(store.pendingInit);
  });

  type Node = { name: string; path: string; key?: string; children: Node[] };

  // Árbol: carpetas (también vacías) + solicitudes, filtrado por texto.
  const tree = $derived.by(() => {
    const root: Node = { name: "", path: "", children: [] };
    if (!store.collection) return root;
    const q = filter.trim().toLowerCase();
    const ensure = (folder: string): Node => {
      let node = root;
      let acc = "";
      for (const p of folder.split("/").filter(Boolean)) {
        acc = acc ? acc + "/" + p : p;
        let child = node.children.find((c) => c.path === acc && !c.key);
        if (!child) {
          child = { name: p, path: acc, children: [] };
          node.children.push(child);
        }
        node = child;
      }
      return node;
    };
    if (!q) for (const f of store.folders) ensure(f);
    for (const [key, req] of Object.entries(store.collection.requests)) {
      if (q && !key.toLowerCase().includes(q) && !req.name.toLowerCase().includes(q)) continue;
      const i = key.lastIndexOf("/");
      const parent = i >= 0 ? ensure(key.slice(0, i)) : root;
      parent.children.push({ name: i >= 0 ? key.slice(i + 1) : key, path: key, key, children: [] });
    }
    const sort = (n: Node) => {
      n.children.sort((a, b) => Number(!!a.key) - Number(!!b.key) || a.name.localeCompare(b.name));
      n.children.forEach(sort);
    };
    sort(root);
    return root;
  });

  let collapsed = $state<Record<string, boolean>>({});

  /** Lista plana de los nodos visibles, para la navegación con teclado. */
  const visible = $derived.by(() => {
    const out: Node[] = [];
    const walk = (list: Node[]) => {
      for (const n of list) {
        out.push(n);
        if (!n.key && !collapsed[n.path]) walk(n.children);
      }
    };
    walk(tree.children);
    return out;
  });

  let focused = $state<string | null>(null); // path del nodo con foco de teclado
  let treeEl = $state<HTMLElement | null>(null);

  function onTreeKey(e: KeyboardEvent) {
    if (!visible.length) return;
    const idx = Math.max(0, visible.findIndex((n) => n.path === focused));
    const cur = visible[idx];
    const go = (i: number) => {
      const n = visible[Math.min(visible.length - 1, Math.max(0, i))];
      focused = n.path;
      (treeEl?.querySelector(`[data-path="${CSS.escape(n.path)}"]`) as HTMLElement | null)?.focus();
    };
    switch (e.key) {
      case "ArrowDown": e.preventDefault(); go(idx + 1); break;
      case "ArrowUp": e.preventDefault(); go(idx - 1); break;
      case "Home": e.preventDefault(); go(0); break;
      case "End": e.preventDefault(); go(visible.length - 1); break;
      case "ArrowRight":
        if (cur && !cur.key) { e.preventDefault(); collapsed[cur.path] = false; }
        break;
      case "ArrowLeft":
        if (cur && !cur.key) { e.preventDefault(); collapsed[cur.path] = true; }
        break;
      case "Enter":
      case " ":
        if (cur) { e.preventDefault(); if (cur.key) store.select(cur.key); else collapsed[cur.path] = !collapsed[cur.path]; }
        break;
      case "Delete":
      case "Backspace":
        if (cur && (e.metaKey || e.ctrlKey)) { e.preventDefault(); if (cur.key) store.deleteRequest(cur.key); else store.deleteFolder(cur.path); }
        break;
      case "F2":
        if (cur) { e.preventDefault(); if (cur.key) store.renameRequest(cur.key); else store.renameFolder(cur.path); }
        break;
    }
  }

  // ---- menú contextual
  let menu = $state<{ x: number; y: number; items: Item[] } | null>(null);
  function openMenu(e: MouseEvent, items: Item[]) {
    e.preventDefault();
    e.stopPropagation();
    menu = { x: e.clientX, y: e.clientY, items };
  }
  function openImportMenu(e: MouseEvent) {
    e.stopPropagation();
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    menu = { x: rect.left, y: rect.bottom + 4, items: [
      { label: "Importar / exportar colección…", action: () => { store.modal = "share"; } },
      ...(store.collection ? [{ label: "Importar desde cURL…", action: () => store.importCurl() }] : []),
    ] };
  }
  const requestMenu = (key: string): Item[] => [
    { label: "Renombrar / mover…", action: () => store.renameRequest(key) },
    { label: "Duplicar…", action: () => store.duplicate(key) },
    { label: "Borrar…", action: () => store.deleteRequest(key), danger: true, sep: true },
  ];
  const folderMenu = (path: string): Item[] => [
    { label: "Ejecutar carpeta…", action: () => store.openRunner(path) },
    { label: "Variables de la carpeta…", action: () => { store.editingFolder = path; store.modal = "folder"; } },
    { label: "Nueva solicitud…", action: () => store.newRequest(path) },
    { label: "Nueva subcarpeta…", action: () => store.newFolder(path) },
    { label: "Renombrar / mover…", action: () => store.renameFolder(path), sep: true },
    { label: "Borrar carpeta…", action: () => store.deleteFolder(path), danger: true },
  ];
  const rootMenu = (): Item[] => [
    { label: "Ejecutar colección…", action: () => store.openRunner() },
    { label: "Servidor mock local…", action: () => { store.modal = "mock"; } },
    { label: "Importar / exportar colección…", action: () => { store.modal = "share"; } },
    { label: "Importar desde cURL…", action: () => store.importCurl() },
    { label: "Nueva solicitud…", action: () => store.newRequest("") },
    { label: "Nueva carpeta…", action: () => store.newFolder("") },
  ];

  // ---- arrastrar y soltar
  let dragging = $state<{ kind: "request" | "folder"; path: string } | null>(null);
  let dropOver = $state<string | null>(null); // "" = raíz

  function dragStart(e: DragEvent, kind: "request" | "folder", path: string) {
    dragging = { kind, path };
    e.dataTransfer?.setData("text/plain", `${kind}:${path}`);
    if (e.dataTransfer) e.dataTransfer.effectAllowed = "move";
  }
  function canDrop(target: string): boolean {
    if (!dragging) return false;
    const parent = dragging.path.includes("/") ? dragging.path.slice(0, dragging.path.lastIndexOf("/")) : "";
    if (parent === target) return false; // ya está ahí
    if (dragging.kind === "folder" && (target === dragging.path || target.startsWith(dragging.path + "/"))) return false;
    return true;
  }
  function dragOver(e: DragEvent, target: string) {
    if (!canDrop(target)) return;
    e.preventDefault();
    e.stopPropagation();
    if (e.dataTransfer) e.dataTransfer.dropEffect = "move";
    dropOver = target;
  }
  async function drop(e: DragEvent, target: string) {
    e.preventDefault();
    e.stopPropagation();
    const d = dragging;
    const ok = !!d && canDrop(target);
    dragging = null;
    dropOver = null;
    if (d && ok) await store.moveInto(d.kind, d.path, target);
  }
  function dragEnd() {
    dragging = null;
    dropOver = null;
  }

  function onGlobalKey(e: KeyboardEvent) {
    if (!(e.metaKey || e.ctrlKey)) return;
    if (e.key === ",") { e.preventDefault(); store.modal = "themes"; return; }
    if (!store.collection) return;
    if (e.shiftKey && e.key.toLowerCase() === "f" && !store.modal && !store.dialog) { e.preventDefault(); store.modal = "search"; }
    else if (e.key === "k") { e.preventDefault(); filterInput?.focus(); filterInput?.select(); }
    else if (e.key === "n" && !e.shiftKey) { e.preventDefault(); store.newRequest(""); }
    else if (e.key === "n" && e.shiftKey) { e.preventDefault(); store.newFolder(""); }
    else if (e.key === "e") { e.preventDefault(); store.modal = "environments"; }
  }
</script>

<svelte:window onkeydown={onGlobalKey} />

{#snippet nodes(list: Node[], depth: number)}
  {#each list as n (n.path + (n.key ? ":r" : ":f"))}
    {#if n.key}
      {@const req = store.collection!.requests[n.key]}
      <div
        class="item"
        class:active={store.selected === n.key}
        class:dragging={dragging?.path === n.key}
        style="padding-left: {12 + depth * 14}px"
        role="treeitem"
        tabindex={focused === n.path || (!focused && store.selected === n.key) ? 0 : -1}
        aria-selected={store.selected === n.key}
        aria-level={depth + 1}
        data-path={n.path}
        draggable="true"
        onclick={() => { focused = n.path; store.select(n.key!); }}
        onkeydown={() => {}}
        onfocus={() => (focused = n.path)}
        oncontextmenu={(e) => openMenu(e, requestMenu(n.key!))}
        ondragstart={(e) => dragStart(e, "request", n.key!)}
        ondragend={dragEnd}
        title={req.name}
      >
        <span class="badge-slot"><MethodBadge method={req.method} /></span>
        <span class="name">{n.name}</span>
        {#if store.selected === n.key && store.dirty}<span class="dot" title="Sin guardar" aria-label="sin guardar"></span>{/if}
      </div>
    {:else}
      <div
        class="folder"
        class:drop-target={dropOver === n.path}
        class:dragging={dragging?.path === n.path && dragging.kind === "folder"}
        style="padding-left: {12 + depth * 14}px"
        role="treeitem"
        tabindex={focused === n.path ? 0 : -1}
        aria-expanded={!collapsed[n.path]}
        aria-selected={false}
        aria-level={depth + 1}
        data-path={n.path}
        draggable="true"
        onclick={() => { focused = n.path; collapsed[n.path] = !collapsed[n.path]; }}
        onkeydown={() => {}}
        onfocus={() => (focused = n.path)}
        oncontextmenu={(e) => openMenu(e, folderMenu(n.path))}
        ondragstart={(e) => dragStart(e, "folder", n.path)}
        ondragend={dragEnd}
        ondragover={(e) => dragOver(e, n.path)}
        ondragleave={() => dropOver === n.path && (dropOver = null)}
        ondrop={(e) => drop(e, n.path)}
      >
        <span class="chev" class:closed={collapsed[n.path]} aria-hidden="true">▾</span>
        <span class="ficon"><Icon name={collapsed[n.path] ? "folder" : "folder-open"} size={17} /></span>
        <span class="fname">{n.name}</span>
        <span class="n muted">{countRequests(n)}</span>
      </div>
      {#if !collapsed[n.path]}
        {#if n.children.length === 0}
          <div class="empty-folder muted" style="padding-left: {26 + (depth + 1) * 14}px">vacía · arrastra aquí o clic derecho</div>
        {/if}
        <div role="group">{@render nodes(n.children, depth + 1)}</div>
      {/if}
    {/if}
  {/each}
{/snippet}

<aside class="sidebar" aria-label="Colección">
  <div class="head">
    <div class="brand">
      <svg width="16" height="16" viewBox="0 0 16 16" fill="none" aria-hidden="true">
        <path d="M3 13L12 4" stroke="var(--accent)" stroke-width="2" stroke-linecap="round" />
        <path d="M13.5 2.5L9 4l3 3 1.5-4.5z" fill="var(--accent)" />
      </svg>
      <span>Harpoon</span>
    </div>
    <div class="actions">
      <button class="btn icon" title="Abrir colección" aria-label="Abrir colección" onclick={() => store.pickAndOpen()}>
        <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" /></svg>
      </button>
      <button class="btn icon" title="Importar y exportar" aria-label="Importar y exportar" aria-haspopup="menu" onclick={openImportMenu}><svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M12 3v12m-4-4 4 4 4-4M4 17v3h16v-3" /></svg></button>
      <button class="btn icon" title="Configuración (⌘,)" aria-label="Configuración" onclick={() => (store.modal = "themes")}>
        <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="12" cy="12" r="3"/><path d="M19 12a7 7 0 0 0-.1-1l2-1.5-2-3.5-2.4 1a7 7 0 0 0-1.7-1L14.5 3h-4l-.3 3a7 7 0 0 0-1.7 1l-2.4-1-2 3.5 2 1.5a7 7 0 0 0 0 2l-2 1.5 2 3.5 2.4-1a7 7 0 0 0 1.7 1l.3 3h4l.3-3a7 7 0 0 0 1.7-1l2.4 1 2-3.5-2-1.5c.1-.3.1-.7.1-1z"/></svg>
      </button>
    </div>
  </div>

  {#if store.pendingInit}
    <div class="init fade-in">
      <p><b>Sin colección</b></p>
      <p class="muted small">No hay <code>harpoon.toml</code> en<br /><span class="mono">{store.pendingInit}</span></p>
      <label class="muted small">Nombre de la colección
        <input bind:value={newName} onkeydown={(e) => e.key === "Enter" && store.createHere(newName)} />
      </label>
      <div class="row">
        <button class="btn primary" onclick={() => store.createHere(newName)}>Crear aquí</button>
        <button class="btn" onclick={() => (store.pendingInit = null)}>Cancelar</button>
      </div>
      <p class="muted small">Se crearán <code>harpoon.toml</code>, <code>environments/dev.toml</code> y <code>requests/health.toml</code>.</p>
    </div>
  {:else if store.collection}
    <div class="collection">
      <div class="col-row">
        <div class="col-name" title={store.collection.root}>{store.collection.meta.name}</div>
        <button class="btn icon" title="Ejecutar colección" aria-label="Ejecutar colección" onclick={() => store.openRunner()}><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M8 5v14l11-7z" /></svg></button>
        <button class="btn icon" title="Ajustes de la colección" aria-label="Ajustes de la colección" onclick={() => (store.modal = "settings")}>
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="12" cy="12" r="3" /><path d="M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3h0a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8v0a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z" /></svg>
        </button>
      </div>
    </div>
    <div class="toolbar">
      <input bind:this={filterInput} placeholder="Filtrar  ⌘K" bind:value={filter} spellcheck="false" aria-label="Filtrar solicitudes" />
      <button class="btn icon" title="Buscar en la colección (⇧⌘F)" aria-label="Buscar en la colección" onclick={() => (store.modal = "search")}><svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true"><circle cx="11" cy="11" r="7"/><path d="m16 16 4 4"/></svg></button>
      <button class="btn icon" title="Nueva solicitud (⌘N)" aria-label="Nueva solicitud" onclick={() => store.newRequest("")}>
        <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true"><path d="M12 5v14M5 12h14" /></svg>
      </button>
      <button class="btn icon" title="Nueva carpeta (⇧⌘N)" aria-label="Nueva carpeta" onclick={() => store.newFolder("")}>
        <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" /><path d="M12 11v5M9.5 13.5h5" /></svg>
      </button>
    </div>
    <div
      class="tree"
      class:drop-target={dropOver === ""}
      role="tree"
      tabindex="-1"
      aria-label="Solicitudes"
      bind:this={treeEl}
      onkeydown={onTreeKey}
      oncontextmenu={(e) => openMenu(e, rootMenu())}
      ondragover={(e) => dragOver(e, "")}
      ondragleave={(e) => { if (e.currentTarget === e.target) dropOver = null; }}
      ondrop={(e) => drop(e, "")}
    >
      {@render nodes(tree.children, 0)}
      {#if tree.children.length === 0}
        <p class="muted small pad">{filter.trim() ? "No hay resultados para este filtro." : "Aún no hay solicitudes. Pulsa + para crear una."}</p>
      {/if}
      {#if dragging}
        <div class="root-drop muted">Suelta aquí para mover a la raíz</div>
      {/if}
    </div>
  {:else}
    <div class="empty">
      <p class="muted">Ninguna colección abierta.</p>
      <button class="btn" onclick={() => store.pickAndOpen()}>Abrir carpeta…</button>
    </div>
  {/if}
</aside>

{#if menu}
  <ContextMenu x={menu.x} y={menu.y} items={menu.items} onclose={() => (menu = null)} />
{/if}

<script module lang="ts">
  function countRequests(n: { key?: string; children: { key?: string; children: unknown[] }[] }): number {
    let c = 0;
    for (const ch of n.children) c += ch.key ? 1 : countRequests(ch as never);
    return c;
  }
</script>

<style>
  .sidebar {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg-panel);
    min-width: 0;
    border-right: 1px solid var(--border);
  }
  .head {
    height: var(--titlebar);
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 10px 0 78px; /* hueco para los semáforos de macOS */
    -webkit-app-region: drag;
    flex: 0 0 auto;
    border-bottom: 1px solid var(--border);
  }
  .brand {
    display: flex;
    min-width: 0;
    overflow: hidden;
    align-items: center;
    gap: 7px;
    font-weight: 600;
    letter-spacing: 0.01em;
  }
  .actions {
    display: flex;
    flex: 0 0 auto;
    gap: 2px;
    -webkit-app-region: no-drag;
  }
  .collection {
    padding: 16px 10px 4px 14px;
  }
  .col-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .col-name {
    flex: 1;
    min-width: 0;
    font-weight: 600;
    font-size: 13px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .toolbar {
    display: flex;
    gap: 2px;
    align-items: center;
    padding: 8px 10px 12px;
    border-bottom: 1px solid var(--border);
  }
  .toolbar input {
    flex: 1;
    min-width: 0;
    font-size: 12px;
    padding: 7px 8px;
  }
  .tree {
    flex: 1;
    overflow: auto;
    padding: 10px 8px 12px;
    outline: none;
  }
  .folder,
  .item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    text-align: left;
    padding: 7px 8px;
    border-radius: var(--radius);
    color: var(--text);
    transition: background var(--t);
    cursor: default;
    user-select: none;
  }
  .folder {
    color: var(--text);
    font-size: 12.5px;
    font-weight: 600;
    margin-top: 2px;
    gap: 6px;
  }
  .fname {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .n {
    font-size: 10.5px;
    font-weight: 400;
    opacity: 0.7;
  }
  .folder:hover,
  .item:hover {
    background: var(--bg-hover);
  }
  .item.active {
    background: var(--accent-soft);
    box-shadow: inset 2px 0 0 var(--accent);
  }
  .chev {
    display: inline-block;
    width: 10px;
    transition: transform var(--t);
    font-size: 10px;
  }
  .chev.closed {
    transform: rotate(-90deg);
  }
  .badge-slot {
    width: 50px;
    flex: 0 0 auto;
    display: inline-flex;
  }
  .ficon {
    display: inline-flex;
    color: var(--text-muted);
    opacity: 0.85;
  }
  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .dot {
    margin-left: auto;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--warn);
  }
  .empty-folder {
    font-size: 11px;
    font-style: italic;
    padding: 3px 0;
  }
  .root-drop {
    margin: 10px 6px 0;
    padding: 10px;
    border: 1px dashed var(--border-strong);
    border-radius: var(--radius);
    text-align: center;
    font-size: 11.5px;
  }
  .init {
    padding: 14px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    border-bottom: 1px solid var(--border);
  }
  .init p {
    margin: 0;
  }
  .small {
    font-size: 11.5px;
    overflow-wrap: anywhere;
  }
  .pad {
    padding: 8px;
  }
  .init label {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .init input {
    font-size: 12px;
  }
  .row {
    display: flex;
    gap: 6px;
  }
  code {
    font-family: var(--mono);
    font-size: 11px;
  }
  .empty {
    padding: 24px 16px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    align-items: flex-start;
  }
</style>
