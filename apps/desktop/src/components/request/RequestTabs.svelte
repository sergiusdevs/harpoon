<script lang="ts">
  import ContextMenu from "../common/ContextMenu.svelte";
  import { store } from "../../state/store.svelte";
  import { requestType } from "../../domain/types";
  import MethodBadge from "../common/MethodBadge.svelte";
  let node = $state<HTMLElement>();
  let width = $state(1000);
  let menu = $state<{ x: number; y: number } | null>(null);
  function measure(node: HTMLElement) {
    const observer = new ResizeObserver(([entry]) => { width = entry.contentRect.width; });
    observer.observe(node); return { destroy: () => observer.disconnect() };
  }
  const visible = $derived.by(() => {
    let used = 0;
    const shown = store.tabs.opened.filter((key, index) => {
      used += Math.max(145, Math.min(210, 70 + (store.tabRequest(key)?.name.length ?? key.length) * 6));
      return index === 0 || used <= width - 78;
    });
    if (store.selected && !shown.includes(store.selected)) shown.splice(Math.max(0, shown.length - 1), 1, store.selected);
    return shown;
  });
  const hidden = $derived(store.tabs.opened.filter((key) => !visible.includes(key)));
  function select(key: string) { store.select(key); }
  function keydown(event: KeyboardEvent, key: string) {
    const keys = store.tabs.opened, index = keys.indexOf(key);
    let next = index;
    if (event.key === "ArrowRight") next = (index + 1) % keys.length;
    else if (event.key === "ArrowLeft") next = (index + keys.length - 1) % keys.length;
    else if (event.key === "Home") next = 0;
    else if (event.key === "End") next = keys.length - 1;
    else return;
    event.preventDefault(); select(keys[next]);
    queueMicrotask(() => node?.querySelector<HTMLElement>(`[data-request="${CSS.escape(keys[next])}"]`)?.focus());
  }
</script>
<nav class="requests" aria-label="Solicitudes abiertas" use:measure bind:this={node}>
  <div class="open-tabs" role="tablist" aria-label="Solicitudes abiertas">
    {#each visible as key (key)}
      {@const request = store.tabRequest(key)}
      {#if request}
        <div class="open-tab" class:active={key === store.selected}>
          {#if key === store.selected}<svg class="active-shape" viewBox="0 0 200 40" preserveAspectRatio="none" aria-hidden="true"><path d="M0 39.5C5.5 39.5 8 36.5 8 31V12C8 5 12 .5 20 .5H180C188 .5 192 5 192 12V31C192 36.5 194.5 39.5 200 39.5" vector-effect="non-scaling-stroke" /></svg>{/if}
          <button role="tab" data-request={key} aria-selected={key === store.selected} tabindex={key === store.selected ? 0 : -1} title={key} onclick={() => select(key)} onkeydown={(event) => keydown(event, key)}>
            <span class="tab-method">{#if requestType(request) === "http"}<MethodBadge method={request.method} />{:else}<span class="protocol">{requestType(request) === "websocket" ? "WS" : requestType(request) === "graphql" ? "GQL" : "SSE"}</span>{/if}</span>
            <span class="label">{request.name.trim() || key.split("/").at(-1)}</span>
            {#if store.tabDirty(key)}<span class="status-dot" aria-label="Cambios sin guardar" title="Cambios sin guardar"></span>{/if}
          </button>
          <button class="close" aria-label={`Cerrar ${request.name}`} title="Cerrar solicitud" onclick={() => store.closeTab(key)}>×</button>
        </div>
      {/if}
    {/each}
  </div>
  {#if hidden.length}<button class="add" aria-label="Más solicitudes abiertas" aria-haspopup="menu" aria-expanded={!!menu} title="Más solicitudes abiertas" onclick={(event) => { event.stopPropagation(); const rect = event.currentTarget.getBoundingClientRect(); menu = menu ? null : { x: rect.left, y: rect.bottom + 4 }; }}><svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" aria-hidden="true"><path d="m5 6 6 6-6 6m8-12 6 6-6 6"/></svg></button>{/if}
  {#if store.collection}<button class="add" aria-label="Nueva solicitud" title="Nueva solicitud (⌘N)" onclick={() => store.newRequest()}>+</button>{/if}
</nav>
{#if menu}<ContextMenu x={menu.x} y={menu.y} items={hidden.map((key) => ({ label: (store.tabRequest(key)?.name ?? key) + (store.tabDirty(key) ? " · sin guardar" : ""), action: () => select(key) }))} onclose={() => (menu = null)} />{/if}
<style>
  .requests { display: flex; min-width: 0; flex: 0 0 auto; background: var(--bg-panel); border-bottom: 1px solid var(--border); padding: 5px 14px 0; gap: 4px; }
  .open-tabs { display: flex; align-items: flex-end; gap: 6px; min-width: 0; padding: 0 8px 1px; margin: 0 -8px -1px; overflow: hidden; flex: 1; }
  .open-tab { display: flex; align-items: center; min-width: 0; flex: 0 0 auto; max-width: 210px; min-width: 145px; height: 34px; position: relative; margin-bottom: 3px; transition: background 140ms ease, transform 140ms ease, border-color 140ms ease; border: 1px solid transparent; border-bottom: 1px solid var(--border); border-radius: 9px; background: color-mix(in srgb, var(--bg-raised) 55%, var(--bg-panel)); color: var(--text-muted); }
  .open-tab.active { height: 37px; margin-bottom: -1px; background: transparent; border-color: transparent; border-radius: 0; color: var(--text); z-index: 2; }
  .active-shape { position: absolute; left: -8px; bottom: -1px; width: calc(100% + 16px); height: calc(100% + 2px); overflow: visible; pointer-events: none; }
  .active-shape path { fill: var(--bg); stroke: var(--border-strong); stroke-width: 1; }
  .open-tab > button { position: relative; z-index: 1; }
  .open-tab > button[role="tab"] { display: flex; align-items: center; gap: 7px; padding: 7px 9px 8px 11px; min-width: 0; max-width: calc(100% - 23px); overflow: hidden; height: 33px; flex: 1; font-size: 11.5px; }
  .open-tab.active .label { font-weight: 600; }
  .open-tab :global(.badge) { flex-shrink: 0; }
  .label { min-width: 0; flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; max-width: 140px; }
  .protocol { color: var(--accent); font: 600 10px var(--mono); }
  .status-dot { display: block; width: 6px; height: 6px; min-width: 6px; max-width: 6px; border: 1px solid var(--warn); background: var(--warn); border-radius: 50%; flex: 0 0 auto; }
  .close { flex: 0 0 20px; opacity: 0; color: var(--text-faint); font-size: 15px; width: 20px; height: 23px; margin-right: 3px; border-radius: 4px; }
  .open-tab.active .close, .open-tab:hover .close, .open-tab:focus-within .close { opacity: 1; }
  .open-tab:hover { color: var(--text); background: var(--bg-raised); border-color: var(--border-strong); }
  .open-tab.active:hover { background: transparent; border-color: transparent; }
  .tab-method { position: relative; display: inline-flex; flex: 0 0 auto; }
  @media (prefers-reduced-motion: reduce) { .open-tab { transition: none; } }
  .close:hover { background: var(--bg-hover); color: var(--text); }
  .add { display: grid; place-items: center; flex: 0 0 26px; align-self: center; height: 26px; font-size: 18px; color: var(--text-faint); border-radius: 4px; }
  .add:hover { background: var(--bg-hover); color: var(--text); }
</style>
