<script lang="ts">
  import type { Body } from "../../domain/types";
  let { type }: { type: Body["type"] } = $props();
</script>
<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
  {#if type === "json" || type === "graphql"}<path d="M8 3H6v7l-3 2 3 2v7h2M16 3h2v7l3 2-3 2v7h-2"/>
  {:else if type === "xml"}<path d="m8 6-6 6 6 6m8-12 6 6-6 6M14 3l-4 18"/>
  {:else if type === "text"}<path d="M4 5h16M4 10h16M4 15h10M4 20h7"/>
  {:else if type === "multipart"}<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M3 10h18M10 4v16M6 7h1M6 14h1M14 14h4M14 17h3"/>
  {:else if type === "form"}<path d="M8 6h13M8 12h13M8 18h13M3 6h.01M3 12h.01M3 18h.01"/>
  {:else if type === "binary"}<path d="M14 3H5v18h14V8zM14 3v5h5M9 12v5m4-5v5"/>
  {:else}<circle cx="12" cy="12" r="8"/><path d="m6.5 6.5 11 11"/>{/if}
</svg>
