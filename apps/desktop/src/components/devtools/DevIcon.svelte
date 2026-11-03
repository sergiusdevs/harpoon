<script lang="ts">
  // Iconos del panel inferior y de los botones de disposición (estilo Lucide).
  interface Props { name: "terminal" | "globe" | "filter" | "trash" | "x" | "sidebar" | "panel-bottom" | "split-rows" | "split-cols"; size?: number; filled?: boolean }
  let { name, size = 14, filled = false }: Props = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
  {#if name === "terminal"}
    <rect x="3" y="4" width="18" height="16" rx="2.5" /><path d="m7.5 9.5 3 2.5-3 2.5M13 15h3.5" />
  {:else if name === "globe"}
    <circle cx="12" cy="12" r="9" /><path d="M3 12h18M12 3c2.5 2.7 3.5 5.7 3.5 9s-1 6.3-3.5 9c-2.5-2.7-3.5-5.7-3.5-9s1-6.3 3.5-9z" />
  {:else if name === "filter"}
    <path d="M4 5h16l-6 7.5V19l-4 1.5v-8z" />
  {:else if name === "trash"}
    <path d="M4 7h16M9 7V4.5h6V7M6.5 7l1 13h9l1-13M10 11v5.5M14 11v5.5" />
  {:else if name === "x"}
    <path d="M6 6l12 12M18 6 6 18" />
  {:else if name === "sidebar"}
    <rect x="3.5" y="4" width="17" height="16" rx="2.5" />{#if filled}<path d="M5 5.5h4v13H5z" fill="currentColor" stroke="none" />{/if}<path d="M9.5 4v16" />
  {:else if name === "panel-bottom"}
    <rect x="3.5" y="4" width="17" height="16" rx="2.5" />{#if filled}<path d="M5 14.5h14v4H5z" fill="currentColor" stroke="none" />{/if}<path d="M3.5 14h17" />
  {:else if name === "split-rows"}
    <rect x="3.5" y="4" width="17" height="16" rx="2.5" /><path d="M3.5 12h17" />
  {:else}
    <rect x="3.5" y="4" width="17" height="16" rx="2.5" /><path d="M12 4v16" />
  {/if}
</svg>
