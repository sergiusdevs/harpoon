<script lang="ts">
  import { onMount } from "svelte";
  import { store } from "./state/store.svelte";
  import { isMock } from "./services/api";
  import Sidebar from "./components/collection/Sidebar.svelte";
  import RequestPane from "./components/request/RequestPane.svelte";
  import Splitter from "./components/common/Splitter.svelte";
  import Dialog from "./components/common/Dialog.svelte";
  import EnvEditor from "./components/collection/EnvEditor.svelte";
  import CollectionSettings from "./components/collection/CollectionSettings.svelte";
  import FolderSettings from "./components/collection/FolderSettings.svelte";
  import EnvironmentSelect from "./components/collection/EnvironmentSelect.svelte";
  import ThemePicker from "./components/settings/ThemePicker.svelte";
  import CurlImport from "./components/request/CurlImport.svelte";
  import VariableInspector from "./components/request/VariableInspector.svelte";

  let sidebarW = $state(250);

  onMount(() => {
    store.restore();
    if (isMock && !store.collection) void store.open("mock");
    const beforeUnload = (event: BeforeUnloadEvent) => {
      if (store.dirty) { event.preventDefault(); event.returnValue = ""; }
    };
    window.addEventListener("beforeunload", beforeUnload);
    let unlisten: (() => void) | undefined;
    if (!isMock) void import("@tauri-apps/api/window").then(async ({ getCurrentWindow }) => {
      const window = getCurrentWindow();
      unlisten = await window.onCloseRequested((event) => {
        if (!store.dirty) return;
        event.preventDefault();
        store.confirmDiscard(() => window.close());
      });
    });
    return () => { window.removeEventListener("beforeunload", beforeUnload); unlisten?.(); };
  });

</script>

<div class="app">
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
        <EnvironmentSelect />
      {/if}
    </div>
    <main class="workspace">
      <RequestPane />
    </main>
    <footer class="statusbar {store.status.kind}">
      <span class="dot"></span>
      <span class="text">{store.status.text}</span>
      {#if isMock}<span class="muted mock">modo demo (sin Tauri)</span>{/if}
    </footer>
  </div>
</div>

<Dialog />
{#if store.modal === "variables" && store.collection}
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
  .workspace { flex: 1; min-height: 0; min-width: 0; }
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
