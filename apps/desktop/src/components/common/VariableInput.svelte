<script lang="ts">
  import { onMount, tick } from "svelte";
  import { variableAtCursor, isKnownVariable, type VariableSuggestion } from "../../domain/variables";
  interface Props {
    value: string;
    oninput: (event: Event & { currentTarget: HTMLInputElement }) => void;
    onkeydown?: (event: KeyboardEvent & { currentTarget: HTMLInputElement }) => void;
    onpaste?: (event: ClipboardEvent & { currentTarget: HTMLInputElement }) => void;
    placeholder?: string;
    label: string;
    title?: string;
    inputClass?: string;
    compact?: boolean;
    integrated?: boolean;
    variables?: VariableSuggestion[];
  }

  let { value, oninput, onkeydown, onpaste, placeholder = "", label, title, inputClass = "", compact = false, integrated = false, variables }: Props = $props();
  const id = $props.id();
  let input = $state<HTMLInputElement>();
  let scrollLeft = $state(0);
  let fragment = $state<ReturnType<typeof variableAtCursor>>(null);
  let active = $state(0);
  let dismissed = $state(false);
  let popup = $state({ left: 0, top: 0, width: 0, height: 220, above: false });
  const matches = $derived(fragment && variables ? variables.filter((item) => item.name.toLowerCase().includes(fragment!.query)) : []);

  function position() {
    if (!input || !fragment) return;
    const rect = input.getBoundingClientRect();
    const below = window.innerHeight - rect.bottom - 8;
    const above = rect.top - 8;
    const openAbove = below < 160 && above > below;
    const width = Math.min(Math.max(rect.width, 260), window.innerWidth - 16);
    popup = { left: Math.max(8, Math.min(rect.left, window.innerWidth - width - 8)), top: openAbove ? rect.top - 4 : rect.bottom + 4, width, height: Math.max(60, Math.min(240, openAbove ? above : below)), above: openAbove };
  }
  function update(inputElement: HTMLInputElement, reopen = false) {
    if (reopen) dismissed = false;
    fragment = variables && !dismissed ? variableAtCursor(inputElement.value, inputElement.selectionStart ?? inputElement.value.length) : null;
    active = 0;
    position();
  }
  function choose(item: VariableSuggestion) {
    if (!input || !fragment) return;
    const el = input;
    const cursor = fragment.start + item.name.length + 4;
    el.setRangeText(`{{${item.name}}}`, fragment.start, fragment.end, "end");
    fragment = null;
    dismissed = true;
    el.dispatchEvent(new Event("input", { bubbles: true }));
    void tick().then(() => { el.focus(); el.setSelectionRange(cursor, cursor); });
  }
  function keydown(event: KeyboardEvent & { currentTarget: HTMLInputElement }) {
    if (fragment && matches.length && (event.key === "ArrowDown" || event.key === "ArrowUp")) {
      event.preventDefault();
      active = (active + (event.key === "ArrowDown" ? 1 : matches.length - 1)) % matches.length;
      void tick().then(() => document.getElementById(`${id}-option-${active}`)?.scrollIntoView({ block: "nearest" }));
      return;
    }
    if (fragment && matches.length && (event.key === "Enter" || event.key === "Tab")) {
      event.preventDefault();
      choose(matches[active]);
      return;
    }
    if (fragment && event.key === "Escape") { event.preventDefault(); fragment = null; dismissed = true; return; }
    onkeydown?.(event);
  }
  onMount(() => {
    window.addEventListener("resize", position);
    window.addEventListener("scroll", position, true);
    return () => { window.removeEventListener("resize", position); window.removeEventListener("scroll", position, true); };
  });
  const parts = $derived(value.split(/(\{\{[^{}]+\}\})/g));
</script>

