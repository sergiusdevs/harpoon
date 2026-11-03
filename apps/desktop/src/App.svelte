<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { streams } from "./state/streams.svelte";
  import { store } from "./state/store.svelte";
  import { isMock } from "./services/api";
  import Sidebar from "./components/collection/Sidebar.svelte";
  import RequestTabs from "./components/request/RequestTabs.svelte";
  import RequestPane from "./components/request/RequestPane.svelte";
  import Splitter from "./components/common/Splitter.svelte";
  import Dialog from "./components/common/Dialog.svelte";
  import EnvEditor from "./components/collection/EnvEditor.svelte";
  import CollectionSettings from "./components/collection/CollectionSettings.svelte";
  import FolderSettings from "./components/collection/FolderSettings.svelte";
  import EnvironmentSelect from "./components/collection/EnvironmentSelect.svelte";
  import GitStatus from "./components/collection/GitStatus.svelte";
  import ThemePicker from "./components/settings/ThemePicker.svelte";
  import CurlImport from "./components/request/CurlImport.svelte";
  import VariableInspector from "./components/request/VariableInspector.svelte";
  import CookieManager from "./components/collection/CookieManager.svelte";
  import ShareCollection from "./components/collection/ShareCollection.svelte";
  import CollectionRunner from "./components/collection/CollectionRunner.svelte";
  import CollectionSearch from "./components/collection/CollectionSearch.svelte";
  import LiveConnection from "./components/request/LiveConnection.svelte";
  import MockServer from "./components/collection/MockServer.svelte";
  import DevTools from "./components/devtools/DevTools.svelte";
  import DevIcon from "./components/devtools/DevIcon.svelte";
  import { devtools } from "./state/devtools.svelte";
  import SecretFindings from "./components/secrets/SecretFindings.svelte";
  import MissingSecrets from "./components/secrets/MissingSecrets.svelte";
  import MissingIndicator from "./components/secrets/MissingIndicator.svelte";

  let sidebarW = $state(250);

  // Secretos sin valor en esta máquina: se recalculan al cambiar de colección o de entorno…
  const collectionRoot = $derived(store.collection?.root ?? null);
  $effect(() => {
    void collectionRoot; void store.env;
    untrack(() => void store.refreshMissingSecrets());
  });
  // …y se piden una vez por colección+entorno, si no hay otro modal o diálogo abierto.
  $effect(() => {
    if (store.missingSecrets.length && store.dismissedMissing !== store.missingKey && !store.modal && !store.dialog) store.modal = "missing-secrets";
  });

  onMount(() => {
    store.restore();
    void devtools.start();
    if (isMock && !store.collection) void store.open("mock");
    const beforeUnload = (event: BeforeUnloadEvent) => {
      if (store.hasUnsavedTabs) { event.preventDefault(); event.returnValue = ""; }
    };
    window.addEventListener("beforeunload", beforeUnload);
    let unlisten: (() => void) | undefined;
    if (!isMock) void import("@tauri-apps/api/window").then(async ({ getCurrentWindow }) => {
      const window = getCurrentWindow();
      unlisten = await window.onCloseRequested((event) => {
        if (!store.hasUnsavedTabs) return;
        event.preventDefault();
        store.confirmDiscardAll(() => window.close());
      });
    });
    return () => { window.removeEventListener("beforeunload", beforeUnload); unlisten?.(); };
  });

  function onKey(e: KeyboardEvent) {
    if (!(e.metaKey || e.ctrlKey) || e.shiftKey || e.altKey || e.defaultPrevented) return;
    if (e.key === "j") { e.preventDefault(); devtools.toggle(); }
    else if (e.key === "b" && !store.modal && !store.dialog) { e.preventDefault(); store.toggleSidebar(); }
  }
</script>

<svelte:window onkeydown={onKey} />

