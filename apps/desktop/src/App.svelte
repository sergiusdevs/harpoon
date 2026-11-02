<script lang="ts">
  import { onMount } from "svelte";
  import { store } from "./lib/store.svelte";
  import { isMock } from "./lib/api";
  import Sidebar from "./lib/Sidebar.svelte";
  import RequestPane from "./lib/RequestPane.svelte";
  import ResponsePane from "./lib/ResponsePane.svelte";
  import Splitter from "./lib/Splitter.svelte";
  import Dialog from "./lib/Dialog.svelte";
  import EnvEditor from "./lib/EnvEditor.svelte";
  import CollectionSettings from "./lib/CollectionSettings.svelte";

  let sidebarW = $state(250);
  let requestW = $state(0.5); // fracción del área de trabajo
  let workspace: HTMLDivElement;

  onMount(() => {
    store.restore();
    if (isMock && !store.collection) void store.open("mock");
  });

  function resizeRequest(delta: number) {
    const total = workspace.clientWidth - sidebarW;
    requestW = Math.min(0.8, Math.max(0.2, requestW + delta / total));
  }
</script>

<div class="app" bind:this={workspace}>
  <div class="col sidebar" style="width: {sidebarW}px">
    <Sidebar />
  </div>
  <Splitter onresize={(d) => (sidebarW = Math.min(480, Math.max(180, sidebarW + d)))} />
  <div class="col work">
    <div class="titlebar" data-tauri-drag-region>
      <span class="crumb">
        {#if store.collection && store.selected}
          <span class="muted">{store.collection.meta.name}</span>
          <span class="sep">/</span>
          {#each store.selected.split("/") as part, i}
            {#if i}<span class="sep">/</span>{/if}<span>{part}</span>
          {/each}
        {/if}
      </span>
      {#if store.collection}
        <label class="env">
          <span class="muted" id="env-label">Entorno</span>
          <select
            aria-labelledby="env-label"
            value={store.env ?? ""}
            onchange={(e) => {
              const v = e.currentTarget.value;
              if (v === "__edit") {
                e.currentTarget.value = store.env ?? "";
                store.modal = "environments";
              } else store.env = v || null;
            }}
          >
            <option value="">— ninguno —</option>
            {#each Object.keys(store.collection.environments) as env}<option value={env}>{env}</option>{/each}
            <option disabled>──────</option>
            <option value="__edit">Editar entornos…</option>
          </select>
          <button class="btn icon" title="Nuevo entorno" aria-label="Nuevo entorno" onclick={() => store.quickNewEnvironment()}>
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true"><path d="M12 5v14M5 12h14" /></svg>
          </button>
          <button class="btn icon" title="Editar entornos (⌘E)" aria-label="Editar entornos" onclick={() => (store.modal = "environments")}>
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M12 20h9" /><path d="M16.5 3.5a2.1 2.1 0 0 1 3 3L7 19l-4 1 1-4z" /></svg>
          </button>
        </label>
      {/if}
    </div>
    <div class="split">
      <div class="col" style="width: {requestW * 100}%">
        <RequestPane />
      </div>
      <Splitter onresize={resizeRequest} />
      <div class="col" style="flex: 1">
        <ResponsePane />
      </div>
    </div>
    <footer class="statusbar {store.status.kind}">
      <span class="dot"></span>
      <span class="text">{store.status.text}</span>
      {#if isMock}<span class="muted mock">modo demo (sin Tauri)</span>{/if}
    </footer>
  </div>
</div>

<Dialog />
{#if store.modal === "environments" && store.collection}
  <EnvEditor />
{:else if store.modal === "settings" && store.collection}
  <CollectionSettings />
{/if}

<style>
  .app {
    display: flex;
    height: 100%;
  }
  .col {
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .sidebar {
    flex: 0 0 auto;
  }
  .work {
    flex: 1;
  }
  .titlebar {
    height: var(--titlebar);
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 14px;
    border-bottom: 1px solid var(--border);
    -webkit-app-region: drag;
    font-size: 12.5px;
  }
  .crumb {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .sep {
    color: var(--text-faint);
  }
  .env {
    display: flex;
    gap: 4px;
    align-items: center;
    font-size: 12px;
    -webkit-app-region: no-drag;
  }
  .env > span {
    margin-right: 4px;
  }
  .env select {
    padding: 3px 8px;
    font-size: 12px;
  }
  .split {
    flex: 1;
    display: flex;
    min-height: 0;
  }
  .statusbar {
    height: 26px;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 12px;
    border-top: 1px solid var(--border);
    font-size: 11.5px;
    color: var(--text-muted);
  }
  .statusbar .text {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    user-select: text;
  }
  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--text-faint);
    flex: 0 0 auto;
    transition: background var(--t);
  }
  .ok .dot {
    background: var(--ok);
  }
  .err .dot {
    background: var(--err);
  }
  .busy .dot {
    background: var(--accent);
    animation: pulse 1s ease-in-out infinite;
  }
  .mock {
    margin-left: auto;
    font-size: 11px;
  }
</style>
