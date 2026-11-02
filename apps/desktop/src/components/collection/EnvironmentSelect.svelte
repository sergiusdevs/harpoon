<script lang="ts">
  import { onMount, tick } from "svelte";
  import { store } from "../../state/store.svelte";

  const id = $props.id();
  let root = $state<HTMLDivElement>();
  let trigger = $state<HTMLButtonElement>();
  let open = $state(false);
  let active = $state(0);
  const options = $derived([null, ...Object.keys(store.collection?.environments ?? {}).sort((a, b) => a.localeCompare(b))]);

  function show() {
    active = Math.max(0, options.indexOf(store.env));
    open = true;
  }
  function select(index: number) {
    if (index < 0 || index >= options.length) return;
    store.env = options[index];
    open = false;
    trigger?.focus();
  }
  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Escape" && open) {
      e.preventDefault();
      open = false;
    } else if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      if (!open) show();
      else active = (active + (e.key === "ArrowDown" ? 1 : options.length - 1)) % options.length;
      void tick().then(() => document.getElementById(`${id}-option-${active}`)?.scrollIntoView({ block: "nearest" }));
    } else if (open && (e.key === "Home" || e.key === "End")) {
      e.preventDefault();
      active = e.key === "Home" ? 0 : options.length - 1;
    } else if (open && (e.key === "Enter" || e.key === " ")) {
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

<div class="environment" bind:this={root} onfocusout={(event) => { if (!root?.contains(event.relatedTarget as Node | null)) open = false; }}>
  <span class="label">Entorno</span>
  <div class="select-wrap">
    <button class="trigger" bind:this={trigger} role="combobox" aria-label="Entorno activo" aria-expanded={open}
      aria-controls={open ? `${id}-list` : undefined} aria-activedescendant={open ? `${id}-option-${active}` : undefined}
      aria-haspopup="listbox" onclick={() => open ? (open = false) : show()} onkeydown={onKeydown}>
      <span class="env-dot" class:enabled={!!store.env}></span>
      <span class="current">{store.env ?? "Sin entorno"}</span>
      <svg class:rotated={open} width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="m6 9 6 6 6-6" /></svg>
    </button>
    {#if open}
      <div class="menu">
        <div class="menu-heading">ENTORNO ACTIVO</div>
        <div class="options" role="listbox" id={`${id}-list`} aria-label="Seleccionar entorno">
          {#each options as option, i}
            <button type="button" role="option" tabindex="-1" id={`${id}-option-${i}`} aria-selected={store.env === option}
              class:active={active === i} onpointerdown={(e) => e.preventDefault()} onclick={() => select(i)} onmousemove={() => (active = i)}>
              <span class="env-dot" class:enabled={!!option}></span><span class="option-name">{option ?? "Sin entorno"}</span>
              {#if store.env === option}<span class="check" aria-hidden="true">✓</span>{/if}
            </button>
          {/each}
        </div>
        <div class="menu-actions">
          <button type="button" onclick={() => { open = false; store.quickNewEnvironment(); }}><span aria-hidden="true">＋</span> Nuevo entorno</button>
          <button type="button" onclick={() => { open = false; store.modal = "cookies"; }}><span aria-hidden="true">◉</span> Gestionar cookies</button>
          <button type="button" onclick={() => { open = false; store.modal = "mock"; }}><span aria-hidden="true">◇</span> Servidor mock local</button>
          <button type="button" onclick={() => { open = false; store.modal = "environments"; }}><span aria-hidden="true">⚙</span> Gestionar entornos <span class="shortcut">⌘E</span></button>
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  .environment { display: flex; align-items: center; gap: 8px; flex: 0 0 auto; margin-left: 12px; font-size: 12px; -webkit-app-region: no-drag; }
  .label { color: var(--text-muted); }
  .select-wrap { position: relative; }
  .trigger { display: flex; align-items: center; gap: 8px; min-width: 140px; max-width: 220px; height: 30px; padding: 0 9px; border: 1px solid var(--border); border-radius: var(--radius); background: var(--bg-raised); text-align: left; }
  .trigger:hover, .trigger[aria-expanded="true"] { border-color: var(--border-strong); background: var(--bg-hover); }
  .trigger svg { margin-left: auto; color: var(--text-muted); transition: transform var(--t); }
  .trigger svg.rotated { transform: rotate(180deg); }
  .current, .option-name { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .env-dot { flex: 0 0 auto; width: 7px; height: 7px; border-radius: 50%; background: var(--text-faint); }
  .env-dot.enabled { background: var(--ok); }
  .menu { position: absolute; top: calc(100% + 6px); right: 0; z-index: 30; width: max(100%, 225px); max-width: min(320px, calc(100vw - 16px)); padding: 5px; border: 1px solid var(--border-strong); border-radius: 9px; background: var(--bg-raised); box-shadow: 0 12px 32px rgba(0, 0, 0, .28); }
  .menu-heading { padding: 7px 9px; color: var(--text-muted); font-size: 9px; font-weight: 700; letter-spacing: .06em; }
  .options { max-height: 220px; overflow-y: auto; }
  .options button, .menu-actions button { display: flex; align-items: center; gap: 9px; width: 100%; padding: 8px 9px; border-radius: 5px; text-align: left; }
  .options button:hover, .options button.active, .menu-actions button:hover { background: var(--bg-hover); }
  .check { margin-left: auto; color: var(--accent-strong); }
  .menu-actions { margin-top: 5px; padding-top: 5px; border-top: 1px solid var(--border); color: var(--text-muted); }
  .menu-actions button:hover { color: var(--text); }
  .menu-actions button > span:first-child { width: 14px; text-align: center; font-size: 14px; }
  .shortcut { margin-left: auto; font-size: 10px; color: var(--text-faint); }
</style>
