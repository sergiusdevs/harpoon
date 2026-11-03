<script lang="ts">
  // Panel inferior: consola de scripts y red. Solo pinta lo que manda Rust.
  import { tick } from "svelte";
  import { devtools, type LogFilter, type NetFilter, type NetEntry } from "../../state/devtools.svelte";
  import { store, fmtSize } from "../../state/store.svelte";
  import Icon from "./DevIcon.svelte";

  const logFilters: { value: LogFilter; label: string }[] = [
    { value: "all", label: "Todos" }, { value: "log", label: "Log" }, { value: "info", label: "Info" },
    { value: "warn", label: "Avisos" }, { value: "error", label: "Errores" }, { value: "debug", label: "Debug" },
  ];
  const netFilters: { value: NetFilter; label: string }[] = [
    { value: "all", label: "Todos" }, { value: "2xx", label: "2xx" }, { value: "3xx", label: "3xx" },
    { value: "4xx", label: "4xx" }, { value: "5xx", label: "5xx" }, { value: "error", label: "Errores" },
  ];

  const logs = $derived(devtools.logFilter === "all" ? devtools.logs : devtools.logs.filter((l) => l.level === devtools.logFilter));
  const netMatches = (n: NetEntry, f: NetFilter) =>
    f === "all" || (f === "error" ? n.status === null : n.status !== null && Math.floor(n.status / 100) === Number(f[0]));
  const net = $derived(devtools.netFilter === "all" ? devtools.net : devtools.net.filter((n) => netMatches(n, devtools.netFilter)));
  const errorCount = $derived(devtools.logs.filter((l) => l.level === "error").length);

  function time(at: number) {
    const d = new Date(at);
    const p = (n: number, w = 2) => String(n).padStart(w, "0");
    return `${p(d.getHours())}:${p(d.getMinutes())}:${p(d.getSeconds())}.${p(d.getMilliseconds(), 3)}`;
  }
  const statusClass = (s: number | null) => (s === null ? "s-err" : s < 300 ? "s-ok" : s < 400 ? "s-redir" : s < 500 ? "s-warn" : "s-err");
  function openRequest(key: string) { if (store.collection?.requests[key]) store.select(key); }

  // Autoscroll de la consola mientras se esté mirando el final.
  let scroller = $state<HTMLDivElement | null>(null);
  let pinned = true;
  function onScroll() { if (scroller) pinned = scroller.scrollHeight - scroller.scrollTop - scroller.clientHeight < 24; }
  $effect(() => {
    void logs.length; void net.length; void devtools.tab;
    if (pinned) void tick().then(() => scroller && (scroller.scrollTop = scroller.scrollHeight));
  });
</script>

