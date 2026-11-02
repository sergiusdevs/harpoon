<script lang="ts">
  // Editor de pares clave/valor: fila vacía al final, botón "Añadir",
  // borrado accesible por teclado y pegado en bloque ("Clave: valor" por línea).
  interface Props {
    entries: Record<string, string>;
    onchange: (entries: Record<string, string>) => void;
    keyPlaceholder?: string;
    valuePlaceholder?: string;
    /** Texto del botón de añadir (p. ej. "Añadir cabecera"). */
    addLabel?: string;
  }
  let { entries, onchange, keyPlaceholder = "clave", valuePlaceholder = "valor", addLabel = "Añadir" }: Props = $props();

  let rows = $state<{ k: string; v: string }[]>([]);
  let table = $state<HTMLTableElement | null>(null);

  $effect(() => {
    const next = Object.entries(entries).map(([k, v]) => ({ k, v }));
    rows = [...next, { k: "", v: "" }];
  });

  function commit() {
    const out: Record<string, string> = {};
    for (const r of rows) if (r.k.trim()) out[r.k.trim()] = r.v;
    onchange(out);
  }

  function edit(i: number, field: "k" | "v", val: string) {
    rows[i][field] = val;
    if (i === rows.length - 1 && (rows[i].k || rows[i].v)) rows.push({ k: "", v: "" });
    commit();
  }

  function remove(i: number) {
    rows.splice(i, 1);
    if (rows.length === 0 || rows[rows.length - 1].k) rows.push({ k: "", v: "" });
    commit();
    focusKey(Math.min(i, rows.length - 1));
  }

  function add() {
    if (rows[rows.length - 1].k) rows.push({ k: "", v: "" });
    focusKey(rows.length - 1);
  }

  function focusKey(i: number) {
    queueMicrotask(() => {
      (table?.querySelectorAll<HTMLInputElement>("input.k")[i])?.focus();
    });
  }

  /** Pegar varias líneas "Clave: valor" (o "clave=valor") a la vez. */
  function onPaste(e: ClipboardEvent, i: number) {
    const text = e.clipboardData?.getData("text") ?? "";
    const lines = text.split(/\r?\n/).map((l) => l.trim()).filter(Boolean);
    if (lines.length < 2 && !/[:=]/.test(text)) return;
    const parsed = lines
      .map((l) => {
        const m = l.match(/^([^:=]+?)\s*[:=]\s*(.*)$/);
        return m ? { k: m[1].trim(), v: m[2].trim() } : null;
      })
      .filter((x): x is { k: string; v: string } => !!x);
    if (!parsed.length) return;
    e.preventDefault();
    rows.splice(i, rows[i].k ? 0 : 1, ...parsed);
    if (rows[rows.length - 1].k) rows.push({ k: "", v: "" });
    commit();
  }

  function onKey(e: KeyboardEvent, i: number, field: "k" | "v") {
    if (e.key === "Enter") {
      e.preventDefault();
      if (field === "k") (e.currentTarget as HTMLElement).closest("tr")?.querySelector<HTMLInputElement>("input.v")?.focus();
      else if (i === rows.length - 1) add();
      else focusKey(i + 1);
    } else if (e.key === "Backspace" && (e.metaKey || e.ctrlKey) && i < rows.length - 1) {
      e.preventDefault();
      remove(i);
    }
  }
</script>

<table class="kv" bind:this={table}>
  <tbody>
    {#each rows as row, i (i)}
      <tr>
        <td style="width: 38%">
          <input class="k" value={row.k} placeholder={keyPlaceholder} aria-label="{keyPlaceholder} {i + 1}" oninput={(e) => edit(i, "k", e.currentTarget.value)} onpaste={(e) => onPaste(e, i)} onkeydown={(e) => onKey(e, i, "k")} spellcheck="false" />
        </td>
        <td>
          <input class="v" value={row.v} placeholder={valuePlaceholder} aria-label="{valuePlaceholder} {i + 1}" oninput={(e) => edit(i, "v", e.currentTarget.value)} onkeydown={(e) => onKey(e, i, "v")} spellcheck="false" />
        </td>
        <td class="del">
          {#if i < rows.length - 1}
            <button class="del" title="Quitar (⌘⌫)" aria-label="Quitar fila {i + 1}" onclick={() => remove(i)}>✕</button>
          {/if}
        </td>
      </tr>
    {/each}
  </tbody>
</table>
<div class="kv-actions">
  <button class="btn small" onclick={add}>+ {addLabel}</button>
  <span class="muted hint">Enter salta de campo · pega varias líneas "Clave: valor" de golpe</span>
</div>

<style>
  .hint {
    font-size: 11px;
    align-self: center;
  }
</style>
