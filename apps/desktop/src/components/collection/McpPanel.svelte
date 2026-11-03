<script lang="ts">
  // Panel "Asistentes IA (MCP)": estado, registro en Claude Code y Codex con un
  // clic y configuración para copiar en el resto de asistentes.
  import { onMount } from "svelte";
  import { store } from "../../state/store.svelte";
  import { api, type McpRegistration, type McpStatus } from "../../services/api";
  import Modal from "../common/Modal.svelte";

  const root = store.collection!.root;
  let status = $state<McpStatus | null>(null);
  let allowWrite = $state(false);
  let busy = $state<string | null>(null);
  let error = $state("");
  let copied = $state("");

  onMount(async () => {
    try {
      status = await (await api).mcpStatus();
      const current = [status.claude, status.codex].find((r) => r?.collection === root);
      if (current) allowWrite = current.allow_write;
    } catch (e) { error = String(e); }
  });

  const command = $derived(status?.cli ?? "harpoon");
  const args = $derived(["mcp", ...(allowWrite ? ["--allow-write"] : []), "-C", root]);
  const quote = (s: string) => (/[\s"'$`\\]/.test(s) ? `'${s.replaceAll("'", `'\\''`)}'` : s);
  const snippets = $derived({
    claude: `claude mcp add --scope user harpoon -- ${[command, ...args].map(quote).join(" ")}`,
    json: JSON.stringify({ mcpServers: { harpoon: { command, args } } }, null, 2),
    codex: `[mcp_servers.harpoon]\ncommand = ${JSON.stringify(command)}\nargs = ${JSON.stringify(args).replaceAll(",", ", ")}`,
  });

  function describe(r: McpRegistration | null) {
    if (!r) return { tone: "off", text: "No registrado" };
    if (r.collection !== root) return { tone: "other", text: `Apunta a otra colección: ${r.collection ?? "la carpeta desde la que se abra"}` };
    return { tone: "on", text: `Registrado para esta colección${r.allow_write ? " · con escritura" : " · solo lectura"}` };
  }

  async function register(assistant: "claude" | "codex") {
    busy = assistant; error = "";
    try {
      status = await (await api).registerMcp(root, assistant, allowWrite);
      store.status = { kind: "ok", text: `Harpoon registrado en ${assistant === "claude" ? "Claude Code" : "Codex"}. Abre una sesión nueva para usarlo.` };
    } catch (e) { error = String(e); }
    finally { busy = null; }
  }
  async function unregister(assistant: "claude" | "codex") {
    busy = assistant; error = "";
    try { status = await (await api).unregisterMcp(assistant); }
    catch (e) { error = String(e); }
    finally { busy = null; }
  }
  async function copy(id: keyof typeof snippets) {
    try { await navigator.clipboard.writeText(snippets[id]); copied = id; setTimeout(() => (copied = ""), 1500); }
    catch { error = "No se pudo copiar; selecciona el texto y cópialo a mano."; }
  }
</script>

<Modal title="Asistentes IA (MCP)" width={720} onclose={() => (store.modal = null)}>
  <p class="lead">Conecta <strong>Claude Code</strong>, <strong>Codex</strong>, Cursor o Zed a esta colección: el asistente podrá ver tus solicitudes, cabeceras, cuerpos y entornos <strong>sin ver nunca tus secretos</strong>. Funciona en local por stdio, sin red ni puertos.</p>

  {#if !status && !error}<p class="muted">Comprobando…</p>{/if}
  {#if status}
    <div class="cli" class:missing={!status.cli}>
      {#if status.cli}
        <span class="dot on"></span><span>CLI instalada · <span class="mono">{status.cli}</span></span>
      {:else}
        <span class="dot off"></span>
        <span>Falta la CLI <span class="mono">harpoon</span>. Instálala con <code>cargo install --path crates/harpoon-cli</code> desde el repositorio de Harpoon.</span>
      {/if}
    </div>

    <label class="write">
      <input type="checkbox" bind:checked={allowWrite} />
      <span><strong>Permitir que el asistente cree carpetas y solicitudes</strong><br /><span class="muted">Rechaza credenciales escritas tal cual; pide usar <code>{"{{variable}}"}</code>. Desactivado = solo lectura.</span></span>
    </label>

    <div class="assistants">
      {#each [["claude", "Claude Code", status.claude, status.claude_config], ["codex", "Codex", status.codex, status.codex_config]] as [id, name, reg, file] (id)}
        {@const state = describe(reg as McpRegistration | null)}
        {@const current = (reg as McpRegistration | null)?.collection === root && (reg as McpRegistration | null)?.allow_write === allowWrite}
        <div class="assistant">
          <div class="head"><strong>{name}</strong><span class="state {state.tone}"><span class="dot {state.tone}"></span>{state.text}</span></div>
          <p class="muted file">Se guarda en <span class="mono">{file}</span></p>
          <div class="actions">
            <button class="btn small primary" disabled={!status.cli || busy !== null || current} onclick={() => register(id as "claude" | "codex")}>
              {busy === id ? "Guardando…" : reg ? (current ? "Registrado ✓" : "Actualizar") : "Registrar"}
            </button>
            {#if reg}<button class="btn small" disabled={busy !== null} onclick={() => unregister(id as "claude" | "codex")}>Quitar</button>{/if}
          </div>
        </div>
      {/each}
    </div>
    <p class="muted note">Tras registrar, abre una sesión nueva del asistente para que cargue Harpoon.</p>

    <details>
      <summary>Configuración manual (Cursor, Zed, Claude Desktop, terminal…)</summary>
      {#each [["json", "JSON (mcpServers)"], ["claude", "Claude Code (terminal)"], ["codex", "Codex · ~/.codex/config.toml"]] as [id, label] (id)}
        <div class="snippet">
          <div class="snippet-head"><span>{label}</span><button class="btn small" onclick={() => copy(id as keyof typeof snippets)}>{copied === id ? "Copiado ✓" : "Copiar"}</button></div>
          <pre class="mono">{snippets[id as keyof typeof snippets]}</pre>
        </div>
      {/each}
    </details>

    <div class="grid">
      <div>
        <h4>Qué puede hacer</h4>
        <ul>
          <li>Listar y buscar solicitudes</li>
          <li>Ver una solicitud con lo heredado de sus carpetas</li>
          <li>Ver entornos y resolver URLs</li>
          <li>Validar la colección</li>
          {#if allowWrite}<li>Crear carpetas (con color) y solicitudes</li>{/if}
        </ul>
      </div>
      <div>
        <h4>Qué nunca ve</h4>
        <ul>
          <li>Variables secretas: solo <code>{"{{nombre}}"}</code></li>
          <li>Tokens y contraseñas escritos tal cual: <code>‹oculto›</code></li>
          <li>Cookies, tokens OAuth e historial</li>
        </ul>
      </div>
    </div>
  {/if}
  {#if error}<p class="error" role="alert">{error}</p>{/if}
  {#snippet footer()}<button class="btn" onclick={() => (store.modal = null)}>Cerrar</button>{/snippet}
</Modal>

<style>
  .lead { margin: 0 0 14px; font-size: 12.5px; line-height: 1.55; }
  code { font: 11px var(--mono); background: var(--bg-active); padding: 1px 5px; border-radius: 4px; }
  .cli { display: flex; align-items: center; gap: 8px; padding: 9px 12px; border: 1px solid var(--border); border-radius: 8px; font-size: 12px; margin-bottom: 12px; }
  .cli.missing { border-color: color-mix(in srgb, var(--warn) 50%, transparent); background: color-mix(in srgb, var(--warn) 8%, transparent); }
  .dot { flex: 0 0 auto; width: 8px; height: 8px; border-radius: 50%; background: var(--text-faint); }
  .dot.on { background: var(--ok); } .dot.off { background: var(--text-faint); } .dot.other { background: var(--warn); }
  .write { display: flex; gap: 10px; align-items: flex-start; font-size: 12px; margin-bottom: 14px; }
  .write input { margin-top: 2px; accent-color: var(--accent); }
  .assistants { display: grid; grid-template-columns: 1fr 1fr; gap: 10px; }
  .assistant { padding: 12px; border: 1px solid var(--border-strong); border-radius: 9px; background: var(--bg-panel); transition: border-color var(--t); }
  .assistant:hover { border-color: color-mix(in srgb, var(--accent) 40%, var(--border-strong)); }
  .head { display: flex; flex-direction: column; gap: 4px; font-size: 13px; }
  .state { display: inline-flex; align-items: center; gap: 6px; font-size: 11.5px; color: var(--text-muted); }
  .state.on { color: var(--ok); } .state.other { color: var(--warn); }
  .file { margin: 6px 0 10px; font-size: 11px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .actions { display: flex; gap: 6px; }
  .note { margin: 10px 0 12px; font-size: 11.5px; }
  details { margin-bottom: 14px; border: 1px solid var(--border); border-radius: 8px; padding: 8px 12px; }
  summary { cursor: pointer; font-size: 12px; font-weight: 600; }
  .snippet { margin-top: 10px; }
  .snippet-head { display: flex; justify-content: space-between; align-items: center; font-size: 11.5px; color: var(--text-muted); margin-bottom: 4px; }
  pre { margin: 0; padding: 10px; border-radius: 6px; background: var(--bg); border: 1px solid var(--border); font-size: 11px; white-space: pre-wrap; word-break: break-all; user-select: text; }
  .grid { display: grid; grid-template-columns: 1fr 1fr; gap: 14px; }
  h4 { margin: 0 0 6px; font-size: 11px; text-transform: uppercase; letter-spacing: .04em; color: var(--text-muted); }
  ul { margin: 0; padding-left: 16px; font-size: 12px; line-height: 1.7; }
  .error { color: var(--err); font-size: 12px; }
</style>
