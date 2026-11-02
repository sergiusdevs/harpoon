<script lang="ts">
  // Formulario de autenticación, compartido por solicitud y colección.
  import type { Auth } from "./types";

  interface Props {
    auth: Auth;
    onchange: (auth: Auth) => void;
    /** En la colección no tiene sentido "heredar". */
    allowInherit?: boolean;
    inheritHint?: string;
  }
  let { auth, onchange, allowInherit = true, inheritHint = "" }: Props = $props();

  function setKind(kind: Auth["type"]) {
    const a: Auth =
      kind === "basic" ? { type: "basic", username: "", password: "" }
      : kind === "bearer" ? { type: "bearer", token: "" }
      : kind === "api_key" ? { type: "api_key", key: "", value: "", in: "header" }
      : { type: kind };
    onchange(a);
  }
</script>

<div class="form">
  <label>
    <span>Tipo</span>
    <select value={auth.type} onchange={(e) => setKind(e.currentTarget.value as Auth["type"])}>
      {#if allowInherit}<option value="inherit">Heredar de la colección</option>{/if}
      <option value="none">Ninguna</option>
      <option value="basic">Basic</option>
      <option value="bearer">Bearer</option>
      <option value="api_key">API key</option>
    </select>
  </label>
  {#if auth.type === "basic"}
    {@const a = auth}
    <label><span>Usuario</span><input class="mono" value={a.username} oninput={(e) => onchange({ ...a, username: e.currentTarget.value })} /></label>
    <label><span>Contraseña</span><input class="mono" value={a.password} oninput={(e) => onchange({ ...a, password: e.currentTarget.value })} placeholder="{'{{password}}'}" /></label>
  {:else if auth.type === "bearer"}
    {@const a = auth}
    <label><span>Token</span><input class="mono" value={a.token} oninput={(e) => onchange({ ...a, token: e.currentTarget.value })} placeholder="{'{{token}}'}" /></label>
  {:else if auth.type === "api_key"}
    {@const a = auth}
    <label><span>Clave</span><input class="mono" value={a.key} oninput={(e) => onchange({ ...a, key: e.currentTarget.value })} placeholder="X-API-Key" /></label>
    <label><span>Valor</span><input class="mono" value={a.value} oninput={(e) => onchange({ ...a, value: e.currentTarget.value })} placeholder="{'{{api_key}}'}" /></label>
    <label><span>Enviar en</span><select value={a.in} onchange={(e) => onchange({ ...a, in: e.currentTarget.value as "header" | "query" })}><option value="header">cabecera</option><option value="query">query</option></select></label>
  {:else if auth.type === "inherit" && inheritHint}
    <p class="muted hint">{inheritHint}</p>
  {/if}
  <p class="muted hint">Usa variables (<code>{"{{token}}"}</code>) y guarda los valores reales como secretos del entorno.</p>
</div>

<style>
  .form {
    display: flex;
    flex-direction: column;
    gap: 10px;
    max-width: 560px;
  }
  label {
    display: grid;
    grid-template-columns: 140px 1fr;
    align-items: center;
    gap: 10px;
  }
  label span {
    color: var(--text-muted);
    font-size: 12px;
  }
  input,
  select {
    font-size: 12px;
  }
  .hint {
    font-size: 12px;
    margin: 0;
  }
  code {
    font-family: var(--mono);
    font-size: 11.5px;
    background: var(--bg-active);
    padding: 1px 5px;
    border-radius: 4px;
  }
</style>
