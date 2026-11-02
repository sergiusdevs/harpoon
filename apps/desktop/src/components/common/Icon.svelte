<script lang="ts">
  // Iconos de trazo (estilo Lucide), inline para no depender de nada externo.
  interface Props {
    name: "folder" | "folder-open" | "get" | "post" | "put" | "patch" | "delete" | "head" | "options" | "file";
    size?: number;
  }
  let { name, size = 14 }: Props = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
  {#if name === "folder"}
    <path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
  {:else if name === "folder-open"}
    <path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v2H3z" /><path d="M3 11h18l-2 8H5z" />
  {:else if name === "get"}
    <path d="M12 5v14" /><path d="m6 13 6 6 6-6" />
  {:else if name === "post"}
    <path d="M12 5v14M5 12h14" />
  {:else if name === "put"}
    <path d="M12 19V5" /><path d="m6 11 6-6 6 6" />
  {:else if name === "patch"}
    <path d="M12 20h9" /><path d="M16.5 3.5a2.1 2.1 0 0 1 3 3L7 19l-4 1 1-4z" />
  {:else if name === "delete"}
    <path d="M3 6h18" /><path d="M8 6V4h8v2" /><path d="M19 6l-1 14H6L5 6" /><path d="M10 11v6M14 11v6" />
  {:else if name === "head"}
    <circle cx="12" cy="12" r="9" /><path d="M12 8h.01M11 12h1v4h1" />
  {:else if name === "options"}
    <path d="M8 6h13M8 12h13M8 18h13" /><path d="M3 6h.01M3 12h.01M3 18h.01" />
  {:else}
    <path d="M14 3H6a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z" /><path d="M14 3v6h6" />
  {/if}
</svg>
