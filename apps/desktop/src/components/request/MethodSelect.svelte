<script lang="ts">
  import { tick } from "svelte";
  import { METHODS, type Method } from "../../domain/types";
  let { value, onchange }: { value: Method; onchange: (method: Method) => void } = $props();
  const id = $props.id();
  let trigger = $state<HTMLButtonElement>();
  let menu = $state<HTMLDivElement>();
  let position = $state<{ left: number; top: number } | null>(null);
  function close() { position = null; }
  async function open() {
    if (position) { close(); return; }
    const rect = trigger!.getBoundingClientRect();
    position = { left: Math.max(8, Math.min(rect.left, window.innerWidth - 138)), top: rect.bottom + 5 };
    await tick(); if (!menu || !position) return;
    if (rect.bottom + menu.offsetHeight + 10 > window.innerHeight) position.top = Math.max(8, rect.top - menu.offsetHeight - 5);
    menu.querySelector<HTMLButtonElement>('[aria-selected="true"]')?.focus();
  }
  function choose(method: Method) { close(); trigger?.focus(); onchange(method); }
  function keydown(event: KeyboardEvent) {
    if (event.key === "Escape") { event.preventDefault(); event.stopPropagation(); close(); trigger?.focus(); }
    if (event.key === "Tab") { close(); trigger?.focus(); }
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault(); const buttons = [...menu!.querySelectorAll<HTMLButtonElement>("button")]; const index = buttons.indexOf(document.activeElement as HTMLButtonElement);
      buttons[(index + (event.key === "ArrowDown" ? 1 : -1) + buttons.length) % buttons.length]?.focus();
    }
  }
</script>
<svelte:window onclick={close} onresize={close} onblur={close} />
<button class="trigger method {value}" bind:this={trigger} aria-label={`Método HTTP: ${value}`} aria-haspopup="listbox" aria-expanded={!!position} aria-controls={position ? id : undefined} onclick={(event) => { event.stopPropagation(); void open(); }} onkeydown={(event) => { if (event.key === "ArrowDown") { event.preventDefault(); void open(); } }}>{value}<svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" aria-hidden="true"><path d="m6 9 6 6 6-6"/></svg></button>
{#if position}
<div class="method-menu" id={id} role="listbox" aria-label="Método HTTP" tabindex="-1" bind:this={menu} style:left={`${position.left}px`} style:top={`${position.top}px`} onkeydown={keydown}>
  {#each METHODS as method}<button class="option" class:active={value === method} role="option" aria-selected={value === method} onclick={(event) => { event.stopPropagation(); choose(method); }}><span class="method {method}">{method}</span>{#if value === method}<span class="check" aria-hidden="true">✓</span>{/if}</button>{/each}
</div>
{/if}
<style>
  .trigger { display: flex; align-items: center; justify-content: space-between; gap: 8px; flex: 0 0 74px; padding: 0 9px; align-self: stretch; font-size: 12px; border-radius: 5px; margin: 3px; min-height: 28px; background: color-mix(in srgb, currentColor 10%, transparent); }
  .trigger:hover, .trigger[aria-expanded="true"] { background: var(--bg-active); }
  .trigger svg { color: var(--text-muted); }
  .method-menu { position: fixed; z-index: 65; width: 130px; background: var(--bg-raised); border: 1px solid var(--border-strong); border-radius: 6px; padding: 5px; box-shadow: 0 8px 20px #0003; }
  .option { display: flex; align-items: center; justify-content: space-between; width: 100%; padding: 8px 9px; border-radius: 4px; text-align: left; }
  .option:hover { background: var(--bg-hover); }
  .option.active { background: var(--bg-active); }
  .check { color: var(--accent); font-size: 12px; }
</style>
