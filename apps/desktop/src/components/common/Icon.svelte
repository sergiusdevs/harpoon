<script lang="ts">
  // Iconos de trazo (estilo Lucide), inline para no depender de nada externo.
  interface Props {
    name: "folder" | "folder-open" | "get" | "post" | "put" | "patch" | "delete" | "head" | "options" | "file"
      | "file-plus" | "folder-plus" | "play" | "copy" | "pencil" | "share" | "collapse" | "reveal" | "server"
      | "settings" | "terminal" | "x" | "dots" | "vars" | "lock" | "eye" | "eye-off" | "refresh" | "check-circle" | "sparkles" | "search" | "cookie";
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
  {:else if name === "file-plus"}
    <path d="M14 3H6a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z" /><path d="M14 3v6h6" /><path d="M12 12v6M9 15h6" />
  {:else if name === "folder-plus"}
    <path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" /><path d="M12 11v5M9.5 13.5h5" />
  {:else if name === "play"}
    <path d="M7 4v16l13-8z" />
  {:else if name === "copy"}
    <rect x="9" y="9" width="12" height="12" rx="2" /><path d="M5 15H4a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1h10a1 1 0 0 1 1 1v1" />
  {:else if name === "pencil"}
    <path d="M12 20h9" /><path d="M16.5 3.5a2.1 2.1 0 0 1 3 3L7 19l-4 1 1-4z" />
  {:else if name === "share"}
    <circle cx="18" cy="5" r="3" /><circle cx="6" cy="12" r="3" /><circle cx="18" cy="19" r="3" /><path d="m8.6 13.5 6.8 4M15.4 6.5l-6.8 4" />
  {:else if name === "collapse"}
    <path d="M12 3v7" /><path d="m8 7 4 4 4-4" /><path d="M4 16h2M10 16h4M18 16h2M12 19v2" />
  {:else if name === "reveal"}
    <path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
  {:else if name === "server"}
    <rect x="3" y="4" width="18" height="7" rx="2" /><rect x="3" y="13" width="18" height="7" rx="2" /><path d="M7 7.5h.01M7 16.5h.01" />
  {:else if name === "settings"}
    <circle cx="12" cy="12" r="3" /><path d="M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3 1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8 1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z" />
  {:else if name === "terminal"}
    <rect x="3" y="4" width="18" height="16" rx="2" /><path d="m7 9 3 3-3 3M13 15h4" />
  {:else if name === "x"}
    <path d="M6 6l12 12M18 6 6 18" />
  {:else if name === "dots"}
    <circle cx="5" cy="12" r="1" fill="currentColor" /><circle cx="12" cy="12" r="1" fill="currentColor" /><circle cx="19" cy="12" r="1" fill="currentColor" />
  {:else if name === "vars"}
    <path d="M8 4c-2 0-3 1-3 3v2c0 1-1 2-2 3 1 1 2 2 2 3v2c0 2 1 3 3 3M16 4c2 0 3 1 3 3v2c0 1 1 2 2 3-1 1-2 2-2 3v2c0 2-1 3-3 3" />
  {:else if name === "refresh"}
    <path d="M20 11a8 8 0 0 0-14.6-4.5L4 8" /><path d="M4 3v5h5" /><path d="M4 13a8 8 0 0 0 14.6 4.5L20 16" /><path d="M20 21v-5h-5" />
  {:else if name === "check-circle"}
    <circle cx="12" cy="12" r="9" /><path d="m8.5 12.5 2.5 2.5 4.5-5" />
  {:else if name === "sparkles"}
    <path d="M12 3l1.8 4.6L18.5 9l-4.7 1.5L12 15l-1.8-4.5L5.5 9l4.7-1.4z" /><path d="M19 15l.8 2 2 .8-2 .8-.8 2-.8-2-2-.8 2-.8z" />
  {:else if name === "search"}
    <circle cx="11" cy="11" r="6.5" /><path d="m16 16 4 4" />
  {:else if name === "cookie"}
    <path d="M12 3a9 9 0 1 0 9 9 4 4 0 0 1-5-4 4 4 0 0 1-4-5z" /><path d="M8.5 9.5h.01M15 15h.01M9.5 15.5h.01M13 12h.01" />
  {:else if name === "lock"}
    <rect x="4" y="11" width="16" height="10" rx="2" /><path d="M8 11V7a4 4 0 0 1 8 0v4" />
  {:else if name === "eye"}
    <path d="M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7S2 12 2 12z" /><circle cx="12" cy="12" r="3" />
  {:else if name === "eye-off"}
    <path d="M2 12s3.5-7 10-7c2 0 3.7.7 5.1 1.6M22 12s-3.5 7-10 7c-2 0-3.7-.7-5.1-1.6" /><path d="M9.9 9.9a3 3 0 0 0 4.2 4.2" /><path d="m3 3 18 18" />
  {:else}
    <path d="M14 3H6a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z" /><path d="M14 3v6h6" />
  {/if}
</svg>
