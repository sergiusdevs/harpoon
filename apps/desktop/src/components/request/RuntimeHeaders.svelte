<script lang="ts">
  // Sección plegable "Heredadas y del runtime" de la pestaña Headers.
  import type { RuntimeHeader } from "../../domain/runtimeHeaders";

  interface Props {
    headers: RuntimeHeader[];
    disabled: string[];
    ontoggle: (name: string, enabled: boolean) => void;
  }
  let { headers, disabled, ontoggle }: Props = $props();

  const KEY = "harpoon.runtimeHeadersOpen";
  let open = $state(true);
  try { open = localStorage.getItem(KEY) !== "0"; } catch { /* sin almacenamiento */ }
  function toggleOpen() {
    open = !open;
    try { localStorage.setItem(KEY, open ? "1" : "0"); } catch { /* sin almacenamiento */ }
  }
  const isOff = (name: string) => disabled.some((d) => d.toLowerCase() === name.toLowerCase());
</script>

<div class="runtime" class:open>
  <button class="heading" aria-expanded={open} onclick={toggleOpen}>
    <svg class="chevron" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="m6 9 6 6 6-6" /></svg>
    Heredadas y del runtime <span class="count">{headers.filter((h) => !h.overridden && !isOff(h.name)).length}</span>
  </button>
  {#if open}
    <table class="rows fade-in">
      <tbody>
        {#each headers as h (h.name)}
          {@const off = h.toggleable && isOff(h.name)}
          <tr class:off class:overridden={h.overridden} title={h.overridden ? "La sustituye una cabecera de la solicitud" : h.toggleable ? h.source : `${h.source} · no se puede desactivar; añade una cabecera con el mismo nombre para cambiar su valor`}>
            <td class="toggle">
              <input type="checkbox" checked={!off && !h.overridden} disabled={!h.toggleable || h.overridden}
                aria-label={`${off ? "Activar" : "Desactivar"} ${h.name}`} onchange={(e) => ontoggle(h.name, e.currentTarget.checked)} />
            </td>
            <td class="name">{h.name}</td>
            <td class="value mono">{h.value}</td>
            <td class="source">{h.overridden ? "sustituida" : h.source}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</div>

<style>
  .runtime { margin-bottom: 14px; border: 1px solid var(--border-strong); border-radius: 7px; overflow: hidden; }
  .heading { display: flex; align-items: center; gap: 8px; width: 100%; padding: 9px 12px; font-size: 12.5px; font-weight: 600; text-align: left; background: var(--bg-panel); transition: background var(--t); }
  .heading:hover { background: var(--bg-hover); }
  .open .heading { border-bottom: 1px solid var(--border-strong); }
  .chevron { color: var(--text-muted); transform: rotate(-90deg); transition: transform 200ms cubic-bezier(.2, .7, .3, 1); }
  .open .chevron { transform: none; }
  .count { font-size: 10px; font-weight: 600; padding: 1px 6px; border-radius: 10px; background: var(--bg-active); color: var(--text-muted); }
  .rows { width: 100%; border-collapse: collapse; font-size: 12px; table-layout: fixed; }
  .rows tr + tr td { border-top: 1px solid var(--border); }
  .rows td { padding: 7px 10px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .rows tr { transition: background var(--t), opacity var(--t); }
  .rows tr:hover { background: color-mix(in srgb, var(--bg-hover) 55%, transparent); }
  .toggle { width: 34px; text-align: center; }
  .toggle input { accent-color: var(--accent); margin: 0; }
  .name { width: 34%; font-style: italic; color: var(--text-muted); }
  .value { color: var(--text-muted); }
  .source { width: 26%; color: var(--text-faint); font-size: 11px; text-align: right; }
  tr.off .name, tr.off .value, tr.overridden .name, tr.overridden .value { text-decoration: line-through; opacity: .6; }
</style>