<div class="variable-input" class:compact class:integrated>
  <div class="highlight" aria-hidden="true"><div class="highlight-text" style:transform={`translateX(${-scrollLeft}px)`}>{#each parts as part}{#if part.startsWith("{{") && part.endsWith("}}") }<span class="variable" class:unknown={variables && !isKnownVariable(part.slice(2, -2), variables)}>{part}</span>{:else}{part}{/if}{/each}</div></div>
  <input
    bind:this={input}
    class={inputClass}
    role={variables ? "combobox" : undefined}
    aria-autocomplete={variables ? "list" : undefined}
    aria-expanded={variables ? !!fragment : undefined}
    aria-controls={fragment ? `${id}-list` : undefined}
    aria-activedescendant={fragment && matches.length ? `${id}-option-${active}` : undefined}
    autocomplete="off"
    {value}
    {placeholder}
    aria-label={label}
    {title}
    spellcheck="false"
    oninput={(event) => { oninput(event); update(event.currentTarget, true); }}
    onkeydown={keydown}
    onkeyup={(event) => { if (["ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) update(event.currentTarget, true); }}
    onclick={(event) => update(event.currentTarget, true)}
    onblur={() => { fragment = null; dismissed = false; }}
    {onpaste}
    onscroll={(event) => (scrollLeft = event.currentTarget.scrollLeft)}
  />
</div>
{#if fragment && variables}
  <div class="variable-menu" style:left={`${popup.left}px`} style:top={`${popup.top}px`} style:width={`${popup.width}px`} style:max-height={`${popup.height}px`} style:transform={popup.above ? "translateY(-100%)" : undefined}>
    <div class="menu-heading">VARIABLES <span>↑↓ · Enter</span></div>
    <div role="listbox" id={`${id}-list`} aria-label="Variables disponibles">
      {#each matches as item, index}
        <button type="button" role="option" tabindex="-1" id={`${id}-option-${index}`} aria-selected={active === index} class:active={active === index}
          onpointerdown={(event) => event.preventDefault()} onclick={() => choose(item)} onmousemove={() => (active = index)}>
          <span class="variable-name">{item.name}</span><span class="source">{item.source}</span>
        </button>
      {:else}
        <div class="no-matches">No hay variables con ese nombre</div>
      {/each}
    </div>
  </div>
{/if}

<style>
  .variable-input { position: relative; min-width: 0; width: 100%; background: var(--bg-raised); border-radius: var(--radius); }
  .highlight {
    position: absolute; inset: 1px; overflow: hidden; pointer-events: none;
    display: flex; align-items: center; padding: 0 10px;
    font: 12.5px/normal var(--mono); white-space: pre; color: var(--text);
  }
  .compact { background: transparent; }
  .compact .highlight { padding: 0 8px; font-size: 12px; }
  .compact input { padding: 5px 8px; font-size: 12px; }
  .integrated, .integrated:focus-within { background: transparent; }
  .integrated input { border-color: transparent; box-shadow: none; }
  .integrated input:focus-visible { outline: none; }
  .highlight-text { min-width: max-content; }
  .variable { color: var(--accent-strong); }
  .variable.unknown { color: var(--warn); text-decoration: underline dotted; text-underline-offset: 3px; }
  .variable-menu { position: fixed; z-index: 100; overflow-y: auto; padding: 5px; border: 1px solid var(--border-strong); border-radius: 9px; background: var(--bg-raised); box-shadow: 0 12px 32px rgba(0, 0, 0, .28); }
  .menu-heading { display: flex; justify-content: space-between; padding: 6px 8px; color: var(--text-muted); font-size: 9px; font-weight: 700; letter-spacing: .06em; }
  .menu-heading span { font-size: 10px; font-weight: 400; letter-spacing: 0; }
  .variable-menu button { display: flex; align-items: center; gap: 12px; width: 100%; padding: 8px; border-radius: 5px; text-align: left; }
  .variable-menu button:hover, .variable-menu button.active { background: var(--accent-soft); }
  .variable-name { min-width: 0; overflow: hidden; text-overflow: ellipsis; font: 12px var(--mono); color: var(--accent-strong); }
  .source { margin-left: auto; color: var(--text-muted); font-size: 11px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .no-matches { padding: 9px 8px; color: var(--text-muted); font-size: 12px; }
  input {
    position: relative; display: block; width: 100%; min-width: 0;
    background: transparent; color: transparent; caret-color: var(--text);
    font: 12.5px/normal var(--mono); padding: 8px 10px;
  }
  input::placeholder { color: var(--text-faint); opacity: 1; }
  input::selection { color: transparent; background: color-mix(in srgb, var(--accent) 35%, transparent); }
  .variable-input:focus-within { background: var(--bg-raised); border-radius: var(--radius); }
  .compact:focus-within { background: transparent; }
</style>
