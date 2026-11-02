<script lang="ts">
  // Editor de pares clave/valor con fila vacía al final para añadir.
  interface Props {
    entries: Record<string, string>;
    onchange: (entries: Record<string, string>) => void;
    keyPlaceholder?: string;
    valuePlaceholder?: string;
  }
  let { entries, onchange, keyPlaceholder = "clave", valuePlaceholder = "valor" }: Props = $props();

  let rows = $state<{ k: string; v: string }[]>([]);

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
  }
</script>

<table class="kv">
  <tbody>
    {#each rows as row, i (i)}
      <tr>
        <td style="width: 38%">
          <input value={row.k} placeholder={keyPlaceholder} oninput={(e) => edit(i, "k", e.currentTarget.value)} spellcheck="false" />
        </td>
        <td>
          <input value={row.v} placeholder={valuePlaceholder} oninput={(e) => edit(i, "v", e.currentTarget.value)} spellcheck="false" />
        </td>
        <td class="del">
          {#if i < rows.length - 1}
            <button class="del" title="Quitar" onclick={() => remove(i)}>✕</button>
          {/if}
        </td>
      </tr>
    {/each}
  </tbody>
</table>
