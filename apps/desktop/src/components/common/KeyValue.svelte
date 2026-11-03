<script lang="ts">
  import { parseBulkEntries } from "../../domain/bulkEntries";
  import { onMount, tick } from "svelte";
  import VariableInput from "./VariableInput.svelte";
  import type { VariableSuggestion } from "../../domain/variables";
  // Editor de pares clave/valor: fila vacía al final, botón "Añadir",
  // borrado accesible por teclado y pegado en bloque ("Clave: valor" por línea).
  interface Props {
    entries: Record<string, string>;
    onchange: (entries: Record<string, string>) => void;
    keyPlaceholder?: string;
    valuePlaceholder?: string;
    /** Texto del botón de añadir (p. ej. "Añadir cabecera"). */
    addLabel?: string;
    suggestions?: readonly string[];
    variables?: VariableSuggestion[];
    disabledKeys?: string[];
    descriptions?: Record<string, string>;
    caseInsensitive?: boolean;
    onDescriptionsChange?: (descriptions: Record<string, string>) => void;
    onDisabledChange?: (keys: string[]) => void;
  }
  let { entries, onchange, keyPlaceholder = "clave", valuePlaceholder = "valor", addLabel = "Añadir", suggestions = [], variables, disabledKeys = [], onDisabledChange, descriptions = {}, onDescriptionsChange, caseInsensitive = false }: Props = $props();

  const sameKey = (first: string, second: string) => caseInsensitive ? first.toLowerCase() === second.toLowerCase() : first === second;
  const isDisabled = (key: string) => disabledKeys.some((disabled) => sameKey(disabled, key));
  const suggestionsId = $props.id();

  let rows = $state<{ k: string; v: string; d: string }[]>([]);
  let bulk = $state(false);
  let bulkText = $state("");
  let bulkError = $state("");
  let table = $state<HTMLTableElement | null>(null);
  let openRow = $state<number | null>(null);
  let activeOption = $state(-1);
  let anchor: HTMLInputElement | null = null;
  let popup = $state({ left: 0, top: 0, width: 0, maxHeight: 220, above: false });
  const matches = $derived.by(() => {
    if (openRow === null || !rows[openRow]) return [];
    const query = rows[openRow].k.trim().toLowerCase();
    return suggestions.filter((item) => item.toLowerCase().includes(query));
  });

  function positionPopup() {
    if (!anchor || openRow === null || !anchor.isConnected) return;
    const rect = anchor.getBoundingClientRect();
    const below = window.innerHeight - rect.bottom - 8;
    const above = rect.top - 8;
    const openAbove = below < 160 && above > below;
    const maxHeight = Math.max(80, Math.min(240, openAbove ? above : below));
    const width = Math.min(Math.max(rect.width, 250), window.innerWidth - 16);
    popup = {
      left: Math.max(8, Math.min(rect.left, window.innerWidth - width - 8)),
      top: openAbove ? rect.top - 4 : rect.bottom + 4,
      width,
      maxHeight,
      above: openAbove,
    };
  }

  function openSuggestions(i: number, input: HTMLInputElement) {
    if (!suggestions.length) return;
    anchor = input;
    openRow = i;
    activeOption = -1;
    positionPopup();
  }

  function closeSuggestions() {
    openRow = null;
    activeOption = -1;
    anchor = null;
  }

  function chooseSuggestion(value: string) {
    if (openRow === null || !anchor) return;
    const input = anchor;
    const row = openRow;
    edit(row, "k", value);
    closeSuggestions();
    tick().then(() => input.closest("tr")?.querySelector<HTMLInputElement>("input.v")?.focus());
  }

  onMount(() => {
    window.addEventListener("resize", positionPopup);
    window.addEventListener("scroll", positionPopup, true);
    return () => {
      window.removeEventListener("resize", positionPopup);
      window.removeEventListener("scroll", positionPopup, true);
    };
  });

  $effect(() => {
    const next = Object.entries(entries).map(([k, v]) => ({ k, v, d: descriptions[k] ?? "" }));
    rows = [...next, { k: "", v: "", d: "" }];
  });

  function commit() {
    const out: Record<string, string> = {};
    for (const r of rows) if (r.k.trim()) out[r.k.trim()] = r.v;
    onchange(out);
    onDescriptionsChange?.(Object.fromEntries(rows.filter((row) => row.k.trim() && row.d.trim()).map((row) => [row.k.trim(), row.d])));
  }

  function edit(i: number, field: "k" | "v" | "d", val: string) {
    const oldKey = rows[i].k.trim();
    rows[i][field] = val;
    if (field === "k" && onDisabledChange && isDisabled(oldKey)) {
      const next = val.trim();
      onDisabledChange(disabledKeys.filter((key) => !sameKey(key, oldKey)).concat(next ? [next] : []));
    }
    if (i === rows.length - 1 && (rows[i].k || rows[i].v)) rows.push({ k: "", v: "", d: "" });
    commit();
  }

  function remove(i: number) {
    const key = rows[i].k.trim();
    if (onDisabledChange && isDisabled(key)) onDisabledChange(disabledKeys.filter((item) => !sameKey(item, key)));
    rows.splice(i, 1);
    if (rows.length === 0 || rows[rows.length - 1].k) rows.push({ k: "", v: "", d: "" });
    commit();
    focusKey(Math.min(i, rows.length - 1));
  }

  function add() {
    if (rows[rows.length - 1].k) rows.push({ k: "", v: "", d: "" });
    focusKey(rows.length - 1);
  }

  function focusKey(i: number) {
    queueMicrotask(() => {
      (table?.querySelectorAll<HTMLInputElement>("input.k")[i])?.focus();
    });
  }

  /** Pegar varias líneas "Clave: valor" (o "clave=valor") a la vez. */
  function onPaste(e: ClipboardEvent, i: number) {
    const text = e.clipboardData?.getData("text") ?? "";
    const lines = text.split(/\r?\n/).map((l) => l.trim()).filter(Boolean);
    if (lines.length < 2 && !/[:=]/.test(text)) return;
    const parsed = lines
      .map((l) => {
        const m = l.match(/^([^:=]+?)\s*[:=]\s*(.*)$/);
        return m ? { k: m[1].trim(), v: m[2].trim(), d: descriptions[m[1].trim()] ?? "" } : null;
      })
      .filter((x): x is { k: string; v: string; d: string } => !!x);
    if (!parsed.length) return;
    e.preventDefault();
    rows.splice(i, rows[i].k ? 0 : 1, ...parsed);
    if (rows[rows.length - 1].k) rows.push({ k: "", v: "", d: "" });
    commit();
  }

  function openBulk() {
    bulkText = Object.entries(entries).map(([key, value]) => [key, value, ...(onDescriptionsChange ? [descriptions[key] ?? ""] : [])].join("\t")).join("\n");
    bulkError = ""; bulk = true;
  }
  function applyBulk() {
    try {
      const parsed = parseBulkEntries(bulkText);
      rows = [...parsed.map((row) => ({ ...row, d: row.d || descriptions[row.k] || "" })), { k: "", v: "", d: "" }];
      if (onDisabledChange) onDisabledChange(disabledKeys.filter((key) => parsed.some((row) => sameKey(row.k, key))));
      commit(); bulk = false; bulkError = "";
    } catch (error) { bulkError = error instanceof Error ? error.message : String(error); }
  }

  function onKey(e: KeyboardEvent, i: number, field: "k" | "v") {
    if (field === "k" && suggestions.length) {
      if (e.key === "Escape" && openRow === i) {
        e.preventDefault();
        closeSuggestions();
        return;
      }
      if (e.key === "ArrowDown" || e.key === "ArrowUp") {
        e.preventDefault();
        if (openRow !== i) openSuggestions(i, e.currentTarget as HTMLInputElement);
        if (matches.length) {
          activeOption = e.key === "ArrowDown"
            ? (activeOption + 1) % matches.length
            : (activeOption <= 0 ? matches.length - 1 : activeOption - 1);
          void tick().then(() => document.getElementById(`${suggestionsId}-option-${activeOption}`)?.scrollIntoView({ block: "nearest" }));
        }
        return;
      }
      if (e.key === "Enter" && openRow === i && activeOption >= 0 && matches[activeOption]) {
        e.preventDefault();
        chooseSuggestion(matches[activeOption]);
        return;
      }
    }
    if (e.key === "Enter") {
      e.preventDefault();
      if (field === "k") (e.currentTarget as HTMLElement).closest("tr")?.querySelector<HTMLInputElement>("input.v")?.focus();
      else if (i === rows.length - 1) add();
      else focusKey(i + 1);
    } else if (e.key === "Backspace" && (e.metaKey || e.ctrlKey) && i < rows.length - 1) {
      e.preventDefault();
      remove(i);
    }
  }
