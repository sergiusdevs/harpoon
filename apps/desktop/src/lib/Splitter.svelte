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
    };
    target.addEventListener("pointermove", move);
    target.addEventListener("pointerup", up);
  }
</script>

<div class="splitter {direction}" class:dragging onpointerdown={start} role="separator" aria-orientation={direction}></div>

<style>
  .splitter {
    flex: 0 0 auto;
    background: var(--border);
    position: relative;
    z-index: 2;
    transition: background var(--t);
  }
  .splitter.horizontal {
    width: 1px;
    cursor: col-resize;
  }
  .splitter.vertical {
    height: 1px;
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
    top: -3px;
    bottom: -3px;
  }
  .splitter:hover,
  .splitter.dragging {
    background: var(--accent);
  }
</style>
