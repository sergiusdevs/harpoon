<script lang="ts">
  import Icon from "./Icon.svelte";
  import type { Method } from "./types";

  interface Props {
    method: Method;
    /** Texto corto (GET → GET, DELETE → DEL, OPTIONS → OPT). */
    compact?: boolean;
  }
  let { method, compact = true }: Props = $props();
  const short: Record<Method, string> = { GET: "GET", POST: "POST", PUT: "PUT", PATCH: "PATCH", DELETE: "DEL", HEAD: "HEAD", OPTIONS: "OPT" };
  const icon = $derived(method.toLowerCase() as "get" | "post" | "put" | "patch" | "delete" | "head" | "options");
</script>

<span class="badge {method}" title={method}>
  <Icon name={icon} size={11} />
  <span>{compact ? short[method] : method}</span>
</span>

<style>
  .badge {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 1px 6px 1px 5px;
    border-radius: 4px;
    font-family: var(--mono);
    font-size: 10.5px;
    font-weight: 700;
    letter-spacing: 0.02em;
    line-height: 16px;
    background: color-mix(in srgb, currentColor 14%, transparent);
    white-space: nowrap;
  }
  .GET {
    color: var(--m-get);
  }
  .POST {
    color: var(--m-post);
  }
  .PUT {
    color: var(--m-put);
  }
  .PATCH {
    color: var(--m-patch);
  }
  .DELETE {
    color: var(--m-delete);
  }
  .HEAD,
  .OPTIONS {
    color: var(--m-head);
  }
</style>
