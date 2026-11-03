<script lang="ts">
  // Elegir el color de una carpeta (se guarda en su folder.toml).
  import { store } from "../../state/store.svelte";
  import { FOLDER_COLORS, folderColor } from "../../domain/folderColors";
  import Modal from "../common/Modal.svelte";
  import Icon from "../common/Icon.svelte";

  const folder = store.editingFolder;
  const current = store.collection?.folder_configs[folder]?.color ?? null;
  let custom = $state(current?.startsWith("#") ? current : "#4a8fe7");
  const kind = $derived(store.themeKind);

  async function choose(color: string | null) {
    await store.setFolderColor(folder, color);
    store.modal = null;
  }
</script>

<Modal title={`Color · ${folder}`} width={420} onclose={() => (store.modal = null)}>
  <p class="muted hint">Se guarda en <code>{folder}/folder.toml</code>, así lo ve todo el equipo.</p>
  <div class="swatches" role="radiogroup" aria-label="Color de la carpeta">
    {#each FOLDER_COLORS as c (c.id)}
      <button role="radio" aria-checked={current === c.id} class:on={current === c.id} title={c.name} aria-label={c.name} onclick={() => choose(c.id)} style:--swatch={folderColor(c.id, kind)}>
        <Icon name="folder" size={20} />
      </button>
    {/each}
  </div>
  <div class="row">
    <label class="custom">
      <input type="color" bind:value={custom} aria-label="Color personalizado" />
      <span>Personalizado <span class="mono muted">{custom}</span></span>
    </label>
    <button class="btn small" onclick={() => choose(custom)}>Usar</button>
  </div>
  {#snippet footer()}
    <button class="btn" disabled={!current} onclick={() => choose(null)}>Quitar color</button>
    <button class="btn" onclick={() => (store.modal = null)}>Cerrar</button>
  {/snippet}
</Modal>

<style>
  .hint { margin: 0 0 14px; font-size: 12px; }
  code { font: 11px var(--mono); background: var(--bg-active); padding: 1px 5px; border-radius: 4px; }
  .swatches { display: grid; grid-template-columns: repeat(9, 1fr); gap: 6px; }
  .swatches button { display: grid; place-items: center; height: 38px; border: 1px solid var(--border); border-radius: 8px; color: var(--swatch); background: color-mix(in srgb, var(--swatch) 14%, transparent); transition: transform var(--t), border-color var(--t); }
  .swatches button:hover { transform: translateY(-2px); border-color: var(--swatch); }
  .swatches button.on { border-color: var(--swatch); box-shadow: 0 0 0 2px color-mix(in srgb, var(--swatch) 45%, transparent); }
  .row { display: flex; align-items: center; justify-content: space-between; gap: 10px; margin-top: 14px; padding-top: 12px; border-top: 1px solid var(--border); }
  .custom { display: flex; align-items: center; gap: 10px; font-size: 12px; }
  .custom input { width: 34px; height: 28px; padding: 2px; border-radius: 6px; cursor: pointer; }
</style>
