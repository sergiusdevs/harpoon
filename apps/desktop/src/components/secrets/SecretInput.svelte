<script lang="ts">
  // Campo de texto enmascarado con botón de ojo para ver el valor un momento.
  import Icon from "../common/Icon.svelte";
  interface Props {
    value: string;
    oninput: (value: string) => void;
    label: string;
    placeholder?: string;
    inputClass?: string;
    disabled?: boolean;
    /** Si es false se comporta como un input normal (sin máscara ni ojo). */
    masked?: boolean;
    onkeydown?: (event: KeyboardEvent & { currentTarget: HTMLInputElement }) => void;
  }
  let { value, oninput, label, placeholder = "", inputClass = "", disabled = false, masked = true, onkeydown }: Props = $props();
  let show = $state(false);
</script>

<span class="secret-input">
  <input class={inputClass} type={masked && !show ? "password" : "text"} {value} {placeholder} {disabled} aria-label={label}
    autocomplete="off" spellcheck="false" oninput={(e) => oninput(e.currentTarget.value)} {onkeydown} />
  {#if masked}
    <button type="button" class="eye" aria-label={`${show ? "Ocultar" : "Mostrar"} ${label}`} aria-pressed={show} title={show ? "Ocultar" : "Mostrar"} onclick={() => (show = !show)} {disabled}>
      <Icon name={show ? "eye-off" : "eye"} size={14} />
    </button>
  {/if}
</span>

<style>
  .secret-input { display: flex; align-items: center; min-width: 0; width: 100%; gap: 2px; }
  .secret-input input { flex: 1; min-width: 0; }
  .eye { display: inline-flex; align-items: center; justify-content: center; width: 24px; height: 24px; flex: 0 0 24px; color: var(--text-muted); border-radius: 4px; }
  .eye:hover { background: var(--bg-hover); color: var(--text); }
</style>
