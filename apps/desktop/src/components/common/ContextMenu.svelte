<script lang="ts">
  import { onMount } from "svelte";
  import Icon from "./Icon.svelte";
  type IconName = import("svelte").ComponentProps<typeof Icon>["name"];
  // Menú contextual ligero, posicionado donde se hizo clic.
  export interface Item {
    label: string;
    action: () => void;
    danger?: boolean;
    sep?: boolean;
    /** Icono a la izquierda (opcional). */
    icon?: IconName;
    /** Atajo con ⌘/Ctrl: una sola tecla, p. ej. "n". Se muestra como "⌘ N" y funciona con el menú abierto. */
    shortcut?: string;
    /** Etiqueta pequeña a la derecha, p. ej. "Beta". */
    badge?: string;
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
    if ((event.metaKey || event.ctrlKey) && !event.shiftKey && !event.altKey) {
      const hit = items.find((it) => it.shortcut && it.shortcut.toLowerCase() === event.key.toLowerCase());
      if (hit) { event.preventDefault(); event.stopPropagation(); onclose(); hit.action(); return; }
    }
    if (event.key === "Escape") { event.preventDefault(); event.stopPropagation(); onclose(); }
    if (event.key === "Tab") onclose();
  }
  // Mantener dentro de la ventana.
  const pos = $derived.by(() => {
    const w = el?.offsetWidth ?? 240;
    const h = el?.offsetHeight ?? 160;
    return {
      left: Math.max(8, Math.min(x, window.innerWidth - w - 8)),
      top: Math.max(8, Math.min(y, window.innerHeight - h - 8)),
    };
  });
</script>

<svelte:window onclick={onclose} oncontextmenu={onclose} onkeydown={(e) => e.key === "Escape" && onclose()} onblur={onclose} />

<div class="menu fade-in" class:rich={items.some((it) => it.icon)} bind:this={el} style="left: {pos.left}px; top: {pos.top}px" role="menu" tabindex="-1" onkeydown={onKey}>
  {#each items as it}
    {#if it.sep}<div class="sep"></div>{/if}
    <button
      role="menuitem"
      class:danger={it.danger}
      onclick={(e) => {
        e.stopPropagation();
        onclose();
        it.action();
      }}
    >
      {#if it.icon}<span class="ico"><Icon name={it.icon} size={16} /></span>{/if}
      <span class="label">{it.label}</span>
      {#if it.badge}<span class="badge">{it.badge}</span>{/if}
      {#if it.shortcut}<kbd>⌘ {it.shortcut.toUpperCase()}</kbd>{/if}
    </button>
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
    display: flex;
    align-items: center;
    gap: 10px;
    text-align: left;
    padding: 6px 10px;
    border-radius: 5px;
    font-size: 12.5px;
  }
  .label { flex: 1; white-space: nowrap; }
  .menu.rich { min-width: 240px; padding: 5px; border-radius: 9px; }
  .menu.rich button { padding: 7px 10px; font-size: 13px; gap: 12px; }
  .ico { display: inline-flex; color: var(--text-muted); flex: 0 0 auto; }
  button:hover .ico { color: var(--text); }
  button.danger .ico { color: var(--err); }
  kbd { margin-left: 16px; font: 11.5px var(--font); color: var(--text-faint); letter-spacing: 0.04em; white-space: nowrap; }
  .badge { margin-left: 12px; padding: 2px 7px; border-radius: 5px; font-size: 10.5px; color: var(--accent); background: color-mix(in srgb, var(--accent) 16%, transparent); }
  button:hover,
  button:focus-visible {
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
