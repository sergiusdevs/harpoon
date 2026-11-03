<script lang="ts">
  // Panel "Asistentes IA (MCP)": registrar Harpoon en Claude Code o Codex con un
  // clic y configuración para copiar en el resto de asistentes.
  import { onMount } from "svelte";
  import { store } from "../../state/store.svelte";
  import { api, type McpRegistration, type McpStatus } from "../../services/api";
  import Modal from "../common/Modal.svelte";

  type Assistant = "claude" | "codex";
  const root = store.collection!.root;
  let status = $state<McpStatus | null>(null);
  let allowWrite = $state(false);
  let busy = $state<Assistant | null>(null);
  let justRegistered = $state(false);
  let error = $state("");
  let copied = $state("");

  onMount(async () => {
    try {
      status = await (await api).mcpStatus();
      const current = [status.claude, status.codex].find((r) => r?.collection === root);
      if (current) allowWrite = current.allow_write;
    } catch (e) { error = String(e); }
  });

  const assistants = $derived(status ? ([
    { id: "claude", name: "Claude Code", reg: status.claude, file: status.claude_config },
    { id: "codex", name: "Codex", reg: status.codex, file: status.codex_config },
  ] as const) : []);

  /** Registrado para esta colección y con el mismo permiso de escritura. */
  const upToDate = (r: McpRegistration | null) => r?.collection === root && r.allow_write === allowWrite;

  function label(r: McpRegistration | null) {
    if (!r) return "";
    if (r.collection !== root) return "otra colección";
    return r.allow_write ? "con escritura" : "solo lectura";
  }

  const command = $derived(status?.cli ?? "harpoon");
  const args = $derived(["mcp", ...(allowWrite ? ["--allow-write"] : []), "-C", root]);
  const quote = (s: string) => (/[\s"'$`\\]/.test(s) ? `'${s.replaceAll("'", `'\\''`)}'` : s);
  const snippets = $derived({
    json: JSON.stringify({ mcpServers: { harpoon: { command, args } } }, null, 2),
    terminal: `claude mcp add --scope user harpoon -- ${[command, ...args].map(quote).join(" ")}`,
  });

  async function register(id: Assistant) {
    busy = id; error = "";
    try { status = await (await api).registerMcp(root, id, allowWrite); justRegistered = true; }
    catch (e) { error = String(e); }
    finally { busy = null; }
  }
  async function unregister(id: Assistant) {
    busy = id; error = "";
    try { status = await (await api).unregisterMcp(id); }
    catch (e) { error = String(e); }
    finally { busy = null; }
  }
  async function copy(id: keyof typeof snippets) {
    try { await navigator.clipboard.writeText(snippets[id]); copied = id; setTimeout(() => (copied = ""), 1500); }
    catch { error = "No se pudo copiar; selecciona el texto y cópialo a mano."; }
  }
</script>

<Modal title="Asistentes IA (MCP)" width={540} onclose={() => (store.modal = null)}>
  <p class="lead muted">Tu asistente verá las solicitudes y entornos de esta colección, nunca tus secretos.</p>

  {#if status && !status.cli}
    <p class="warning">Falta la CLI <code>harpoon</code>: instálala con <code>cargo install --path crates/harpoon-cli</code>.</p>
  {/if}

  {#if !status && !error}<p class="muted">Comprobando…</p>{/if}
  {#if status}
    <ul class="assistants">
      {#each assistants as a (a.id)}
        <li title={`Configuración en ${a.file}`}>
          <span class="name">{a.name}</span>
          {#if a.reg}
            <span class="state" class:ok={upToDate(a.reg)}>{upToDate(a.reg) ? "✓ " : ""}{label(a.reg)}</span>
          {/if}
          <span class="spacer"></span>
          {#if a.reg && upToDate(a.reg)}
            <button class="link" disabled={busy !== null} onclick={() => unregister(a.id)}>Quitar</button>
          {:else}
            <button class="btn small primary" disabled={!status.cli || busy !== null} onclick={() => register(a.id)}>
              {busy === a.id ? "Guardando…" : a.reg ? "Actualizar" : "Conectar"}
            </button>
          {/if}
        </li>
      {/each}
    </ul>

    <label class="switch">
      <input type="checkbox" bind:checked={allowWrite} />
      <span>Permitir que cree carpetas y solicitudes</span>
    </label>

    {#if justRegistered}<p class="hint ok">Listo. Abre una sesión nueva del asistente para que cargue Harpoon.</p>{/if}

    <details>
      <summary>Otros asistentes</summary>
      {#each [["json", "Cursor, Zed, Claude Desktop (JSON)"], ["terminal", "Terminal (Claude Code)"]] as [id, title] (id)}
        <div class="snippet">
          <div class="snippet-head"><span>{title}</span><button class="link" onclick={() => copy(id as keyof typeof snippets)}>{copied === id ? "Copiado ✓" : "Copiar"}</button></div>
          <pre class="mono">{snippets[id as keyof typeof snippets]}</pre>
        </div>
      {/each}
    </details>
  {/if}
  {#if error}<p class="error" role="alert">{error}</p>{/if}
  {#snippet footer()}<button class="btn" onclick={() => (store.modal = null)}>Cerrar</button>{/snippet}
</Modal>

<style>
  .lead { margin: 0 0 16px; font-size: 12.5px; }
  code { font: 11px var(--mono); background: var(--bg-active); padding: 1px 5px; border-radius: 4px; }
  .warning { margin: 0 0 14px; padding: 8px 10px; font-size: 12px; border-radius: 7px; color: var(--warn); background: color-mix(in srgb, var(--warn) 10%, transparent); }
  .assistants { list-style: none; margin: 0 0 14px; padding: 0; border: 1px solid var(--border); border-radius: 9px; }
  .assistants li { display: flex; align-items: center; gap: 10px; min-height: 48px; padding: 0 12px; }
  .assistants li + li { border-top: 1px solid var(--border); }
  .name { font-size: 13px; font-weight: 600; }
  .state { font-size: 12px; color: var(--text-muted); }
  .state.ok { color: var(--ok); }
  .spacer { flex: 1; }
  .link { font-size: 12px; color: var(--text-muted); padding: 4px 6px; border-radius: 5px; }
  .link:hover:not(:disabled) { color: var(--text); background: var(--bg-hover); }
  .switch { display: flex; align-items: center; gap: 8px; font-size: 12.5px; color: var(--text-muted); }
  .switch input { accent-color: var(--accent); margin: 0; }
  .hint { margin: 12px 0 0; font-size: 12px; }
  .hint.ok { color: var(--ok); }
  details { margin-top: 16px; }
  summary { cursor: pointer; font-size: 12px; color: var(--text-muted); }
  summary:hover { color: var(--text); }
  .snippet { margin-top: 10px; }
  .snippet-head { display: flex; justify-content: space-between; align-items: center; font-size: 11.5px; color: var(--text-muted); margin-bottom: 4px; }
  pre { margin: 0; padding: 10px; border-radius: 6px; background: var(--bg); border: 1px solid var(--border); font-size: 11px; white-space: pre-wrap; word-break: break-all; user-select: text; }
  .error { color: var(--err); font-size: 12px; }
</style>
