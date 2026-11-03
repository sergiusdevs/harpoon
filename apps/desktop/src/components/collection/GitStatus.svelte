<script lang="ts">
  // Botón de la barra de título: rama actual si la colección está en Git,
  // o "Inicializar Git" si no lo está.
  import { onMount } from "svelte";
  import { api, type GitStatus } from "../../services/api";
  import { store } from "../../state/store.svelte";

  let git = $state<GitStatus | null>(null);
  let busy = $state(false);
  const root = $derived(store.collection?.root ?? null);

  async function refresh() {
    if (!root) return void (git = null);
    const forRoot = root;
    try {
      const status = await (await api).gitStatus(forRoot);
      if (forRoot === root) git = status;
    } catch {
      git = null;
    }
  }

  function init() {
    if (!root || busy) return;
    const target = root;
    store.dialog = {
      kind: "confirm",
      title: "Inicializar Git",
      text: `Se creará un repositorio Git en ${target}. Los secretos y el estado local ya están en .gitignore.`,
      ok: "Inicializar",
      onOk: async () => {
        busy = true;
        try {
          git = await (await api).gitInit(target);
          store.status = { kind: "ok", text: `Repositorio Git creado en ${target}` };
        } catch (e) {
          store.status = { kind: "err", text: String(e) };
        } finally {
          busy = false;
        }
      },
    };
  }

  $effect(() => {
    void root;
    void refresh();
  });

  // La rama puede cambiar fuera de la app (terminal, otro editor).
  onMount(() => {
    const onFocus = () => void refresh();
    window.addEventListener("focus", onFocus);
    return () => window.removeEventListener("focus", onFocus);
  });
</script>

{#snippet branchIcon()}
  <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="6" cy="5" r="2.2" /><circle cx="6" cy="19" r="2.2" /><circle cx="18" cy="7" r="2.2" /><path d="M6 7.2v9.6" /><path d="M18 9.2c0 4.5-6 4-11.2 7.6" /></svg>
{/snippet}

{#if git?.available}
  {#if git.initialized}
    <span class="git on" title={git.branch ? `Repositorio Git · rama ${git.branch}` : "Repositorio Git (HEAD separado)"}>
      {@render branchIcon()}<span class="label">{git.branch ?? "HEAD separado"}</span>
    </span>
  {:else}
    <button class="git" disabled={busy} title="La colección no está en un repositorio Git" onclick={init}>
      {@render branchIcon()}<span class="label">{busy ? "Inicializando…" : "Inicializar Git"}</span>
    </button>
  {/if}
{/if}

<style>
  .git { display: flex; align-items: center; gap: 7px; flex: 0 0 auto; max-width: 180px; height: 30px; padding: 0 10px; border: 1px solid var(--border); border-radius: var(--radius); background: var(--bg-raised); color: var(--text); font-size: 12px; -webkit-app-region: no-drag; transition: background var(--t), border-color var(--t), transform var(--t); }
  .git :global(svg) { flex: 0 0 auto; color: var(--accent); }
  button.git:hover:not(:disabled) { border-color: var(--accent); background: var(--accent-soft); transform: translateY(-1px); }
  .git.on { border-color: transparent; background: transparent; color: var(--text-muted); }
  .git.on :global(svg) { color: var(--ok); }
  .label { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
</style>
