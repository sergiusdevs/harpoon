type Options = { onEscape: () => void };
type Layer = { node: HTMLElement; previous: Element | null };
const layers: Layer[] = [];
const originalInert = new Map<HTMLElement, boolean>();
const selector = 'button:not(:disabled), a[href], input:not(:disabled), select:not(:disabled), textarea:not(:disabled), [tabindex]:not([tabindex="-1"]), [contenteditable="true"]';
const focusable = (node: HTMLElement) => [...node.querySelectorAll<HTMLElement>(selector)].filter((element) => element.getClientRects().length && !element.closest('[inert], [aria-hidden="true"]'));
function syncInert() {
  const top = layers.at(-1);
  for (const child of [...(document.getElementById("app")?.children ?? [])]) {
    if (!(child instanceof HTMLElement)) continue;
    if (top) {
      if (!originalInert.has(child)) originalInert.set(child, child.inert);
      child.inert = !child.contains(top.node);
    }
  }
  if (!top) { for (const [node, inert] of originalInert) node.inert = inert; originalInert.clear(); }
}

/** Mantiene foco, Escape y fondo inerte en el diálogo que está delante. */
export function modalFocus(node: HTMLElement, initial: Options) {
  let options = initial;
  const layer = { node, previous: document.activeElement };
  layers.push(layer); syncInert();
  queueMicrotask(() => { if (layers.at(-1) === layer) (focusable(node)[0] ?? node).focus(); });
  const keydown = (event: KeyboardEvent) => {
    if (layers.at(-1) !== layer) return;
    if (event.key === "Escape") { event.preventDefault(); event.stopImmediatePropagation(); options.onEscape(); }
    if (event.key !== "Tab") return;
    const elements = focusable(node), first = elements[0], last = elements.at(-1);
    if (!first) { event.preventDefault(); node.focus(); return; }
    if (event.shiftKey && (document.activeElement === first || !node.contains(document.activeElement))) { event.preventDefault(); last!.focus(); }
    else if (!event.shiftKey && (document.activeElement === last || !node.contains(document.activeElement))) { event.preventDefault(); first.focus(); }
  };
  document.addEventListener("keydown", keydown, true);
  return {
    update(next: Options) { options = next; },
    destroy() {
      document.removeEventListener("keydown", keydown, true);
      const index = layers.indexOf(layer); if (index >= 0) layers.splice(index, 1);
      syncInert();
      const top = layers.at(-1);
      if (layer.previous instanceof HTMLElement && layer.previous.isConnected && !layer.previous.closest("[inert]") && (!top || top.node.contains(layer.previous))) layer.previous.focus();
      else if (top) (focusable(top.node)[0] ?? top.node).focus();
    },
  };
}
