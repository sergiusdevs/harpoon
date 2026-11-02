<script lang="ts">
  import Icon from "./Icon.svelte";
  import type { Method } from "../../domain/types";

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
  <Icon name={icon} size={9} />
  <span>{compact ? short[method] : method}</span>
</span>

<style>
  .badge {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    padding: 0 5px 0 4px;
    border-radius: 4px;
    font-family: var(--mono);
    font-size: 9px;
    font-weight: 700;
    letter-spacing: 0.02em;
    line-height: 15px;
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
