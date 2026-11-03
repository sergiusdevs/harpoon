<script lang="ts">
  import { onMount, tick } from "svelte";
  import { store } from "../../state/store.svelte";
  import DevIcon from "../devtools/DevIcon.svelte";

  const id = $props.id();
  let root = $state<HTMLDivElement>();
  let trigger = $state<HTMLButtonElement>();
  let search = $state<HTMLInputElement>();
  let open = $state(false);
  let active = $state(0);
  let query = $state("");
  const names = $derived(Object.keys(store.collection?.environments ?? {}).sort((a, b) => a.localeCompare(b)));
  const options = $derived.by(() => {
    const q = query.trim().toLowerCase();
    return q ? names.filter((n) => n.toLowerCase().includes(q)) : [null, ...names];
  });

  async function show() {
    query = "";
    open = true;
    active = Math.max(0, options.indexOf(store.env));
    await tick();
    search?.focus();
  }
  function close(focusTrigger = true) {
    open = false;
    if (focusTrigger) trigger?.focus();
  }
  function select(index: number) {
    if (index < 0 || index >= options.length) return;
    store.env = options[index];
    close();
  }
  function configure() {
    close();
    store.modal = "environments";
  }
  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Escape" && open) {
      e.preventDefault();
      close();
    } else if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      if (!open) return void show();
      if (!options.length) return;
      active = (active + (e.key === "ArrowDown" ? 1 : options.length - 1)) % options.length;
      void tick().then(() => document.getElementById(`${id}-option-${active}`)?.scrollIntoView({ block: "nearest" }));
    } else if (open && e.key === "Enter") {
      e.preventDefault();
      select(active);
    }
  }

  onMount(() => {
    const dismiss = (event: PointerEvent) => {
      if (!root?.contains(event.target as Node)) open = false;
    };
    document.addEventListener("pointerdown", dismiss);
    return () => document.removeEventListener("pointerdown", dismiss);
  });
</script>

