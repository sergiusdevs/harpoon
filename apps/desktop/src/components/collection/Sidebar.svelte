<script lang="ts">
  import { streams } from "../../state/streams.svelte";
  import { store, basename } from "../../state/store.svelte";
  import ContextMenu, { type Item } from "../common/ContextMenu.svelte";
  import Icon from "../common/Icon.svelte";
  import { requestType } from "../../domain/types";

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
    const q = fold(filter.trim());
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
      if (q && !fold(req.name).includes(q) && !fold(key).includes(q) && !fold(req.url).includes(q)) continue;
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
  /** Con búsqueda activa se despliegan todas las carpetas que contienen coincidencias. */
  const searching = $derived(!!filter.trim());
  const isOpen = (path: string) => searching || !collapsed[path];
  let searchOpen = $state(true);
  function toggleSearch() {
    if (searchOpen && !filter) { searchOpen = false; return; }
    searchOpen = true;
    queueMicrotask(() => { filterInput?.focus(); filterInput?.select(); });
  }
  function searchKey(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      if (filter) filter = ""; else filterInput?.blur();
    } else if (e.key === "ArrowDown" && visible.length) {
      e.preventDefault();
      focused = visible[0].path;
      (treeEl?.querySelector(`[data-path="${CSS.escape(visible[0].path)}"]`) as HTMLElement | null)?.focus();
    }
  }

  /** Lista plana de los nodos visibles, para la navegación con teclado. */
  const visible = $derived.by(() => {
    const out: Node[] = [];
    const walk = (list: Node[]) => {
      for (const n of list) {
        out.push(n);
        if (!n.key && (searching || !collapsed[n.path])) walk(n.children);
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
      case "Escape":
        if (filter) { e.preventDefault(); filter = ""; filterInput?.focus(); }
        break;
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
      case "d":
      case "r":
      case "t":
      case "n":
        if (cur && (e.metaKey || e.ctrlKey) && !e.shiftKey && !e.altKey) {
          const items = cur.key ? requestMenu(cur.key) : folderMenu(cur.path);
          const hit = items.find((it) => it.shortcut === e.key);
          if (hit) { e.preventDefault(); hit.action(); }
        }
        break;
      case "F2":
        if (cur) { e.preventDefault(); if (cur.key) store.renameRequest(cur.key); else store.renameFolder(cur.path); }
        break;
    }
  }

  // ---- menú contextual
  let menu = $state<{ x: number; y: number; items: Item[]; owner?: string } | null>(null);
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
  function openMenuAt(e: MouseEvent, items: Item[], owner: string) {
    e.preventDefault();
    e.stopPropagation();
    if (menu?.owner === owner) { menu = null; return; }
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    menu = { x: rect.left, y: rect.bottom + 4, items, owner };
  }
  /** Pliega la carpeta y todas sus subcarpetas. */
  function collapseAll(path: string) {
    collapsed[path] = true;
    for (const f of store.folders) if (f.startsWith(path + "/")) collapsed[f] = true;
  }
  const requestMenu = (key: string): Item[] => [
    { label: "Clonar", icon: "copy", shortcut: "d", action: () => store.duplicate(key) },
    { label: "Renombrar / mover", icon: "pencil", shortcut: "r", action: () => store.renameRequest(key) },
    { label: "Eliminar", icon: "x", action: () => store.deleteRequest(key), sep: true },
  ];
  const folderMenu = (path: string): Item[] => [
    { label: "Nueva solicitud", icon: "file-plus", shortcut: "n", action: () => store.newRequest(path) },
    { label: "Nueva carpeta", icon: "folder-plus", action: () => store.newFolder(path) },
    { label: "Ejecutar", icon: "play", action: () => store.openRunner(path) },
    { label: "Clonar", icon: "copy", shortcut: "d", action: () => store.cloneFolder(path) },
    { label: "Renombrar", icon: "pencil", shortcut: "r", action: () => store.renameFolder(path) },
    { label: "Plegar", icon: "collapse", action: () => collapseAll(path) },
    { label: "Mostrar en Finder", icon: "reveal", action: () => store.revealFolder(path) },
    { label: "Ajustes", icon: "settings", sep: true, action: () => { store.editingFolder = path; store.modal = "folder"; } },
    { label: "Abrir en Terminal", icon: "terminal", shortcut: "t", action: () => store.openTerminal(path) },
    { label: "Eliminar", icon: "x", action: () => store.deleteFolder(path) },
  ];
  const addMenu = (): Item[] => [
    { label: "Nueva solicitud", icon: "file-plus", shortcut: "n", action: () => store.newRequest("") },
    { label: "Nueva carpeta", icon: "folder-plus", action: () => store.newFolder("") },
    { label: "Importar desde cURL", icon: "terminal", sep: true, action: () => store.importCurl() },
  ];
  const headerMenu = (): Item[] => [
    { label: "Buscar en el contenido…", icon: "file", action: () => { store.modal = "search"; } },
    ...rootMenu().filter((item) => item.label !== "Nueva solicitud" && item.label !== "Nueva carpeta" && item.label !== "Importar desde cURL"),
  ];
  const rootMenu = (): Item[] => [
    { label: "Nueva solicitud", icon: "file-plus", shortcut: "n", action: () => store.newRequest("") },
    { label: "Nueva carpeta", icon: "folder-plus", action: () => store.newFolder("") },
    { label: "Ejecutar", icon: "play", action: () => store.openRunner() },
    { label: "Validar colección", icon: "vars", action: () => { store.modal = "validate"; } },
    { label: "Importar desde cURL", icon: "terminal", action: () => store.importCurl() },
    { label: "Importar / exportar", icon: "share", action: () => { store.modal = "share"; } },
    { label: "Plegar todo", icon: "collapse", action: () => { for (const f of store.folders) collapsed[f] = true; } },
    { label: "Mostrar en Finder", icon: "reveal", action: () => store.revealFolder("") },
    { label: "Servidor mock", icon: "server", badge: "Beta", action: () => { store.modal = "mock"; } },
    { label: "Cookies", icon: "lock", action: () => { store.modal = "cookies"; } },
    { label: "Ajustes", icon: "settings", sep: true, action: () => (store.modal = "settings") },
    { label: "Abrir en Terminal", icon: "terminal", shortcut: "t", action: () => store.openTerminal("") },
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
    if (store.modal || store.dialog || e.defaultPrevented) return;
    if (!(e.metaKey || e.ctrlKey)) return;
    if (e.key === ",") { e.preventDefault(); store.modal = "themes"; return; }
    if (!store.collection) return;
    if (e.shiftKey && e.key.toLowerCase() === "f" && !store.modal && !store.dialog) { e.preventDefault(); store.modal = "search"; }
    else if (e.key === "k") { e.preventDefault(); searchOpen = true; queueMicrotask(() => { filterInput?.focus(); filterInput?.select(); }); }
    else if (e.key === "n" && !e.shiftKey) { e.preventDefault(); store.newRequest(""); }
    else if (e.key === "n" && e.shiftKey) { e.preventDefault(); store.newFolder(""); }
    else if (e.key === "e") { e.preventDefault(); store.modal = "environments"; }
  }
</script>

<svelte:window onkeydown={onGlobalKey} />

{#snippet hl(text: string)}
  {#each highlight(text, filter.trim()) as part}{#if part.hit}<mark>{part.text}</mark>{:else}{part.text}{/if}{/each}
{/snippet}

{#snippet guides(depth: number)}
  {#each Array(depth) as _, i}<span class="guide" style="left: {GUIDE0 + i * INDENT}px" aria-hidden="true"></span>{/each}
{/snippet}

{#snippet nodes(list: Node[], depth: number)}
  {#each list as n (n.path + (n.key ? ":r" : ":f"))}
    {#if n.key}
      {@const req = store.collection!.requests[n.key]}
      <div
        class="item"
        class:active={store.selected === n.key}
        class:dragging={dragging?.path === n.key}
        style="--depth: {depth}"
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
        {@render guides(depth)}
        {#if requestType(req) === "http"}<span class="verb {req.method}" title={req.method}>{SHORT[req.method] ?? req.method}</span>{:else}<span class="verb protocol">{requestType(req) === "websocket" ? "WS" : requestType(req) === "graphql" ? "GQL" : "SSE"}</span>{/if}
        <span class="name">{@render hl(req.name || n.name)}</span>
        {#if store.selected === n.key && store.dirty}<span class="dot" title="Sin guardar" aria-label="sin guardar"></span>{/if}
        <button class="more" class:open={menu?.owner === "req:" + n.key} tabindex="-1" title="Más acciones" aria-label="Más acciones de {req.name || n.name}" onclick={(e) => openMenuAt(e, requestMenu(n.key!), "req:" + n.key)}><Icon name="dots" size={16} /></button>
      </div>
    {:else}
      <div
        class="folder"
        class:drop-target={dropOver === n.path}
        class:dragging={dragging?.path === n.path && dragging.kind === "folder"}
        style="--depth: {depth}"
        role="treeitem"
        tabindex={focused === n.path ? 0 : -1}
        aria-expanded={isOpen(n.path)}
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
        {@render guides(depth)}
        <span class="ficon" class:open={isOpen(n.path)}><Icon name={isOpen(n.path) ? "folder-open" : "folder"} size={16} /></span>
        <span class="fname">{@render hl(n.name)}</span>
        <button class="more" class:open={menu?.owner === n.path} tabindex="-1" title="Más acciones" aria-label="Más acciones de {n.name}" onclick={(e) => openMenuAt(e, folderMenu(n.path), n.path)}><Icon name="dots" size={16} /></button>
      </div>
      {#if isOpen(n.path)}
        {#if n.children.length === 0 && !searching}
          <div class="empty-folder" style="--depth: {depth + 1}">{@render guides(depth + 1)}<span class="empty-label">Vacía</span></div>
        {/if}
        <div role="group">{@render nodes(n.children, depth + 1)}</div>
      {/if}
    {/if}
  {/each}
{/snippet}

<aside class="sidebar" aria-label="Colección">
  <div class="head" data-tauri-drag-region>
    <span class="brand" data-tauri-drag-region>Harpoon</span>
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
    <div class="col-row">
      <span class="collection-icon"><Icon name="folder-open" size={16} /></span>
      <div class="col-name" title={`${store.collection.root} · ${Object.keys(store.collection.requests).length} solicitudes`}>{store.collection.meta.name}</div>
      <button class="btn icon head-btn" class:on={searchOpen} title="Buscar solicitudes (⌘K)" aria-label="Buscar solicitudes" aria-pressed={searchOpen} onclick={toggleSearch}><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true"><circle cx="11" cy="11" r="7"/><path d="m16 16 4 4"/></svg></button>
      <button class="btn icon head-btn" class:open={menu?.owner === ":add"} title="Nuevo (⌘N · ⇧⌘N)" aria-label="Nueva solicitud o carpeta" aria-haspopup="menu" onclick={(e) => openMenuAt(e, addMenu(), ":add")}><svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true"><path d="M12 5v14M5 12h14" /></svg></button>
      <button class="btn icon head-btn" class:open={menu?.owner === ":root"} title="Más acciones" aria-label="Más acciones de la colección" aria-haspopup="menu" onclick={(e) => openMenuAt(e, headerMenu(), ":root")}><svg width="15" height="15" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><circle cx="12" cy="5" r="1.6"/><circle cx="12" cy="12" r="1.6"/><circle cx="12" cy="19" r="1.6"/></svg></button>
    </div>
    {#if searchOpen}
      <div class="search">
        <svg class="search-icon" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true"><circle cx="11" cy="11" r="7"/><path d="m16 16 4 4"/></svg>
        <input bind:this={filterInput} placeholder="Buscar solicitudes…" bind:value={filter} spellcheck="false" autocomplete="off" aria-label="Buscar solicitudes por nombre, ruta o URL" onkeydown={searchKey} />
        {#if filter}<button class="clear" title="Limpiar (Esc)" aria-label="Limpiar búsqueda" onclick={() => { filter = ""; filterInput?.focus(); }}><Icon name="x" size={12} /></button>{:else}<kbd>⌘K</kbd>{/if}
      </div>
    {/if}
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

    <nav class="app-actions" aria-label="Accesos de la aplicación">
      {#if store.collection}<button class="btn app-action" onclick={() => (store.modal = "mock")}><svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" aria-hidden="true"><rect x="3" y="3" width="18" height="7" rx="2"/><rect x="3" y="14" width="18" height="7" rx="2"/><path d="M7 6h1M7 17h1"/></svg><span>Servidor mock</span></button>{/if}
      {#if streams.sessions.length}
        <button class="btn app-action" onclick={() => (store.modal = "stream")}>
          <span class="connection-dot" class:connected={streams.activeCount > 0}></span>
          <span>Conexiones</span><span class="connection-count">{streams.activeCount} {streams.activeCount === 1 ? "activa" : "activas"}</span>
        </button>
      {/if}
      <button class="btn app-action" title="Abrir colección" aria-label="Abrir colección" onclick={() => store.pickAndOpen()}>
        <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" /></svg>
        <span>Abrir colección</span>
      </button>
      <button class="btn app-action" title="Importar y exportar" aria-label="Importar y exportar" aria-haspopup="menu" onclick={openImportMenu}><svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M12 3v12m-4-4 4 4 4-4M4 17v3h16v-3" /></svg><span>Importar / exportar</span></button>
      <button class="btn app-action" title="Configuración (⌘,)" aria-label="Configuración" onclick={() => (store.modal = "themes")}>
        <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="12" cy="12" r="3"/><path d="M19 12a7 7 0 0 0-.1-1l2-1.5-2-3.5-2.4 1a7 7 0 0 0-1.7-1L14.5 3h-4l-.3 3a7 7 0 0 0-1.7 1l-2.4-1-2 3.5 2 1.5a7 7 0 0 0 0 2l-2 1.5 2 3.5 2.4-1a7 7 0 0 0 1.7 1l.3 3h4l.3-3a7 7 0 0 0 1.7-1l2.4 1 2-3.5-2-1.5c.1-.3.1-.7.1-1z"/></svg>
        <span>Configuración</span><kbd>⌘,</kbd>
      </button>
    </nav>
</aside>

{#if menu}
  <ContextMenu x={menu.x} y={menu.y} items={menu.items} onclose={() => (menu = null)} />
{/if}

<script module lang="ts">
  /** Sangría por nivel y posición de la primera guía vertical (centro del icono de carpeta). */
  const INDENT = 16;
  const GUIDE0 = 16;
  /** Minúsculas y sin acentos, para comparar. */
  function fold(text: string): string {
    return text.normalize("NFD").replace(/[\u0300-\u036f]/g, "").toLowerCase();
  }
  /** Parte el texto en tramos marcando la coincidencia (sin distinguir mayúsculas ni acentos). */
  function highlight(text: string, query: string): { text: string; hit: boolean }[] {
    const q = fold(query);
    if (!q) return [{ text, hit: false }];
    // Correspondencia entre índices del texto plegado y del original.
    let folded = "";
    const origin: number[] = [];
    for (let i = 0; i < text.length; i++) {
      const piece = fold(text[i]);
      for (let j = 0; j < piece.length; j++) origin.push(i);
      folded += piece;
    }
    origin.push(text.length);
    const parts: { text: string; hit: boolean }[] = [];
    let from = 0;
    let at = folded.indexOf(q);
    while (at >= 0 && q.length) {
      const start = origin[at];
      const end = origin[at + q.length - 1] + 1;
      if (start > from) parts.push({ text: text.slice(from, start), hit: false });
      parts.push({ text: text.slice(start, end), hit: true });
      from = end;
      at = folded.indexOf(q, at + q.length);
    }
    if (from < text.length) parts.push({ text: text.slice(from), hit: false });
    return parts;
  }
  const SHORT: Record<string, string> = { GET: "GET", POST: "POST", PUT: "PUT", PATCH: "PATCH", DELETE: "DEL", HEAD: "HEAD", OPTIONS: "OPT" };
</script>

<style>
  .sidebar {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    background: var(--bg-panel);
    min-width: 0;
    border-right: 1px solid var(--border);
  }
  .head {
    height: var(--titlebar);
    display: flex;
    align-items: center;
    padding: 0 14px 0 78px; /* hueco para los semáforos de macOS */
    -webkit-app-region: drag;
    flex: 0 0 auto;
    border-bottom: 1px solid var(--border);
  }
  .brand {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 600;
    letter-spacing: 0.01em;
  }
  .app-actions {
    margin-top: auto;
    padding: 8px;
    border-top: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex: 0 0 auto;
  }
  .app-action {
    width: 100%;
    min-height: 32px;
    padding: 7px 8px;
    gap: 9px;
    border: 0;
    background: transparent;
    color: var(--text-muted);
    font-size: 12px;
    text-align: left;
  }
  .app-action:hover {
    color: var(--text);
  }
  .app-action :global(svg) {
    flex: 0 0 auto;
  }
  .connection-dot { width: 7px; height: 7px; border-radius: 50%; background: var(--text-faint); margin: 0 4px; }
  .connection-dot.connected { background: var(--ok); }
  .connection-count { margin-left: auto; font-size: 10px; }
  .app-action kbd {
    margin-left: auto;
    font: 11px var(--font);
    color: var(--text-faint);
  }
  .collection-icon { display: inline-flex; color: var(--accent); margin-right: 8px; }
  .col-row {
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 12px 8px 8px 14px;
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
  .head-btn { width: 26px; height: 26px; padding: 0; color: var(--text-muted); }
  .head-btn:hover, .head-btn.open { color: var(--text); }
  .head-btn.on { color: var(--accent); }
  .search {
    position: relative;
    display: flex;
    align-items: center;
    margin: 0 10px 8px;
  }
  .search input {
    flex: 1;
    min-width: 0;
    font-size: 12px;
    padding: 6px 30px 6px 29px;
    border-radius: 7px;
    background: var(--bg);
  }
  .search-icon { position: absolute; left: 10px; color: var(--text-faint); pointer-events: none; }
  .search kbd { position: absolute; right: 8px; font: 10.5px var(--font); color: var(--text-faint); pointer-events: none; }
  .search .clear { position: absolute; right: 5px; display: grid; place-items: center; width: 20px; height: 20px; border-radius: 4px; color: var(--text-muted); }
  .search .clear:hover { color: var(--text); background: var(--bg-hover); }
  mark { background: color-mix(in srgb, var(--warn) 30%, transparent); color: inherit; border-radius: 2px; padding: 0 1px; margin: 0 -1px; }
  .tree {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 10px 8px 12px;
    outline: none;
  }
  .folder,
  .item,
  .empty-folder {
    position: relative;
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    min-height: 28px;
    text-align: left;
    padding: 0 6px 0 calc(8px + var(--depth, 0) * 16px);
    border-radius: 4px;
    color: var(--text);
    font-size: 12.5px;
    transition: background var(--t);
    cursor: default;
    user-select: none;
  }
  .fname {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  /* Guía vertical de 1px por nivel de profundidad. */
  .guide {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 1px;
    background: var(--border);
    pointer-events: none;
  }
  /* Botón ··· : aparece al pasar por encima (o con el menú abierto). */
  .more {
    display: none;
    flex: 0 0 auto;
    align-items: center;
    justify-content: center;
    width: 22px;
    height: 20px;
    margin-left: auto;
    border-radius: 4px;
    color: var(--text-muted);
  }
  .more:hover { color: var(--text); background: var(--bg-active, var(--border)); }
  .folder:hover .more,
  .folder:focus-visible .more,
  .item:hover .more,
  .item:focus-visible .more,
  .more.open { display: inline-flex; }
  .item:hover .dot,
  .item:has(.more.open) .dot { display: none; }
  .folder:has(.more.open),
  .item:has(.more.open) { background: var(--bg-hover); }
  .folder:hover,
  .item:hover {
    background: var(--bg-hover);
  }
  .item.active {
    background: var(--bg-active);
    box-shadow: inset 2px 0 0 var(--accent);
  }
  .verb {
    flex: 0 0 auto;
    min-width: 31px;
    font: 600 10.5px var(--mono);
    letter-spacing: 0.01em;
  }
  .verb.GET { color: var(--m-get); }
  .verb.POST { color: var(--m-post); }
  .verb.PUT { color: var(--m-put); }
  .verb.PATCH { color: var(--m-patch); }
  .verb.DELETE { color: var(--m-delete); }
  .verb.HEAD,
  .verb.OPTIONS { color: var(--m-head); }
  .verb.protocol { color: var(--accent); }
  .ficon {
    display: inline-flex;
    flex: 0 0 auto;
    color: var(--text-muted);
  }
  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .dot {
    flex: 0 0 auto;
    width: 6px;
    height: 6px;
    margin-right: 6px;
    border-radius: 50%;
    background: var(--warn);
  }
  .empty-folder {
    min-height: 24px;
    font-size: 11px;
    color: var(--text-faint);
  }
  .empty-label { padding-left: 22px; }
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
