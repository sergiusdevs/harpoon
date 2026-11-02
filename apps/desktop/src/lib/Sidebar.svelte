<script lang="ts">
  import { store } from "./store.svelte";

  let filter = $state("");

  type Node = { name: string; key?: string; children: Node[] };

  const tree = $derived.by(() => {
    const root: Node = { name: "", children: [] };
    if (!store.collection) return root;
    const q = filter.trim().toLowerCase();
    for (const [key, req] of Object.entries(store.collection.requests)) {
      if (q && !key.toLowerCase().includes(q) && !req.name.toLowerCase().includes(q)) continue;
      const parts = key.split("/");
      let node = root;
      for (const p of parts.slice(0, -1)) {
        let child = node.children.find((c) => c.name === p && !c.key);
        if (!child) {
          child = { name: p, children: [] };
          node.children.push(child);
        }
        node = child;
      }
      node.children.push({ name: parts[parts.length - 1], key, children: [] });
    }
    return root;
  });

  let collapsed = $state<Record<string, boolean>>({});
</script>

{#snippet nodes(list: Node[], depth: number, path: string)}
  {#each list as n (path + "/" + n.name + (n.key ?? ""))}
    {#if n.key}
      {@const req = store.collection!.requests[n.key]}
      <button
        class="item"
        class:active={store.selected === n.key}
        style="padding-left: {12 + depth * 14}px"
        onclick={() => store.select(n.key!)}
        title={req.name}
      >
        <span class="method {req.method}">{req.method}</span>
        <span class="name">{n.name}</span>
        {#if store.selected === n.key && store.dirty}<span class="dot" title="Sin guardar"></span>{/if}
      </button>
    {:else}
      {@const id = path + "/" + n.name}
      <button class="folder" style="padding-left: {12 + depth * 14}px" onclick={() => (collapsed[id] = !collapsed[id])}>
        <span class="chev" class:closed={collapsed[id]}>▾</span>
        <span>{n.name}</span>
      </button>
      {#if !collapsed[id]}
        {@render nodes(n.children, depth + 1, id)}
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

  {#if store.collection}
    <div class="collection">
      <div class="col-name">{store.collection.meta.name}</div>
      <div class="col-path muted" title={store.collection.root}>{store.collection.root}</div>
    </div>
    <div class="search">
      <input placeholder="Filtrar…" bind:value={filter} spellcheck="false" />
    </div>
    <nav class="tree">
      {@render nodes(tree.children, 0, "")}
    </nav>
  {:else}
    <div class="empty">
      <p class="muted">Ninguna colección abierta.</p>
      <button class="btn" onclick={() => store.pickAndOpen()}>Abrir carpeta…</button>
    </div>
  {/if}
</aside>

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
    padding: 8px 12px 4px;
  }
  .col-name {
    font-weight: 600;
    font-size: 13px;
  }
  .col-path {
    font-size: 11px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    direction: rtl;
    text-align: left;
  }
  .search {
    padding: 6px 10px 8px;
  }
  .search input {
    width: 100%;
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
    text-transform: uppercase;
    letter-spacing: 0.04em;
    font-weight: 600;
    margin-top: 4px;
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
  .empty {
    padding: 24px 16px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    align-items: flex-start;
  }
</style>
