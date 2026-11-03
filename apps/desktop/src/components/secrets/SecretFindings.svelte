<script lang="ts">
  // Al guardar: la solicitud lleva secretos escritos tal cual. Se ofrece
  // convertirlos en variables secretas antes de que lleguen a Git.
  import { store } from "../../state/store.svelte";
  import { isConvertibleLocation, isValidVarName } from "../../domain/secrets";
  import type { SecretFinding } from "../../domain/types";
  import Modal from "../common/Modal.svelte";
  import Icon from "../common/Icon.svelte";
  import { scopeOptions } from "./scope";

  type Row = { name: string; scope: string; error: string; busy: boolean };
  const options = $derived(scopeOptions());
  const findings = $derived(store.secretFindings?.findings ?? []);
  const convertible = $derived(findings.filter((f) => isConvertibleLocation(f.location)));
  const initialScope = scopeOptions()[0]?.id ?? "collection";
  let rows = $state<Record<string, Row>>(Object.fromEntries((store.secretFindings?.findings ?? []).map((f) => [f.location, { name: f.suggestion, scope: initialScope, error: "", busy: false }])));
  let busy = $state(false);
  let failed = $state<string[]>([]);

  $effect(() => {
    for (const f of findings) if (!rows[f.location]) rows[f.location] = { name: f.suggestion, scope: options[0]?.id ?? "collection", error: "", busy: false };
  });

  const canConvert = (f: SecretFinding) => isConvertibleLocation(f.location) && !failed.includes(f.location);
  const close = () => { store.modal = null; };

  async function convert(f: SecretFinding): Promise<boolean> {
    const row = rows[f.location];
    const name = row.name.trim();
    if (!isValidVarName(name)) { row.error = "Nombre no válido: usa letras, números y _."; return false; }
    const scope = options.find((o) => o.id === row.scope)?.scope ?? { kind: "collection" as const };
    row.busy = true; row.error = "";
    try {
      const ok = await store.convertFindingToSecret(f, name, scope);
      if (!ok) { failed = [...failed, f.location]; return false; }
      return true;
    } catch (cause) { row.error = cause instanceof Error ? cause.message : String(cause); return false; }
    finally { row.busy = false; }
  }
  async function finishIfDone() {
    if ((store.secretFindings?.findings.length ?? 0) === 0) { store.modal = null; await store.save({ force: true }); }
  }
  async function convertOne(f: SecretFinding) { if (await convert(f)) await finishIfDone(); }
  async function convertAll() {
    busy = true;
    try { for (const f of [...convertible]) if (canConvert(f)) await convert(f); await finishIfDone(); }
    finally { busy = false; }
  }
  async function saveAnyway() { store.modal = null; await store.save({ force: true }); }
</script>

<Modal title="¿Guardar esto en Git?" width={680} onclose={close}>
  <p class="intro">Parece que esta solicitud lleva secretos escritos tal cual. Conviértelos en variables secretas: en Git solo quedará <code>{"{{nombre}}"}</code> y el valor se guardará en tu máquina.</p>
  <ul class="list">
    {#each findings as f (f.location)}
      {@const row = rows[f.location]}
      <li class="finding">
        <div class="what">
          <span class="lock"><Icon name="lock" size={13} /></span>
          <div class="meta">
            <div class="loc">{f.location}{#if f.kind}<span class="kind">{f.kind}</span>{/if}</div>
            {#if f.preview}<code class="preview" title="Valor recortado">{f.preview}</code>{/if}
          </div>
        </div>
        {#if row && canConvert(f)}
          <div class="convert">
            <label class="name"><span class="sr-only">Nombre de la variable para {f.location}</span><span class="brace" aria-hidden="true">{"{{"}</span><input class="mono" bind:value={row.name} spellcheck="false" autocomplete="off" disabled={row.busy || busy} onkeydown={(e) => e.key === "Enter" && convertOne(f)} /><span class="brace" aria-hidden="true">{"}}"}</span></label>
            <select aria-label={`Dónde guardar ${f.location}`} bind:value={row.scope} disabled={row.busy || busy}>{#each options as o}<option value={o.id}>{o.label}</option>{/each}</select>
            <button class="btn small primary" onclick={() => convertOne(f)} disabled={row.busy || busy || !row.name.trim()}>Convertir en secreto</button>
          </div>
          {#if row.error}<p class="error" role="alert">{row.error}</p>{/if}
        {:else}
          <p class="manual">Edítalo a mano (usa <code>{"{{variable}}"}</code>).</p>
        {/if}
      </li>
    {/each}
  </ul>
  {#snippet footer()}
    {#if convertible.filter(canConvert).length > 1}<button class="btn left" onclick={convertAll} disabled={busy}>Convertir todos</button>{/if}
    <button class="btn" onclick={close} disabled={busy}>Cancelar</button>
    <button class="btn" onclick={saveAnyway} disabled={busy}>Guardar igualmente</button>
  {/snippet}
</Modal>

<style>
  .intro { margin: 0 0 14px; font-size: 12.5px; line-height: 1.55; color: var(--text-muted); }
  code { font: 11px var(--mono); background: var(--bg-active); padding: 1px 5px; border-radius: 4px; }
  .list { list-style: none; margin: 0; padding: 0; border: 1px solid var(--border-strong); border-radius: 7px; overflow: hidden; }
  .finding { display: flex; flex-direction: column; gap: 8px; padding: 11px 12px; border-top: 1px solid var(--border); }
  .finding:first-child { border-top: 0; }
  .what { display: flex; gap: 10px; align-items: flex-start; min-width: 0; }
  .lock { display: grid; place-items: center; width: 24px; height: 24px; flex: 0 0 24px; border-radius: 6px; color: var(--warn); background: color-mix(in srgb, var(--warn) 14%, transparent); }
  .meta { min-width: 0; display: flex; flex-direction: column; gap: 3px; }
  .loc { font-size: 12.5px; font-weight: 600; display: flex; gap: 8px; align-items: baseline; flex-wrap: wrap; }
  .kind { font-weight: 400; font-size: 11px; color: var(--text-faint); }
  .preview { align-self: flex-start; color: var(--text-muted); max-width: 100%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .convert { display: flex; gap: 6px; align-items: center; padding-left: 34px; flex-wrap: wrap; }
  .name { display: flex; align-items: center; flex: 1 1 180px; min-width: 0; border: 1px solid var(--border-strong); border-radius: var(--radius); background: var(--bg-raised); padding: 0 6px; }
  .name:focus-within { border-color: var(--accent); }
  .name input { flex: 1; min-width: 0; border: 0; background: transparent; font-size: 12px; padding: 5px 2px; box-shadow: none; outline: none; }
  .brace { color: var(--text-faint); font: 11px var(--mono); }
  select { font-size: 12px; max-width: 200px; }
  .manual { margin: 0; padding-left: 34px; font-size: 12px; color: var(--text-muted); }
  .error { margin: 0; padding-left: 34px; font-size: 11.5px; color: var(--err); }
  .left { margin-right: auto; }
  .sr-only { position: absolute; width: 1px; height: 1px; overflow: hidden; clip-path: inset(50%); }
</style>
