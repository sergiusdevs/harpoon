<script lang="ts">
  import { onMount } from "svelte";
  // Menú contextual ligero, posicionado donde se hizo clic.
  export interface Item {
    label: string;
    action: () => void;
    danger?: boolean;
    sep?: boolean;
  }
  interface Props {
    x: number;
    y: number;
    items: Item[];
    onclose: () => void;
  }
  let { x, y, items, onclose }: Props = $props();
  let el = $state<HTMLDivElement | null>(null);

  onMount(() => {
    const previous = document.activeElement;
    el?.querySelector<HTMLButtonElement>("button")?.focus();
    return () => { if (previous instanceof HTMLElement && previous.isConnected && !previous.closest("[inert]")) previous.focus(); };
  });
  function onKey(event: KeyboardEvent) {
    const buttons = [...(el?.querySelectorAll<HTMLButtonElement>("button") ?? [])];
    const index = buttons.indexOf(document.activeElement as HTMLButtonElement);
    const direction = event.key === "ArrowDown" ? 1 : event.key === "ArrowUp" ? -1 : 0;
    if (direction) { event.preventDefault(); event.stopPropagation(); buttons[(index + direction + buttons.length) % buttons.length]?.focus(); }
    if (event.key === "Escape") { event.preventDefault(); event.stopPropagation(); onclose(); }
    if (event.key === "Tab") onclose();
  }
  // Mantener dentro de la ventana.
  const pos = $derived.by(() => {
    const w = el?.offsetWidth ?? 180;
    const h = el?.offsetHeight ?? 160;
    return {
      left: Math.max(8, Math.min(x, window.innerWidth - w - 8)),
      top: Math.max(8, Math.min(y, window.innerHeight - h - 8)),
    };
  });
</script>

<svelte:window onclick={onclose} oncontextmenu={onclose} onkeydown={(e) => e.key === "Escape" && onclose()} onblur={onclose} />

<div class="menu fade-in" bind:this={el} style="left: {pos.left}px; top: {pos.top}px" role="menu" tabindex="-1" onkeydown={onKey}>
  {#each items as it}
    {#if it.sep}<div class="sep"></div>{/if}
    <button
      role="menuitem"
      class:danger={it.danger}
      onclick={(e) => {
        e.stopPropagation();
        onclose();
        it.action();
      }}>{it.label}</button
    >
  {/each}
</div>

<style>
  .menu {
    position: fixed;
    z-index: 60;
    min-width: 180px;
    background: var(--bg-raised);
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    padding: 4px;
    box-shadow: 0 12px 30px rgba(0, 0, 0, 0.35);
    display: flex;
    flex-direction: column;
  }
  button {
    text-align: left;
    padding: 6px 10px;
    border-radius: 5px;
    font-size: 12.5px;
  }
  button:hover {
    background: var(--bg-hover);
  }
  button.danger {
    color: var(--err);
  }
  .sep {
    height: 1px;
    background: var(--border);
    margin: 4px 2px;
  }
</style>
