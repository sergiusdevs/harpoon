<script lang="ts">
  import { store } from "../../state/store.svelte";
  import { themes } from "../../domain/themes";
  import { interfaceFonts, editorFonts, type InterfaceFont, type EditorFont } from "../../domain/fonts";
  let tab = $state<"themes" | "fonts">("themes");
  import Modal from "../common/Modal.svelte";
  import { tick } from "svelte";

  function navigate(event: KeyboardEvent, index: number) {
    const step = event.key === "ArrowRight" || event.key === "ArrowDown" ? 1 : event.key === "ArrowLeft" || event.key === "ArrowUp" ? -1 : 0;
    if (!step) return;
    event.preventDefault();
    const next = (index + step + themes.length) % themes.length;
    store.setTheme(themes[next].id);
    const group = (event.currentTarget as HTMLElement).parentElement;
    void tick().then(() => group?.querySelectorAll<HTMLButtonElement>('[role="radio"]')[next]?.focus());
  }
</script>

<Modal title="Configuración" width={660} onclose={() => (store.modal = null)}>
  <div class="settings-tabs" aria-label="Secciones de configuración">
    <button class:active={tab === "themes"} aria-pressed={tab === "themes"} onclick={() => (tab = "themes")}>Temas</button>
    <button class:active={tab === "fonts"} aria-pressed={tab === "fonts"} onclick={() => (tab = "fonts")}>Fuentes</button>
  </div>
  <p class="intro muted">Los cambios se aplican al instante y se guardan en este dispositivo.</p>
  {#if tab === "themes"}
  <div class="themes" role="radiogroup" aria-label="Tema de la aplicación">
    {#each themes as theme, index}
      <button type="button" role="radio" aria-checked={store.theme === theme.id} tabindex={store.theme === theme.id ? 0 : -1} class:chosen={store.theme === theme.id} onclick={() => store.setTheme(theme.id)} onkeydown={(event) => navigate(event, index)}>
        <span class="preview" style:background={theme.bg} style:color={theme.text}>
          <span class="preview-sidebar" style:background={theme.panel}></span>
          <span class="preview-content"><i style:background={theme.accent}></i><i></i><i></i></span>
        </span>
        <span class="name">{theme.name} {#if store.theme === theme.id}<span class="check" aria-hidden="true">✓</span>{/if}</span>
        <span class="detail">{theme.detail}</span>
      </button>
    {/each}
  </div>
  {:else}
    <div class="fonts">
      <label><span>Fuente de la interfaz</span><select value={store.interfaceFont} onchange={(e) => store.setInterfaceFont(e.currentTarget.value as InterfaceFont)}>{#each interfaceFonts as font}<option value={font.id}>{font.name}</option>{/each}</select></label>
      <label><span>Fuente del editor</span><select value={store.editorFont} onchange={(e) => store.setEditorFont(e.currentTarget.value as EditorFont)}>{#each editorFonts as font}<option value={font.id}>{font.name}</option>{/each}</select></label>
      <label><span>Tamaño del editor</span><span class="size"><input type="number" min="10" max="20" step="0.5" value={store.editorFontSize} onchange={(e) => { store.setEditorFontSize(e.currentTarget.valueAsNumber); e.currentTarget.value = String(store.editorFontSize); }} /> px</span></label>
      <div class="sample"><span class="muted">Vista previa del editor</span><code><span>{"{ "}</span><b>"status"</b><span>: </span><em>200</em><span>, </span><b>"message"</b><span>: </span><i>"Hello, Harpoon!"</i><span>{" }"}</span></code></div>
      <p class="hint muted">Las fuentes por nombre se usan si están instaladas; si no, se utiliza una alternativa del sistema.</p>
    </div>
  {/if}
  {#snippet footer()}<button class="btn" onclick={() => (store.modal = null)}>Cerrar</button>{/snippet}
</Modal>

<style>
  .settings-tabs { display: flex; gap: 4px; border-bottom: 1px solid var(--border); margin: -6px 0 16px; }
  .settings-tabs button { padding: 8px 12px; color: var(--text-muted); border-bottom: 2px solid transparent; }
  .settings-tabs button.active { color: var(--text); border-bottom-color: var(--accent); }
  .fonts { display: flex; flex-direction: column; gap: 14px; }
  .fonts label { display: grid; grid-template-columns: 190px minmax(0, 260px); gap: 12px; align-items: center; font-size: 12px; }
  .fonts label > span:first-child { color: var(--text-muted); }
  .fonts select { width: 100%; }
  .size { display: flex; align-items: center; gap: 7px; color: var(--text-muted); }
  .size input { width: 78px; }
  .sample { display: flex; flex-direction: column; gap: 10px; padding: 14px; border: 1px solid var(--border); border-radius: var(--radius); background: var(--bg); font-size: 11px; }
  .sample code { font: var(--editor-font-size)/1.5 var(--mono); color: var(--text); user-select: text; }
  .sample code b { color: var(--syntax-property); font-weight: 400; }
  .sample code em { color: var(--syntax-number); font-style: normal; }
  .sample code i { color: var(--syntax-string); font-style: normal; }
  .hint { margin: 0; font-size: 11px; }
  .intro { margin: 0 0 16px; font-size: 12px; }
  .themes { display: grid; grid-template-columns: repeat(auto-fit, minmax(155px, 1fr)); gap: 10px; }
  .themes button { display: flex; flex-direction: column; gap: 4px; min-width: 0; padding: 8px; text-align: left; border: 1px solid var(--border); border-radius: 8px; background: var(--bg); }
  .themes button:hover { border-color: var(--border-strong); }
  .themes button.chosen { border-color: var(--accent); box-shadow: 0 0 0 1px var(--accent); }
  .preview { width: 100%; height: 62px; display: flex; overflow: hidden; border: 1px solid var(--border); border-radius: 5px; }
  .preview-sidebar { width: 25%; }
  .preview-content { flex: 1; display: flex; flex-direction: column; align-items: flex-start; gap: 6px; padding: 12px; }
  .preview-content i { width: 70%; height: 4px; border-radius: 4px; background: currentColor; opacity: .35; }
  .preview-content i:first-child { opacity: 1; width: 45%; }
  .preview-content i:last-child { width: 52%; }
  .name { display: flex; justify-content: space-between; width: 100%; margin-top: 3px; font-size: 12px; font-weight: 600; }
  .check { color: var(--accent); }
  .detail { color: var(--text-muted); font-size: 11px; }
</style>
