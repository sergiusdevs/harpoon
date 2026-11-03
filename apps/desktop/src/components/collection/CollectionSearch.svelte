<script lang="ts">
  import { onMount } from "svelte";
  import { store } from "../../state/store.svelte";
  import type { Request } from "../../domain/types";
  import Modal from "../common/Modal.svelte";
  import MethodBadge from "../common/MethodBadge.svelte";

  type Match = { field: string; before: string; found: string; after: string; hidden: boolean };
  type Result = { key: string; request: Request; matches: Match[]; unsaved: boolean };
  let query = $state("");
  let input = $state<HTMLInputElement>();
  onMount(() => input?.focus());

  function searchRequest(key: string, request: Request, needle: string): Match[] {
    const matches: Match[] = [];
    function search(field: string, value: string, hidden = false) {
      const index = value.toLowerCase().indexOf(needle);
      if (index < 0) return;
      if (hidden) { matches.push({ field, before: "Valor oculto", found: "", after: "", hidden }); return; }
      const left = Math.max(0, index - 45);
      const right = Math.min(value.length, index + needle.length + 75);
      const clean = (s: string) => s.replace(/\s+/g, " ");
      matches.push({ field, before: (left ? "…" : "") + clean(value.slice(left, index)), found: clean(value.slice(index, index + needle.length)), after: clean(value.slice(index + needle.length, right)) + (right < value.length ? "…" : ""), hidden });
    }
    search("Ruta", key); search("Nombre", request.name); search("URL", request.url);
    for (const [name, value] of Object.entries(request.headers)) {
      search("Nombre de cabecera", name);
      search(`Cabecera · ${name}`, value, /authorization|cookie|token|secret|api.?key/i.test(name));
    }
    for (const [name, value] of Object.entries(request.query)) { search("Nombre de parámetro", name); search(`Parámetro · ${name}`, value); }
    switch (request.body.type) {
      case "json": search("Cuerpo JSON", request.body.json); break;
      case "graphql": search("Consulta GraphQL", request.body.query); search("Variables GraphQL", request.body.variables); search("Operación GraphQL", request.body.operation_name); break;
      case "text": search("Cuerpo", request.body.text); break;
      case "xml": search("Cuerpo XML", request.body.xml); break;
      case "binary": search("Ruta del archivo", request.body.path); break;
      case "form": for (const [name, value] of Object.entries(request.body.fields)) { search("Campo de formulario", name); search(`Formulario · ${name}`, value); } break;
      case "multipart": for (const part of request.body.parts) { search("Campo multipart", part.name); search(`Multipart · ${part.name}`, part.type === "file" ? part.path : part.value); } break;
    }
    search("Script previo", request.scripts.pre); search("Script posterior", request.scripts.post); search("Tests", request.scripts.test); search("Documentación", request.docs);
    return matches;
  }
  const results = $derived.by(() => {
    const needle = query.trim().toLowerCase();
    if (!needle || !store.collection) return [];
    const out: Result[] = [];
    for (const [key, saved] of Object.entries(store.collection.requests)) {
      const unsaved = key === store.selected && store.dirty && !!store.draft;
      const request = unsaved ? store.draft! : saved;
      const matches = searchRequest(key, request, needle);
      if (matches.length) out.push({ key, request, matches, unsaved });
    }
    return out;
  });
  function open(key: string) { store.modal = null; store.select(key); }
</script>
<Modal title="Buscar en la colección" width={780} onclose={() => (store.modal = null)}>
  <div class="search"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true"><circle cx="11" cy="11" r="7"/><path d="m16 16 4 4"/></svg><input bind:this={input} bind:value={query} placeholder="Buscar en solicitudes…" aria-label="Buscar en solicitudes" spellcheck="false" onkeydown={(event) => { if (event.key === "Enter" && results.length) { event.preventDefault(); open(results[0].key); } }} />{#if query}<button class="clear" aria-label="Limpiar búsqueda" onclick={() => { query = ""; input?.focus(); }}>✕</button>{/if}</div>
  <p class="muted hint">Busca en nombres, rutas, URL, parámetros, cabeceras, cuerpos, scripts y documentación. Incluye el borrador actual.</p>
  {#if query.trim()}
    <p class="muted count" aria-live="polite">{results.length} {results.length === 1 ? "solicitud encontrada" : "solicitudes encontradas"}{results.length > 300 ? " · se muestran las primeras 300" : ""}</p>
    {#if !results.length}<p class="muted empty">No hay coincidencias.</p>{/if}
    <div class="results">
      {#each results.slice(0, 300) as result (result.key)}
        <button class="result" onclick={() => open(result.key)}>
          <span class="heading"><MethodBadge method={result.request.method} /><strong>{result.request.name}</strong>{#if result.unsaved}<span class="unsaved">Sin guardar</span>{/if}<span class="muted path">{result.key}</span></span>
          {#each result.matches.slice(0, 8) as match}<span class="match"><span class="field">{match.field}</span><span class="excerpt mono">{match.before}{#if !match.hidden}<mark>{match.found}</mark>{/if}{match.after}</span></span>{/each}
          {#if result.matches.length > 8}<span class="muted more">+{result.matches.length - 8} campos con coincidencias</span>{/if}
        </button>
      {/each}
    </div>
  {/if}
  {#snippet footer()}<button class="btn" onclick={() => (store.modal = null)}>Cerrar</button>{/snippet}
</Modal>
<style>
  .search { display: flex; align-items: center; gap: 8px; color: var(--text-muted); border: 1px solid var(--border-strong); border-radius: 7px; background: var(--bg-raised); padding: 0 10px; }
  .search:focus-within { border-color: var(--accent); } .search input { flex: 1; min-width: 0; border: 0; background: transparent; outline: 0; padding: 10px 0; } .clear { color: var(--text-muted); }
  .hint, .count, .empty { font-size: 12px; line-height: 1.5; margin: 10px 0; } .results { max-height: 430px; overflow: auto; }
  .result { display: block; width: 100%; text-align: left; padding: 10px; border-radius: 6px; } .result:hover, .result:focus-visible { background: var(--bg-hover); }
  .heading { display: flex; align-items: center; gap: 8px; min-width: 0; font-size: 12px; } .heading strong { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; } .path { margin-left: auto; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 11px; } .unsaved { font-size: 10px; color: var(--warn); white-space: nowrap; }
  .match { display: flex; gap: 10px; padding-top: 6px; font-size: 11px; min-width: 0; } .field { flex: 0 0 135px; color: var(--text-muted); overflow: hidden; white-space: nowrap; text-overflow: ellipsis; } .excerpt { overflow: hidden; white-space: nowrap; text-overflow: ellipsis; } mark { color: var(--text); background: color-mix(in srgb, var(--accent) 35%, transparent); border-radius: 2px; } .more { display: block; padding: 6px 0 0 145px; font-size: 11px; }
</style>
