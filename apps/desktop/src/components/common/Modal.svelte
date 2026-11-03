<script lang="ts">
  import { modalFocus } from "../../domain/modalFocus";
  import type { Snippet } from "svelte";

  interface Props {
    title: string;
    width?: number;
    onclose: () => void;
    children: Snippet;
    footer?: Snippet;
  }
  const titleId = $props.id();
  let { title, width = 720, onclose, children, footer }: Props = $props();
</script>


<div class="backdrop" role="presentation" onclick={onclose}>
  <div class="modal fade-in" use:modalFocus={{ onEscape: onclose }} role="dialog" aria-modal="true" aria-labelledby={titleId} tabindex="-1" onkeydown={() => {}} style="width: {width}px" onclick={(e) => e.stopPropagation()}>
    <header>
      <h3 id={titleId}>{title}</h3>
      <button class="btn icon" aria-label="Cerrar diálogo" onclick={onclose} title="Cerrar (Esc)">✕</button>
    </header>
    <div class="body">{@render children()}</div>
    {#if footer}<footer>{@render footer()}</footer>{/if}
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.45);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 45;
  }
  .modal {
    max-width: calc(100vw - 40px);
    max-height: calc(100vh - 60px);
    background: var(--bg-panel);
    border: 1px solid var(--border-strong);
    border-radius: 10px;
    box-shadow: 0 20px 50px rgba(0, 0, 0, 0.4);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border);
  }
  h3 {
    margin: 0;
    font-size: 14px;
  }
  .body {
    overflow: auto;
    padding: 14px 16px;
    min-height: 0;
  }
  footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    flex-wrap: wrap;
    padding: 10px 16px;
    border-top: 1px solid var(--border);
  }
</style>
