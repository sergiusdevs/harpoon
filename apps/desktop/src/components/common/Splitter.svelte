<script lang="ts">
  // Separador arrastrable entre dos paneles (horizontal o vertical).
  interface Props {
    direction?: "horizontal" | "vertical";
    onresize: (delta: number) => void;
  }
  let { direction = "horizontal", onresize }: Props = $props();
  let dragging = $state(false);

  function start(e: PointerEvent) {
    dragging = true;
    let last = direction === "horizontal" ? e.clientX : e.clientY;
    const target = e.currentTarget as HTMLElement;
    target.setPointerCapture(e.pointerId);
    const move = (ev: PointerEvent) => {
      const now = direction === "horizontal" ? ev.clientX : ev.clientY;
      onresize(now - last);
      last = now;
    };
    const up = () => {
      dragging = false;
      target.removeEventListener("pointermove", move);
      target.removeEventListener("pointerup", up);
      target.removeEventListener("pointercancel", up);
    };
    target.addEventListener("pointermove", move);
    target.addEventListener("pointerup", up);
    target.addEventListener("pointercancel", up);
  }

  function onKeydown(e: KeyboardEvent) {
    const backward = direction === "horizontal" ? "ArrowLeft" : "ArrowUp";
    const forward = direction === "horizontal" ? "ArrowRight" : "ArrowDown";
    if (e.key === backward || e.key === forward) {
      e.preventDefault();
      onresize((e.key === backward ? -1 : 1) * (e.shiftKey ? 40 : 10));
    }
  }
</script>

<!-- Un separador ajustable es enfocable y se maneja con las flechas. -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div class="splitter {direction}" class:dragging onpointerdown={start} onkeydown={onKeydown} role="separator" tabindex="0" aria-label="Ajustar tamaño de los paneles" aria-orientation={direction === "horizontal" ? "vertical" : "horizontal"}></div>

<style>
  .splitter {
    flex: 0 0 auto;
    background: var(--border);
    position: relative;
    z-index: 2;
    transition: background var(--t);
    touch-action: none;
  }
  .splitter.horizontal {
    width: 1px;
    cursor: col-resize;
  }
  .splitter.vertical {
    height: 5px;
    background: var(--bg);
    cursor: row-resize;
  }
  .splitter::after {
    content: "";
    position: absolute;
    inset: 0;
  }
  .splitter.horizontal::after {
    left: -3px;
    right: -3px;
  }
  .splitter.vertical::after {
    width: 32px; height: 2px; inset: 1px auto auto 50%;
    transform: translateX(-50%); border-radius: 2px; background: var(--border-strong);
  }
  .splitter:hover,
  .splitter.dragging {
    background: var(--accent);
  }
</style>
