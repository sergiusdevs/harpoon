<script lang="ts">
  import BodyTypeIcon from "./BodyTypeIcon.svelte";
  import { onDestroy, tick } from "svelte";
  import type { Body } from "../../domain/types";
  let { value, onchange }: { value: Body["type"]; onchange: (type: Body["type"]) => void } = $props();
  const groups: { label: string; items: { value: Body["type"]; label: string }[] }[] = [
    { label: "Formulario", items: [{ value: "multipart", label: "Multipart" }, { value: "form", label: "URL encoded" }] },
    { label: "Contenido", items: [{ value: "json", label: "JSON" }, { value: "xml", label: "XML" }, { value: "text", label: "Texto" }] },
    { label: "Otros", items: [{ value: "binary", label: "Archivo / binario" }, { value: "none", label: "Sin cuerpo" }] },
  ];
  const menuId = $props.id();
  let trigger = $state<HTMLButtonElement>();
  let menu = $state<HTMLDivElement>();
  let position = $state<{ left: number; top: number } | null>(null);
  const selected = $derived(groups.flatMap((group) => group.items).find((item) => item.value === value));
  function close() { position = null; }
  function open() {
    if (position) { close(); return; }
    const rect = trigger!.getBoundingClientRect();
    position = { left: Math.max(8, Math.min(rect.right - 225, window.innerWidth - 233)), top: Math.max(8, Math.min(rect.bottom + 5, window.innerHeight - 390)) };
    void tick().then(() => {
      if (!menu || !position) return;
      const height = menu.offsetHeight;
      position = { ...position, top: rect.bottom + height + 12 <= window.innerHeight ? rect.bottom + 5 : Math.max(8, rect.top - height - 5) };
      (menu.querySelector<HTMLButtonElement>('[aria-checked="true"]') ?? menu.querySelector<HTMLButtonElement>("button"))?.focus();
    });
  }
  function choose(type: Body["type"]) { close(); trigger?.focus(); onchange(type); }
  function keydown(event: KeyboardEvent) {
    if (event.key === "Escape") { event.preventDefault(); event.stopPropagation(); close(); trigger?.focus(); }
    if (event.key === "Tab") { close(); trigger?.focus(); }
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault(); const buttons = [...menu!.querySelectorAll<HTMLButtonElement>("button")];
      const index = buttons.indexOf(document.activeElement as HTMLButtonElement);
      buttons[(index + (event.key === "ArrowDown" ? 1 : -1) + buttons.length) % buttons.length]?.focus();
    }
  }
  onDestroy(close);
</script>
<svelte:window onclick={close} onresize={close} onblur={close} />
<button class="trigger" bind:this={trigger} aria-label="Tipo de cuerpo" aria-haspopup="menu" aria-expanded={!!position} aria-controls={position ? menuId : undefined} class:open={!!position} onclick={(event) => { event.stopPropagation(); open(); }} onkeydown={(event) => { if (event.key === "ArrowDown") { event.preventDefault(); open(); } }}><BodyTypeIcon type={value} />{selected?.label ?? "GraphQL"}<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><path d="m6 9 6 6 6-6"/></svg></button>
{#if position}
  <div class="body-menu" role="menu" id={menuId} aria-label="Formato del cuerpo" tabindex="-1" bind:this={menu} style:left={`${position.left}px`} style:top={`${position.top}px`} onkeydown={keydown}>
    {#each groups as group, index}<div class="menu-group" class:divided={index > 0}><div class="group-label">{group.label}</div>{#each group.items as item}<button role="menuitemradio" aria-checked={value === item.value} class:active={value === item.value} onclick={(event) => { event.stopPropagation(); choose(item.value); }}><span class="symbol"><BodyTypeIcon type={item.value} /></span><span>{item.label}</span><span class="check" aria-hidden="true">{value === item.value ? "✓" : ""}</span></button>{/each}</div>{/each}
  </div>
{/if}
<style>
  .trigger { flex: 0 0 auto; display: inline-flex; align-items: center; gap: 7px; color: var(--text-muted); font-size: 11.5px; padding: 0 10px; margin: 6px; border: 1px solid transparent; border-radius: 5px; white-space: nowrap; }
  .trigger:hover, .trigger.open { color: var(--text); background: var(--bg-raised); border-color: var(--border-strong); }
  .body-menu { position: fixed; z-index: 65; width: 225px; max-height: calc(100vh - 16px); overflow-y: auto; border: 1px solid var(--border-strong); border-radius: 7px; background: var(--bg-raised); padding: 5px; box-shadow: 0 8px 20px #0003; }
  .menu-group { padding: 3px 0; }
  .menu-group.divided { border-top: 1px solid var(--border); margin-top: 3px; }
  .group-label { padding: 5px 9px 4px; font-size: 10px; color: var(--text-faint); font-weight: 500; }
  .body-menu button { display: flex; align-items: center; gap: 9px; width: 100%; min-height: 31px; padding: 6px 9px; text-align: left; font-size: 12px; border-radius: 4px; color: var(--text); }
  .body-menu button:hover { background: var(--bg-hover); }
  .body-menu button.active { background: var(--bg-active); }
  .symbol { display: inline-flex; flex: 0 0 17px; color: var(--text-muted); }
  .check { margin-left: auto; width: 15px; color: var(--accent); font-size: 12px; }
</style>
