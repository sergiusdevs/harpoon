<script lang="ts">
  import type { MultipartPart } from "../../domain/types";
  import type { VariableSuggestion } from "../../domain/variables";
  import { api, isMock } from "../../services/api";
  import VariableInput from "../common/VariableInput.svelte";
  interface Props { parts: MultipartPart[]; variables: VariableSuggestion[]; onchange: (parts: MultipartPart[]) => void }
  let { parts, variables, onchange }: Props = $props();
  function update(index: number, part: MultipartPart) { onchange(parts.map((value, i) => i === index ? part : value)); }
  function add(type: "text" | "file") { onchange([...parts, type === "text" ? { type, name: "", value: "" } : { type, name: "", path: "" }]); }
  async function browse(index: number) {
    const selected = await (await api).pickFile();
    const current = parts[index];
    if (selected && current?.type === "file") update(index, { ...current, path: selected });
  }
</script>
<div class="multipart">
  <p class="muted hint">Añade campos y archivos. Las rutas relativas se resuelven desde la colección.</p>
  {#each parts as part, i (i)}
    <div class="part">
      <div class="row">
        <input class="name mono" value={part.name} placeholder="Nombre del campo" aria-label={`Nombre del campo ${i + 1}`} oninput={(e) => update(i, { ...part, name: e.currentTarget.value })} />
        <select aria-label={`Tipo del campo ${i + 1}`} value={part.type} onchange={(e) => update(i, e.currentTarget.value === "file" ? { type: "file", name: part.name, path: "" } : { type: "text", name: part.name, value: "" })}>
          <option value="text">Texto</option><option value="file">Archivo</option>
        </select>
        <button class="btn icon remove" title="Quitar campo" aria-label={`Quitar campo ${i + 1}`} onclick={() => onchange(parts.filter((_, index) => index !== i))}>✕</button>
      </div>
      {#if part.type === "text"}
        <VariableInput value={part.value} {variables} label={`Valor del campo ${i + 1}`} placeholder="Valor" oninput={(e) => update(i, { ...part, value: e.currentTarget.value })} />
      {:else}
        <div class="row"><div class="path"><VariableInput value={part.path} {variables} label={`Ruta del archivo ${i + 1}`} placeholder="archivos/ejemplo.pdf" oninput={(e) => update(i, { ...part, path: e.currentTarget.value })} /></div><button class="btn small" disabled={isMock} title={isMock ? "Disponible en la aplicación de escritorio" : "Seleccionar archivo"} onclick={() => browse(i)}>Elegir…</button></div>
        <input class="mono mime" value={part.content_type ?? ""} placeholder="Tipo de contenido (opcional)" aria-label={`Tipo de contenido del archivo ${i + 1}`} oninput={(e) => update(i, { ...part, content_type: e.currentTarget.value || undefined })} />
      {/if}
    </div>
  {/each}
  <div class="actions"><button class="btn small" onclick={() => add("text")}>+ Campo de texto</button><button class="btn small" onclick={() => add("file")}>+ Archivo</button></div>
</div>
<style>
  .multipart { display: flex; flex-direction: column; gap: 10px; }
  .hint { margin: 0; font-size: 11.5px; }
  .part { display: flex; flex-direction: column; gap: 7px; padding: 10px; border: 1px solid var(--border); border-radius: var(--radius); }
  .row, .actions { display: flex; align-items: center; gap: 7px; }
  .actions { flex-wrap: wrap; } .name, .path { flex: 1; min-width: 0; }
  .name, .mime { font-size: 12px; } .mime { width: 100%; }
  .remove { flex: 0 0 auto; } select { font-size: 12px; }
</style>
