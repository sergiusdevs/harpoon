<script lang="ts">
  import { store, basename } from "./store.svelte";
  import ContextMenu, { type Item } from "./ContextMenu.svelte";

  let filter = $state("");
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
  let menu = $state<{ x: number; y: number; items: Item[] } | null>(null);

  function openMenu(e: MouseEvent, items: Item[]) {
    e.preventDefault();
    e.stopPropagation();
    menu = { x: e.clientX, y: e.clientY, items };
  }

  const requestMenu = (key: string): Item[] => [
    { label: "Renombrar / mover…", action: () => store.renameRequest(key) },
    { label: "Duplicar…", action: () => store.duplicate(key) },
    { label: "Borrar…", action: () => store.deleteRequest(key), danger: true, sep: true },
  ];
  const folderMenu = (path: string): Item[] => [
    { label: "Nueva solicitud…", action: () => store.newRequest(path) },
    { label: "Nueva subcarpeta…", action: () => store.newFolder(path) },
    { label: "Renombrar / mover…", action: () => store.renameFolder(path), sep: true },
    { label: "Borrar carpeta…", action: () => store.deleteFolder(path), danger: true },
  ];
  const rootMenu = (): Item[] => [
    { label: "Nueva solicitud…", action: () => store.newRequest("") },
    { label: "Nueva carpeta…", action: () => store.newFolder("") },
  ];
</script>

{#snippet nodes(list: Node[], depth: number)}
  {#each list as n (n.path + (n.key ? ":r" : ":f"))}
    {#if n.key}
      {@const req = store.collection!.requests[n.key]}
      <button
        class="item"
        class:active={store.selected === n.key}
        style="padding-left: {12 + depth * 14}px"
        onclick={() => store.select(n.key!)}
        oncontextmenu={(e) => openMenu(e, requestMenu(n.key!))}
        title={req.name}
      >
        <span class="method {req.method}">{req.method}</span>
        <span class="name">{n.name}</span>
        {#if store.selected === n.key && store.dirty}<span class="dot" title="Sin guardar"></span>{/if}
      </button>
    {:else}
      <button
        class="folder"
        style="padding-left: {12 + depth * 14}px"
        onclick={() => (collapsed[n.path] = !collapsed[n.path])}
        oncontextmenu={(e) => openMenu(e, folderMenu(n.path))}
      >
        <span class="chev" class:closed={collapsed[n.path]}>▾</span>
        <span class="fname">{n.name}</span>
        <span class="n muted">{countRequests(n)}</span>
      </button>
      {#if !collapsed[n.path]}
        {#if n.children.length === 0}
          <div class="empty-folder muted" style="padding-left: {26 + (depth + 1) * 14}px">vacía</div>
        {/if}
        {@render nodes(n.children, depth + 1)}
      {/if}
    {/if}
  {/each}
{/snippet}

<aside class="sidebar">
  <div class="head">
    <div class="brand">
      <svg width="16" height="16" viewBox="0 0 16 16" fill="none" aria-hidden="true">
        <path d="M3 13L12 4" stroke="var(--accent)" stroke-width="2" stroke-linecap="round" />
        <path d="M13.5 2.5L9 4l3 3 1.5-4.5z" fill="var(--accent)" />
      </svg>
      <span>Harpoon</span>
    </div>
    <div class="actions">
      <button class="btn icon" title="Abrir colección" onclick={() => store.pickAndOpen()}>
        <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" /></svg>
      </button>
      <button class="btn icon" title="Cambiar tema" onclick={() => store.toggleTheme()}>
        {#if store.theme === "dark"}
          <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><circle cx="12" cy="12" r="4" /><path d="M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4" /></svg>
        {:else}
          <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 12.8A9 9 0 1 1 11.2 3a7 7 0 0 0 9.8 9.8z" /></svg>
        {/if}
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
        <button class="btn icon" title="Ajustes de la colección" onclick={() => (store.modal = "settings")}>
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3" /><path d="M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3h0a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8v0a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z" /></svg>
        </button>
      </div>
    </div>
    <div class="toolbar">
      <input placeholder="Filtrar…" bind:value={filter} spellcheck="false" />
      <button class="btn icon" title="Nueva solicitud" onclick={() => store.newRequest("")}>
        <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M12 5v14M5 12h14" /></svg>
      </button>
      <button class="btn icon" title="Nueva carpeta" onclick={() => store.newFolder("")}>
        <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" /><path d="M12 11v5M9.5 13.5h5" /></svg>
      </button>
    </div>
    <nav class="tree" oncontextmenu={(e) => openMenu(e, rootMenu())}>
      {@render nodes(tree.children, 0)}
      {#if tree.children.length === 0}
        <p class="muted small pad">Sin solicitudes. Pulsa + o clic derecho para crear una.</p>
      {/if}
    </nav>
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
  }
  .head {
    height: var(--titlebar);
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 10px 0 78px; /* hueco para los semáforos de macOS */
    -webkit-app-region: drag;
    flex: 0 0 auto;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 7px;
    font-weight: 600;
    letter-spacing: 0.01em;
  }
  .actions {
    display: flex;
    gap: 2px;
    -webkit-app-region: no-drag;
  }
  .collection {
    padding: 6px 8px 0 12px;
  }
  .col-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .col-name {
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
    padding: 6px 8px 6px 10px;
  }
  .toolbar input {
    flex: 1;
    min-width: 0;
    font-size: 12px;
    padding: 5px 8px;
  }
  .tree {
    flex: 1;
    overflow: auto;
    padding: 2px 6px 12px;
  }
  .folder,
  .item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    text-align: left;
    padding: 5px 8px;
    border-radius: var(--radius);
    color: var(--text);
    transition: background var(--t);
  }
  .folder {
    color: var(--text-muted);
    font-size: 12px;
    font-weight: 600;
    margin-top: 2px;
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
  .method {
    width: 48px;
    flex: 0 0 auto;
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
    word-break: break-all;
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