<section class="devtools" style="height: {devtools.height}px" aria-label="Consola y red">
  <header class="bar">
    <div class="tabs" role="tablist">
      <button role="tab" class:active={devtools.tab === "console"} aria-selected={devtools.tab === "console"} onclick={() => devtools.setTab("console")}>
        <Icon name="terminal" size={15} /><span>Consola</span>{#if errorCount}<span class="count err">{errorCount}</span>{/if}
      </button>
      <button role="tab" class:active={devtools.tab === "network"} aria-selected={devtools.tab === "network"} onclick={() => devtools.setTab("network")}>
        <Icon name="globe" size={15} /><span>Red</span>{#if devtools.net.length}<span class="count">{devtools.net.length}</span>{/if}
      </button>
    </div>
    <div class="tools">
      <label class="filter" title="Filtrar">
        <Icon name="filter" size={14} />
        {#if devtools.tab === "console"}
          <select bind:value={devtools.logFilter} aria-label="Filtrar logs">{#each logFilters as f}<option value={f.value}>{f.label}</option>{/each}</select>
        {:else}
          <select bind:value={devtools.netFilter} aria-label="Filtrar peticiones">{#each netFilters as f}<option value={f.value}>{f.label}</option>{/each}</select>
        {/if}
      </label>
      <button class="icon-btn" title="Vaciar" aria-label="Vaciar" onclick={() => devtools.clear()}><Icon name="trash" size={15} /></button>
      <button class="icon-btn" title="Cerrar (⌘J)" aria-label="Cerrar panel" onclick={() => devtools.close()}><Icon name="x" size={15} /></button>
    </div>
  </header>

  <div class="body" bind:this={scroller} onscroll={onScroll}>
    {#if devtools.tab === "console"}
      {#if logs.length === 0}
        <div class="empty">
          <Icon name="terminal" size={44} />
          <p class="title">{devtools.logs.length ? "Ningún log con este filtro" : "No hay logs"}</p>
          <p class="hint">Los <code>console.log</code> de los scripts aparecerán aquí al enviar solicitudes.</p>
        </div>
      {:else}
        <div class="logs mono" role="log">
          {#each logs as l (l.id)}
            <div class="log {l.level}">
              <span class="t">{time(l.at)}</span>
              <span class="lvl">{l.level}</span>
              <span class="msg">{l.message}</span>
              <button class="src" title="Abrir solicitud" onclick={() => openRequest(l.source)}>{l.source}</button>
            </div>
          {/each}
        </div>
      {/if}
    {:else if net.length === 0}
      <div class="empty">
        <Icon name="globe" size={44} />
        <p class="title">{devtools.net.length ? "Ninguna petición con este filtro" : "No hay peticiones"}</p>
        <p class="hint">Cada envío (también desde el runner) aparecerá aquí.</p>
      </div>
    {:else}
      <table class="net">
        <thead><tr><th class="c-method">Método</th><th class="c-status">Estado</th><th class="c-domain">Dominio</th><th>Ruta</th><th class="c-time">Hora</th><th class="c-num">Duración</th><th class="c-num">Tamaño</th></tr></thead>
        <tbody>
          {#each net as n (n.id)}
            <tr onclick={() => openRequest(n.key)} title={n.error ?? `${n.method} ${n.domain}${n.path}`}>
              <td class="c-method mono">{n.method}</td>
              <td class="c-status mono {statusClass(n.status)}">{n.status ?? "Error"}</td>
              <td class="c-domain">{n.domain}</td>
              <td class="mono path">{n.path}</td>
              <td class="c-time mono">{time(n.at)}</td>
              <td class="c-num mono">{n.duration_ms === null ? "-" : `${n.duration_ms}ms`}</td>
              <td class="c-num mono">{n.size === null ? "-" : fmtSize(n.size)}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </div>
</section>

<style>
  .devtools { display: flex; flex-direction: column; min-height: 0; flex: 0 0 auto; background: var(--bg); border-top: 1px solid var(--border); }
  .bar { display: flex; align-items: stretch; justify-content: space-between; height: 36px; flex: 0 0 36px; padding: 0 8px; background: var(--bg-panel); border-bottom: 1px solid var(--border); }
  .tabs { display: flex; gap: 2px; }
  .tabs button { display: inline-flex; align-items: center; gap: 7px; padding: 0 12px; font-size: 12.5px; color: var(--text-muted); border-bottom: 2px solid transparent; border-radius: 0; }
  .tabs button:hover { color: var(--text); }
  .tabs button.active { color: var(--accent); border-bottom-color: var(--accent); }
  .count { font-size: 10.5px; padding: 0 5px; border-radius: 8px; background: var(--bg-active); color: var(--text-muted); }
  .count.err { background: color-mix(in srgb, var(--err) 22%, transparent); color: var(--err); }
  .tools { display: flex; align-items: center; gap: 4px; }
  .filter { display: inline-flex; align-items: center; gap: 4px; color: var(--text-muted); padding: 0 4px; }
  .filter select { background: transparent; border: 0; color: var(--text); font-size: 12.5px; padding: 2px 2px; cursor: pointer; }
  .filter select option { background: var(--bg-raised); }
  .icon-btn { display: inline-flex; align-items: center; justify-content: center; width: 28px; height: 26px; border-radius: 5px; color: var(--text-muted); }
  .icon-btn:hover { color: var(--text); background: var(--bg-hover); }
  .body { flex: 1; min-height: 0; overflow: auto; }

  .empty { height: 100%; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 6px; color: var(--text-faint); text-align: center; padding: 16px; }
  .empty .title { margin: 8px 0 0; font-size: 13.5px; font-weight: 600; color: var(--text-muted); }
  .empty .hint { margin: 0; font-size: 12.5px; }
  .empty code { font-family: var(--mono); font-size: 11.5px; }

  .logs { font-size: 12px; padding: 4px 0; }
  .log { display: flex; align-items: baseline; gap: 10px; padding: 3px 12px; border-bottom: 1px solid color-mix(in srgb, var(--border) 50%, transparent); content-visibility: auto; contain-intrinsic-size: auto 22px; }
  .log:hover { background: var(--bg-hover); }
  .t { color: var(--text-faint); flex: 0 0 auto; }
  .lvl { flex: 0 0 44px; text-transform: uppercase; font-size: 10px; font-weight: 600; color: var(--text-faint); }
  .msg { flex: 1; min-width: 0; white-space: pre-wrap; word-break: break-word; user-select: text; color: var(--text); }
  .log.warn { background: color-mix(in srgb, var(--warn) 8%, transparent); }
  .log.warn .lvl, .log.warn .msg { color: var(--warn); }
  .log.error { background: color-mix(in srgb, var(--err) 9%, transparent); }
  .log.error .lvl, .log.error .msg { color: var(--err); }
  .log.info .lvl { color: var(--accent); }
  .log.debug .msg { color: var(--text-muted); }
  .src { flex: 0 0 auto; max-width: 30%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font: 11px var(--font); color: var(--text-faint); }
  .src:hover { color: var(--accent); text-decoration: underline; }

  .net { width: 100%; border-collapse: collapse; table-layout: fixed; font-size: 12.5px; }
  .net th { position: sticky; top: 0; z-index: 1; background: var(--bg-panel); text-align: left; font-weight: 500; font-size: 11px; letter-spacing: .06em; text-transform: uppercase; color: var(--text-muted); padding: 7px 12px; border-bottom: 1px solid var(--border); border-right: 1px solid var(--border); }
  .net td { padding: 5px 12px; border-right: 1px solid var(--border); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; color: var(--text); }
  .net th:last-child, .net td:last-child { border-right: 0; }
  .net tr { content-visibility: auto; contain-intrinsic-size: auto 29px; }
  .net tbody tr { cursor: default; }
  .net tbody tr:hover { background: var(--bg-hover); }
  .c-method { width: 90px; font-size: 12px; }
  .c-status { width: 80px; }
  .c-domain { width: 20%; }
  .c-time { width: 120px; color: var(--text-faint) !important; }
  .c-num { width: 96px; text-align: right !important; }
  .s-ok { color: var(--ok) !important; }
  .s-redir { color: var(--accent) !important; }
  .s-warn { color: var(--warn) !important; }
  .s-err { color: var(--err) !important; }
</style>
