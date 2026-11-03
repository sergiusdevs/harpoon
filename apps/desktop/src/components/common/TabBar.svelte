<script lang="ts">
  import ContextMenu from "./ContextMenu.svelte";
  export interface TabItem { id: string; label: string; count?: string | number; bad?: boolean }
  let { items, active, label, onselect }: { items: TabItem[]; active: string; label: string; onselect: (id: string) => void } = $props();
  let width = $state(1000);
  let menu = $state<{ x: number; y: number } | null>(null);
  let host = $state<HTMLDivElement>();
  function measure(node: HTMLDivElement) {
    const observer = new ResizeObserver(([entry]) => { width = entry.contentRect.width; });
    observer.observe(node); return { destroy: () => observer.disconnect() };
  }
  const visible = $derived.by(() => {
    let used = 0;
    const allFit = items.reduce((sum, item) => sum + 24 + item.label.length * 7 + (item.count ? 26 : 0), 0) <= width;
    const shown = items.filter((item, index) => { used += 24 + item.label.length * 7 + (item.count ? 26 : 0); return index === 0 || used <= width - (allFit ? 0 : 36); });
    const selected = items.find((item) => item.id === active);
    if (selected && !shown.includes(selected)) shown.splice(Math.max(0, shown.length - 1), 1, selected);
    return shown;
  });
  const hidden = $derived(items.filter((item) => !visible.includes(item)));
  function select(id: string) { onselect(id); queueMicrotask(() => host?.querySelector<HTMLButtonElement>(`[data-tab="${id}"]`)?.focus()); }
  function keydown(event: KeyboardEvent, id: string) {
    const index = visible.findIndex((item) => item.id === id);
    let target = index;
    if (event.key === "ArrowRight") target = (index + 1) % visible.length;
    else if (event.key === "ArrowLeft") target = (index + visible.length - 1) % visible.length;
    else if (event.key === "Home") target = 0;
    else if (event.key === "End") target = visible.length - 1;
    else return;
    event.preventDefault(); select(visible[target].id);
  }
</script>
<div class="tabbar" use:measure bind:this={host}>
  <div class="tabs" role="tablist" aria-label={label}>
    {#each visible as item (item.id)}
      <button class="tab" data-tab={item.id} role="tab" aria-selected={active === item.id} tabindex={active === item.id ? 0 : -1} class:active={active === item.id} onclick={() => onselect(item.id)} onkeydown={(event) => keydown(event, item.id)}>{item.label}{#if item.count}<span class="count" class:bad={item.bad}>{item.count}</span>{/if}</button>
    {/each}
  </div>
  {#if hidden.length}<button class="more" aria-label="Más secciones" aria-haspopup="menu" aria-expanded={!!menu} onclick={(event) => { event.stopPropagation(); const rect = event.currentTarget.getBoundingClientRect(); menu = menu ? null : { x: rect.right - 180, y: rect.bottom + 4 }; }} title="Más secciones"><svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" aria-hidden="true"><path d="m5 6 6 6-6 6m8-12 6 6-6 6"/></svg></button>{/if}
</div>
{#if menu}<ContextMenu x={menu.x} y={menu.y} items={hidden.map((item) => ({ label: item.label + (item.count ? ` (${item.count})` : ""), action: () => select(item.id) }))} onclose={() => (menu = null)} />{/if}
<style>
  .tabbar { display: flex; align-items: stretch; width: 100%; min-width: 0; border-bottom: 1px solid var(--border); flex: 0 0 auto; }
  .tabs { flex: 1; min-width: 0; padding: 0 12px; overflow: hidden; border: 0; gap: 0; }
  .tab { font-size: 12px; padding: 12px 10px; margin-bottom: 0; }
  .count { background: transparent; padding: 0; color: var(--text-faint); font-size: 10px; line-height: 1; }
  .count.bad { color: var(--err); }
  .more { flex: 0 0 32px; display: grid; place-items: center; color: var(--text-muted); }
  .more:hover { color: var(--accent); background: var(--bg-hover); }
</style>
