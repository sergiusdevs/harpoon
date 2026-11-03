# Harpoon para asistentes de IA (MCP)

`harpoon mcp` arranca un servidor [MCP](https://modelcontextprotocol.io) local
para que asistentes como **Claude Code**, **Codex**, Cursor o Zed conozcan tus
APIs: qué solicitudes hay, sus URLs, cabeceras, cuerpos y entornos.

**Nunca les da tus secretos.** Y sigue siendo offline: se comunica por stdio con
el asistente que lo lanza, sin abrir puertos ni conectarse a nada.

## Configuración

Sustituye la ruta por la de tu colección (o ejecuta el asistente desde ella y
omite `-C`). Con `--env` eliges el entorno por defecto.

**Claude Code**

```sh
claude mcp add harpoon -- harpoon mcp -C /ruta/a/mi-api
```

**Codex** (`~/.codex/config.toml`)

```toml
[mcp_servers.harpoon]
command = "harpoon"
args = ["mcp", "-C", "/ruta/a/mi-api"]
```

**Cursor, Zed, Claude Desktop y otros** (formato JSON habitual)

```json
{
  "mcpServers": {
    "harpoon": { "command": "harpoon", "args": ["mcp", "-C", "/ruta/a/mi-api"] }
  }
}
```

`harpoon` tiene que estar en el `PATH` (`cargo install --path crates/harpoon-cli`).

## Herramientas

Todas son de solo lectura y leen la colección del disco en cada llamada.

| Herramienta | Qué devuelve |
|---|---|
| `list_requests` | Solicitudes con clave, método y URL (`{{variables}}` sin resolver). Filtra por `folder` o `query` |
| `get_request` | Detalle de una solicitud: query, cabeceras (incluidas las de carpetas), auth efectiva y de dónde sale, cuerpo, scripts y docs |
| `list_environments` | Entornos y variables de colección y carpetas; las secretas aparecen con valor `null` |
| `resolve_url` | URL final en un entorno, con las secretas como `{{nombre}}` |
| `validate` | Lo mismo que `harpoon validate`: variables sin definir, URLs, JSON, archivos… |

Para crear o modificar solicitudes, el asistente edita los archivos `.toml` de la
colección directamente (están documentados en [format.md](format.md)); Harpoon
conserva sus comentarios y formato al guardar.

## Qué nunca sale

Harpoon lo garantiza al generar cada respuesta, no confía en el asistente:

- **Variables secretas** (de colección, carpeta o entorno): nunca se resuelven.
  Se ven como `{{api_key}}` o con valor `null`; su valor en `*.secrets.toml`,
  `.harpoon/secrets.toml` o `HARPOON_<NOMBRE>` no se lee para el asistente.
- **Valores escritos tal cual** en cabeceras, parámetros, auth o campos con
  nombre sensible (`Authorization`, `Cookie`, `token`, `password`, `api_key`…):
  se muestran como `‹oculto›`. Las plantillas sí se ven (`Bearer {{token}}`).
- **Credenciales con formato conocido** (tokens de GitHub, AWS, Stripe, JWT,
  claves privadas…) en cualquier texto, incluidos cuerpos y scripts.
- **Estado local:** variables de ejecución (`.harpoon/vars.toml`), tokens OAuth
  en caché, cookies e historial de respuestas.

## Ejemplos de uso

- «¿Qué endpoints de usuarios tiene la API y qué cabeceras necesitan?»
- «Genera un cliente TypeScript para las solicitudes de `orders/`.»
- «Añade una solicitud `DELETE /users/{id}` en `users/` siguiendo el estilo de las demás.»
- «¿Por qué falla `harpoon validate` en staging?»

## Próximamente

- `--allow-send`: enviar solicitudes desde el asistente (los secretos se resuelven
  dentro de Harpoon y la respuesta llega enmascarada).
- `--allow-write`: crear y editar solicitudes con el detector de secretos activo.