<div class="app">
  {#if !store.sidebarHidden}
    <div class="col sidebar" style="width: {sidebarW}px">
      <Sidebar />
    </div>
    <Splitter onresize={(d) => (sidebarW = Math.min(480, Math.max(180, sidebarW + d)))} />
  {/if}
  <div class="col work">
    <div class="titlebar" class:no-sidebar={store.sidebarHidden} data-tauri-drag-region>
      <span class="crumb">
        {#if store.collection && store.selected}
          <span class="muted">{store.collection.meta.name}</span>
          <span class="sep">/</span>
          {#each store.selected.split("/") as part, i}
            {#if i}<span class="sep">/</span>{/if}<span>{part}</span>
          {/each}
        {/if}
      </span>
      <div class="title-actions">
        {#if store.collection}
          <MissingIndicator />
          <GitStatus />
          <EnvironmentSelect />
        {/if}
        <div class="layout-buttons" role="group" aria-label="Disposición">
          <button class="layout-btn" class:on={!store.sidebarHidden} title="Barra lateral (⌘B)" aria-label="Mostrar u ocultar la barra lateral" aria-pressed={!store.sidebarHidden} onclick={() => store.toggleSidebar()}><DevIcon name="sidebar" size={17} filled={!store.sidebarHidden} /></button>
          <button class="layout-btn" class:on={devtools.open} title="Consola y red (⌘J)" aria-label="Mostrar u ocultar la consola" aria-pressed={devtools.open} onclick={() => devtools.toggle()}><DevIcon name="panel-bottom" size={17} filled={devtools.open} />{#if devtools.unseenErrors}<span class="err-dot" aria-label="errores nuevos"></span>{/if}</button>
          <button class="layout-btn on" title={store.forceStacked ? "Respuesta al lado" : "Respuesta debajo"} aria-label="Cambiar disposición de solicitud y respuesta" onclick={() => store.toggleStacked()}><DevIcon name={store.forceStacked ? "split-cols" : "split-rows"} size={17} /></button>
        </div>
      </div>
    </div>
    {#if store.collection}<RequestTabs />{/if}
    <main class="workspace">
      <RequestPane />
    </main>
    {#if devtools.open}
      <Splitter direction="vertical" onresize={(d) => devtools.resize(d)} />
      <DevTools />
    {/if}
    <footer class="statusbar {store.status.kind}">
      <span class="dot"></span>
      <span class="text">{store.status.text}</span>
      {#if isMock}<span class="muted mock">modo demo (sin Tauri)</span>{/if}
    </footer>
  </div>
</div>

<Dialog />
{#if store.modal === "secret-findings" && store.secretFindings}
  <SecretFindings />
{:else if store.modal === "missing-secrets" && store.collection}
  <MissingSecrets />
{:else if store.modal === "mock" && store.collection}
  <MockServer />
{:else if store.modal === "stream" && streams.current}
  <LiveConnection />
{:else if store.modal === "search" && store.collection}
  <CollectionSearch />
{:else if store.modal === "runner" && store.collection}
  <CollectionRunner />
{:else if store.modal === "share"}
  <ShareCollection />
{:else if store.modal === "cookies" && store.collection}
  <CookieManager />
{:else if store.modal === "variables" && store.collection}
  <VariableInspector />
{:else if store.modal === "curl" && store.collection}
  <CurlImport />
{:else if store.modal === "themes"}
  <ThemePicker />
{:else if store.modal === "environments" && store.collection}
  <EnvEditor />
{:else if store.modal === "folder" && store.collection}
  <FolderSettings />
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
    flex: 0 0 var(--titlebar);
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 14px;
    border-bottom: 1px solid var(--border);
    -webkit-app-region: drag;
    font-size: 12.5px;
  }
  .crumb {
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .sep {
    color: var(--text-faint);
  }
  .workspace { flex: 1; min-height: 120px; min-width: 0; }
  .titlebar.no-sidebar { padding-left: 84px; } /* hueco para los semáforos de macOS */
  .title-actions { display: flex; align-items: center; gap: 12px; -webkit-app-region: no-drag; }
  .layout-buttons { display: flex; align-items: center; gap: 2px; }
  .layout-btn { position: relative; display: inline-flex; align-items: center; justify-content: center; width: 30px; height: 28px; border-radius: 6px; color: var(--text-faint); }
  .layout-btn.on { color: var(--text-muted); }
  .layout-btn:hover { color: var(--text); background: var(--bg-hover); }
  .err-dot { position: absolute; top: 4px; right: 4px; width: 7px; height: 7px; border-radius: 50%; background: var(--err); box-shadow: 0 0 0 2px var(--bg); }
  .statusbar {
    height: 26px;
    flex: 0 0 26px;
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
