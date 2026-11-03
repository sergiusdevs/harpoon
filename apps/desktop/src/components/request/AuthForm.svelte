<script lang="ts">
  // Formulario de autenticación, compartido por solicitud y colección.
  import { emptyOAuth2, type Auth } from "../../domain/types";
  import KeyValue from "../common/KeyValue.svelte";
  import SecretInput from "../secrets/SecretInput.svelte";
  import SaveAsSecret from "../secrets/SaveAsSecret.svelte";
  import { isLiteral } from "../../domain/secrets";

  interface Props {
    auth: Auth;
    onchange: (auth: Auth) => void;
    /** En la colección no tiene sentido "heredar". */
    allowInherit?: boolean;
    inheritHint?: string;
    /** Muestra "Guardar como secreto" junto a los campos con valor escrito tal cual (solo en la solicitud abierta). */
    secretActions?: boolean;
  }
  let { auth, onchange, allowInherit = true, inheritHint = "", secretActions = false }: Props = $props();

  function setKind(kind: Auth["type"]) {
    const a: Auth =
      kind === "basic" ? { type: "basic", username: "", password: "" }
      : kind === "bearer" ? { type: "bearer", token: "" }
      : kind === "api_key" ? { type: "api_key", key: "", value: "", in: "header" }
      : kind === "digest" ? { type: "digest", username: "", password: "" }
      : kind === "oauth2" ? emptyOAuth2()
      : kind === "inherit" ? { type: "inherit" }
      : { type: "none" };
    onchange(a);
  }
</script>