</script>

{#if bulk}
  <div class="bulk-editor">
    <label for={`${suggestionsId}-bulk`}>Una fila por línea: nombre, valor y descripción separados por tabuladores. También acepta Nombre: valor.</label>
    <textarea id={`${suggestionsId}-bulk`} aria-label="Edición en bloque" bind:value={bulkText} spellcheck="false"></textarea>
    {#if bulkError}<p class="bulk-error" role="alert">{bulkError}</p>{/if}
    <div class="bulk-actions"><button class="btn small" onclick={() => (bulk = false)}>Cancelar</button><button class="btn small primary" onclick={applyBulk}>Aplicar cambios</button></div>
  </div>
{:else}
<div class="kv-frame">
<table class="kv" bind:this={table}>
  <thead><tr>{#if onDisabledChange}<th class="toggle" scope="col"><span class="sr-only">Activo</span></th>{/if}<th scope="col">Nombre</th><th scope="col">Valor</th>{#if onDescriptionsChange}<th class="description" scope="col">Descripción</th>{/if}<th class="delete-head" scope="col"><span class="sr-only">Eliminar</span></th></tr></thead>
  <tbody>
    {#each rows as row, i (i)}
      <tr class:disabled-row={!!row.k.trim() && isDisabled(row.k.trim())}>
        {#if onDisabledChange}<td class="toggle">{#if row.k.trim()}<input type="checkbox" checked={!isDisabled(row.k.trim())} aria-label={`Activar ${row.k}`} onchange={(e) => onDisabledChange?.(e.currentTarget.checked ? disabledKeys.filter((key) => !sameKey(key, row.k.trim())) : [...disabledKeys, row.k.trim()])} />{/if}</td>{/if}
        <td class="key-cell">
          <input class="k" autocomplete="off" value={row.k} placeholder={keyPlaceholder} aria-label="{keyPlaceholder} {i + 1}"
            role={suggestions.length ? "combobox" : undefined} aria-autocomplete={suggestions.length ? "list" : undefined}
            aria-expanded={suggestions.length ? openRow === i && matches.length > 0 : undefined}
            aria-controls={suggestions.length && openRow === i && matches.length ? `${suggestionsId}-list` : undefined}
            aria-activedescendant={openRow === i && activeOption >= 0 && matches[activeOption] ? `${suggestionsId}-option-${activeOption}` : undefined}
            onfocus={(e) => openSuggestions(i, e.currentTarget)} onblur={closeSuggestions}
            oninput={(e) => { edit(i, "k", e.currentTarget.value); openSuggestions(i, e.currentTarget); }}
            onpaste={(e) => onPaste(e, i)} onkeydown={(e) => onKey(e, i, "k")} spellcheck="false" />
        </td>
        <td>
          <VariableInput {variables} inputClass="v" value={row.v} placeholder={valuePlaceholder} label="{valuePlaceholder} {i + 1}" oninput={(e) => edit(i, "v", e.currentTarget.value)} onkeydown={(e) => onKey(e, i, "v")} compact />
        </td>
        {#if onDescriptionsChange}<td class="description"><input class="d" value={row.d} placeholder="Descripción" aria-label={`Descripción ${i + 1}`} disabled={!row.k.trim()} oninput={(event) => edit(i, "d", event.currentTarget.value)} /></td>{/if}
        <td class="del">
          {#if i < rows.length - 1}
            <button class="del" title="Quitar (⌘⌫)" aria-label="Quitar fila {i + 1}" onclick={() => remove(i)}><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" aria-hidden="true"><path d="M3 6h18M9 6V3h6v3M5 6l1 15h12l1-15M10 10v7M14 10v7"/></svg></button>
          {/if}
        </td>
      </tr>
    {/each}
  </tbody>
</table>
</div>
{#if openRow !== null && matches.length}
  <div class="suggestions" style:left={`${popup.left}px`} style:top={`${popup.top}px`} style:width={`${popup.width}px`} style:max-height={`${popup.maxHeight}px`} style:transform={popup.above ? "translateY(-100%)" : undefined}>
    <div class="suggestions-title">CABECERAS SUGERIDAS <span>↑↓ para navegar</span></div>
    <div role="listbox" id={`${suggestionsId}-list`} aria-label="Cabeceras sugeridas">
      {#each matches as suggestion, index}
        <button type="button" role="option" tabindex="-1" id={`${suggestionsId}-option-${index}`}
          aria-selected={activeOption === index} class:active={activeOption === index}
          onpointerdown={(e) => e.preventDefault()} onclick={() => chooseSuggestion(suggestion)}>
          <span class="suggestion-icon" aria-hidden="true">#</span><span>{suggestion}</span>
        </button>
      {/each}
    </div>
  </div>
{/if}
<div class="kv-actions">
  <button class="btn small" onclick={add}>+ {addLabel}</button>
  <button class="bulk-link" onclick={openBulk}>Edición en bloque</button>
</div>
{/if}

<style>
  .kv-frame { border: 1px solid var(--border-strong); border-radius: 7px; overflow: hidden; }
  .kv { table-layout: fixed; border-collapse: collapse; width: 100%; }
  .kv th { height: 35px; font-size: 12px; font-weight: 600; color: var(--text); text-align: left; padding: 8px 10px; background: var(--bg-panel); border-bottom: 1px solid var(--border-strong); }
  .kv th + th, .kv td + td { border-left: 1px solid var(--border); }
  .kv td { height: 38px; padding: 3px 4px; border-bottom: 1px solid var(--border); }
  .kv tr:last-child td { border-bottom: 0; }
  .kv th.toggle { width: 28px; padding: 0; }
  .kv th.delete-head { width: 30px; padding: 0; }
  .kv .description { width: 27%; }
  .kv input { min-width: 0; font-family: var(--font); font-size: 12px; }
  .kv td.del { width: 30px; opacity: 1; }
  .kv button.del { opacity: .6; width: 22px; height: 25px; display: inline-flex; align-items: center; justify-content: center; }
  .kv button.del:hover { opacity: 1; }
  .sr-only { position: absolute; width: 1px; height: 1px; overflow: hidden; clip-path: inset(50%); }
  .bulk-link { color: var(--accent); margin-left: auto; font-size: 12px; padding: 5px 0; }
  .bulk-link:hover { color: var(--accent-strong); }
  .bulk-editor { display: flex; flex-direction: column; gap: 8px; font-size: 12px; }
  .bulk-editor label { color: var(--text-muted); }
  .bulk-editor textarea { width: 100%; min-height: 180px; resize: vertical; font-family: var(--mono); background: var(--bg); border: 1px solid var(--border-strong); border-radius: var(--radius); padding: 10px; }
  .bulk-actions { display: flex; justify-content: flex-end; gap: 8px; }
  .bulk-error { color: var(--err); }

  .suggestions {
    position: fixed; z-index: 100; overflow-y: auto; padding: 5px;
    border: 1px solid var(--border-strong); border-radius: 9px;
    background: var(--bg-raised); box-shadow: 0 12px 32px rgba(0, 0, 0, 0.28);
  }
  .suggestions-title {
    display: flex; justify-content: space-between; gap: 12px; padding: 6px 8px 7px;
    color: var(--text-muted); font-size: 9px; font-weight: 700; letter-spacing: .06em;
  }
  .suggestions-title span { font-size: 10px; font-weight: 400; letter-spacing: 0; }
  .suggestions button {
    display: flex; align-items: center; gap: 9px; width: 100%; padding: 7px 8px;
    border-radius: 5px; text-align: left; font: 12px var(--mono); color: var(--text);
  }
  .suggestions button:hover, .suggestions button.active { background: var(--accent-soft); color: var(--accent-strong); }
  .suggestion-icon { display: grid; place-items: center; width: 20px; height: 20px; border-radius: 5px; background: var(--bg-hover); color: var(--text-muted); font: 600 11px var(--font); }
  .kv .toggle { width: 28px; text-align: center; }
  .kv .toggle input { width: 14px; height: 14px; padding: 0; accent-color: var(--accent); }
  .disabled-row { background: color-mix(in srgb, var(--bg-panel) 65%, transparent); }
  .disabled-row td:not(.toggle):not(.del) { opacity: .42; }
  .disabled-row .k { text-decoration: line-through; text-decoration-color: var(--text-faint); }
</style>