{#snippet dbIcon(size: number)}
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><ellipse cx="12" cy="5.5" rx="7.5" ry="2.8" /><path d="M4.5 5.5v13c0 1.5 3.4 2.8 7.5 2.8s7.5-1.3 7.5-2.8v-13" /><path d="M4.5 12c0 1.5 3.4 2.8 7.5 2.8s7.5-1.3 7.5-2.8" /></svg>
{/snippet}

<div class="environment" bind:this={root} onfocusout={(event) => { if (!root?.contains(event.relatedTarget as Node | null)) open = false; }}>
  <button class="trigger" class:none={!store.env} bind:this={trigger} aria-label="Entorno activo" aria-expanded={open} aria-haspopup="dialog"
    title="Entorno activo" onclick={() => (open ? close() : show())} onkeydown={onKeydown}>
    <span class="db">{@render dbIcon(15)}</span>
    <span class="current">{store.env ?? "Sin entorno"}</span>
    <svg class="caret" class:rotated={open} width="10" height="10" viewBox="0 0 24 24" aria-hidden="true"><path d="M5 8h14l-7 9z" fill="currentColor" /></svg>
  </button>
  {#if open}
    <div class="menu" role="dialog" aria-label="Seleccionar entorno">
      <div class="scopes" role="tablist" aria-label="Ámbito">
        <button type="button" role="tab" class="scope on" aria-selected="true">{@render dbIcon(14)} Colección</button>
        <button type="button" role="tab" class="scope" aria-selected="false" aria-disabled="true" disabled title="Los entornos globales todavía no están disponibles"><DevIcon name="globe" size={14} /> Global</button>
      </div>
      <label class="search">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true"><circle cx="11" cy="11" r="7" /><path d="m20 20-3.5-3.5" /></svg>
        <input bind:this={search} bind:value={query} placeholder="Buscar entornos…" aria-label="Buscar entornos"
          role="combobox" aria-expanded="true" aria-controls={`${id}-list`} aria-activedescendant={options.length ? `${id}-option-${active}` : undefined}
          oninput={() => (active = 0)} onkeydown={onKeydown} />
      </label>
      <div class="options" role="listbox" id={`${id}-list`} aria-label="Entornos de la colección">
        {#each options as option, i}
          <button type="button" role="option" tabindex="-1" id={`${id}-option-${i}`} aria-selected={store.env === option}
            class:active={active === i} class:selected={store.env === option} class:none={option === null}
            onpointerdown={(e) => e.preventDefault()} onclick={() => select(i)} onmousemove={() => (active = i)}>
            {option ?? "Sin entorno"}
          </button>
        {:else}
          <p class="empty">{names.length ? "Ningún entorno coincide" : "Aún no hay entornos"}</p>
        {/each}
      </div>
      <div class="footer">
        <button type="button" class="new" title="Nuevo entorno" aria-label="Nuevo entorno" onclick={() => { close(false); store.quickNewEnvironment(); }}>＋</button>
        <button type="button" class="configure" onclick={configure}>
          <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="12" cy="12" r="3" /><path d="M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3 1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8 1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z" /></svg>
          Configurar <span class="shortcut">⌘E</span>
        </button>
      </div>
    </div>
  {/if}
</div>

<style>
  .environment { position: relative; flex: 0 0 auto; margin-left: 4px; font-size: 12px; -webkit-app-region: no-drag; }
  .trigger { display: flex; align-items: center; gap: 7px; max-width: 200px; height: 30px; padding: 0 10px; border: 1px solid var(--border); border-radius: var(--radius); background: var(--bg-raised); }
  .trigger:hover, .trigger[aria-expanded="true"] { border-color: var(--border-strong); background: var(--bg-hover); }
  .db { display: flex; color: var(--accent); }
  .trigger.none .db, .trigger.none .current { color: var(--text-muted); }
  .current { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .caret { flex: 0 0 auto; color: var(--text-muted); transition: transform var(--t); }
  .caret.rotated { transform: rotate(180deg); }

  .menu { position: absolute; top: calc(100% + 6px); right: 0; z-index: 30; display: flex; flex-direction: column; width: 260px; max-width: calc(100vw - 16px); border: 1px solid var(--border-strong); border-radius: 9px; background: var(--bg-raised); box-shadow: 0 12px 32px rgba(0, 0, 0, .28); overflow: hidden; }
  .scopes { display: flex; gap: 14px; padding: 8px 12px 0; }
  .scope { display: flex; align-items: center; gap: 6px; padding: 6px 2px 8px; border-bottom: 2px solid transparent; color: var(--text-muted); font-size: 12.5px; }
  .scope.on { color: var(--text); border-bottom-color: var(--accent); }
  .scope.on :global(svg) { color: var(--accent); }
  .scope:disabled { opacity: .55; cursor: default; }
  .search { display: flex; align-items: center; gap: 7px; margin: 10px 8px 6px; padding: 0 9px; height: 30px; border: 1px solid var(--border); border-radius: var(--radius); color: var(--text-muted); }
  .search:focus-within { border-color: var(--accent); }
  .search input { flex: 1; min-width: 0; border: 0; outline: 0; background: transparent; color: var(--text); font: inherit; }
  .options { max-height: 240px; min-height: 60px; overflow-y: auto; padding: 0 6px 6px; }
  .options button { display: block; width: 100%; padding: 8px 12px; border-radius: 6px; text-align: left; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .options button.none { color: var(--text-muted); }
  .options button.active { background: var(--bg-hover); }
  .options button.selected { background: var(--bg-active); color: var(--accent-strong); }
  .empty { margin: 0; padding: 12px; color: var(--text-muted); text-align: center; }
  .footer { display: flex; align-items: stretch; border-top: 1px solid var(--border); }
  .footer button { display: flex; align-items: center; justify-content: center; gap: 8px; padding: 9px; color: var(--text-muted); }
  .footer button:hover { background: var(--bg-hover); color: var(--text); }
  .new { width: 38px; border-right: 1px solid var(--border); font-size: 14px; }
  .configure { flex: 1; }
  .shortcut { font-size: 10px; color: var(--text-faint); }
</style>