{#snippet lock(field: string, value: string)}
  {#if secretActions && isLiteral(value)}<SaveAsSecret location={`auth.${field}`} />{/if}
{/snippet}

<div class="form">
  <label>
    <span>Tipo</span>
    <select value={auth.type} onchange={(e) => setKind(e.currentTarget.value as Auth["type"])}>
      {#if allowInherit}<option value="inherit">Heredar</option>{/if}
      <option value="none">Ninguna</option>
      <option value="basic">Basic</option>
      <option value="bearer">Bearer</option>
      <option value="api_key">API key</option>
      <option value="digest">Digest</option>
      <option value="oauth2">OAuth 2.0</option>
    </select>
  </label>
  {#if auth.type === "basic"}
    {@const a = auth}
    <label><span>Usuario</span><input class="mono" value={a.username} oninput={(e) => onchange({ ...a, username: e.currentTarget.value })} /></label>
    <label><span>Contraseña</span><span class="field"><SecretInput inputClass="mono" label="Contraseña" value={a.password} oninput={(v) => onchange({ ...a, password: v })} placeholder={"{{password}}"} />{@render lock("password", a.password)}</span></label>
  {:else if auth.type === "bearer"}
    {@const a = auth}
    <label><span>Token</span><span class="field"><input class="mono" value={a.token} oninput={(e) => onchange({ ...a, token: e.currentTarget.value })} placeholder="{'{{token}}'}" />{@render lock("token", a.token)}</span></label>
  {:else if auth.type === "api_key"}
    {@const a = auth}
    <label><span>Clave</span><input class="mono" value={a.key} oninput={(e) => onchange({ ...a, key: e.currentTarget.value })} placeholder="X-API-Key" /></label>
    <label><span>Valor</span><span class="field"><input class="mono" value={a.value} oninput={(e) => onchange({ ...a, value: e.currentTarget.value })} placeholder="{'{{api_key}}'}" />{@render lock("value", a.value)}</span></label>
    <label><span>Enviar en</span><select value={a.in} onchange={(e) => onchange({ ...a, in: e.currentTarget.value as "header" | "query" })}><option value="header">cabecera</option><option value="query">query</option></select></label>
  {:else if auth.type === "digest"}
    {@const a = auth}
    <label><span>Usuario</span><input class="mono" value={a.username} oninput={(e) => onchange({ ...a, username: e.currentTarget.value })} /></label>
    <label><span>Contraseña</span><span class="field"><SecretInput inputClass="mono" label="Contraseña" value={a.password} oninput={(v) => onchange({ ...a, password: v })} placeholder={"{{password}}"} />{@render lock("password", a.password)}</span></label>
    <p class="muted hint">Responde al reto <code>401 WWW-Authenticate: Digest</code> (MD5 o SHA-256, con o sin <code>qop</code>).</p>
  {:else if auth.type === "oauth2"}
    {@const a = auth}
    <label>
      <span>Flujo (grant)</span>
      <select value={a.grant} onchange={(e) => onchange({ ...a, grant: e.currentTarget.value as typeof a.grant })}>
        <option value="client_credentials">Client credentials</option>
        <option value="password">Password (usuario y contraseña)</option>
        <option value="refresh_token">Refresh token</option>
      </select>
    </label>
    <label><span>Token URL</span><input class="mono" value={a.token_url} oninput={(e) => onchange({ ...a, token_url: e.currentTarget.value })} placeholder="https://auth.example.com/oauth/token" /></label>
    <label><span>Client ID</span><input class="mono" value={a.client_id} oninput={(e) => onchange({ ...a, client_id: e.currentTarget.value })} /></label>
    <label><span>Client secret</span><span class="field"><SecretInput inputClass="mono" label="Client secret" value={a.client_secret} oninput={(v) => onchange({ ...a, client_secret: v })} placeholder={"{{client_secret}}"} />{@render lock("client_secret", a.client_secret)}</span></label>
    <label><span>Scope</span><input class="mono" value={a.scope} oninput={(e) => onchange({ ...a, scope: e.currentTarget.value })} placeholder="read write" /></label>
    {#if a.grant === "password"}
      <label><span>Usuario</span><input class="mono" value={a.username} oninput={(e) => onchange({ ...a, username: e.currentTarget.value })} /></label>
      <label><span>Contraseña</span><span class="field"><SecretInput inputClass="mono" label="Contraseña" value={a.password} oninput={(v) => onchange({ ...a, password: v })} />{@render lock("password", a.password)}</span></label>
    {:else if a.grant === "refresh_token"}
      <label><span>Refresh token</span><span class="field"><SecretInput inputClass="mono" label="Refresh token" value={a.refresh_token} oninput={(v) => onchange({ ...a, refresh_token: v })} placeholder={"{{refresh_token}}"} />{@render lock("refresh_token", a.refresh_token)}</span></label>
    {/if}
    <label>
      <span>Credenciales de cliente</span>
      <select value={a.client_auth} onchange={(e) => onchange({ ...a, client_auth: e.currentTarget.value as "basic" | "body" })}>
        <option value="basic">Cabecera Authorization: Basic</option>
        <option value="body">En el cuerpo (client_id / client_secret)</option>
      </select>
    </label>
    <label><span>Prefijo de cabecera</span><input class="mono" value={a.header_prefix} oninput={(e) => onchange({ ...a, header_prefix: e.currentTarget.value })} /></label>
    <div class="sub">Parámetros extra del token (audience, resource…)</div>
    <KeyValue entries={a.extra} onchange={(extra) => onchange({ ...a, extra })} keyPlaceholder="audience" />
    <p class="muted hint">El token se guarda en <code>.harpoon/tokens.toml</code> (fuera de Git) hasta que caduca; si el servidor devuelve 401, se renueva y reintenta una vez.</p>
  {:else if auth.type === "inherit" && inheritHint}
    <p class="muted hint">{inheritHint}</p>
  {/if}
  <p class="muted hint">Usa variables (<code>{"{{token}}"}</code>) y guarda los valores reales como {secretActions ? "secretos (el candado lo hace por ti)" : "secretos"}: en Git solo queda el nombre.</p>
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
  label > span:first-child {
    color: var(--text-muted);
    font-size: 12px;
  }
  .field {
    display: flex;
    align-items: center;
    gap: 4px;
    min-width: 0;
  }
  .field > input {
    flex: 1;
    min-width: 0;
  }
  input,
  select {
    font-size: 12px;
  }
  .hint {
    font-size: 12px;
    margin: 0;
  }
  .sub {
    color: var(--text-muted);
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    margin-top: 4px;
  }
  code {
    font-family: var(--mono);
    font-size: 11.5px;
    background: var(--bg-active);
    padding: 1px 5px;
    border-radius: 4px;
  }
</style>
