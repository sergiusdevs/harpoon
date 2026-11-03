# Harpoon para asistentes de IA (MCP)

`harpoon mcp` arranca un servidor [MCP](https://modelcontextprotocol.io) local
para que asistentes como **Claude Code**, **Codex**, Cursor o Zed conozcan tus
APIs: qué solicitudes hay, sus URLs, cabeceras, cuerpos y entornos.

**Nunca les da tus secretos.** Y sigue siendo offline: se comunica por stdio con
el asistente que lo lanza, sin abrir puertos ni conectarse a nada.

## Configuración

**Desde la app:** pie de la barra lateral → **Asistentes IA (MCP)**. Muestra si la
CLI está instalada y si Harpoon ya está registrado en Claude Code o Codex, lo
registra con un botón para la colección abierta (con o sin escritura) y da la
configuración lista para copiar en el resto de asistentes.

**A mano:** sustituye la ruta por la de tu colección (o ejecuta el asistente desde ella y
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

Leen la colección del disco en cada llamada. Por defecto son de solo lectura.

| Herramienta | Qué devuelve |
|---|---|
| `list_requests` | Solicitudes con clave, método y URL (`{{variables}}` sin resolver). Filtra por `folder` o `query` |
| `get_request` | Detalle de una solicitud: query, cabeceras (incluidas las de carpetas), auth efectiva y de dónde sale, cuerpo, scripts y docs |
| `list_environments` | Entornos y variables de colección y carpetas; las secretas aparecen con valor `null` |
| `resolve_url` | URL final en un entorno, con las secretas como `{{nombre}}` |
| `validate` | Lo mismo que `harpoon validate`: variables sin definir, URLs, JSON, archivos… |

### Escritura (`--allow-write`)

Con `harpoon mcp --allow-write` el asistente también puede crear:

| Herramienta | Qué hace |
|---|---|
| `save_folder` | Crea una carpeta (y sus padres) y añade variables y cabeceras que heredan sus solicitudes |
| `save_request` | Crea una solicitud: método, URL con `{{variables}}`, query, cabeceras, cuerpo (JSON, texto, XML, GraphQL, formulario), docs y status esperado. No sobrescribe salvo `overwrite: true` |

Ambas **rechazan credenciales escritas tal cual** (`Authorization: Bearer abc…`,
`api_key=…`, tokens con formato conocido) y piden usar `{{variable}}`, que luego
marcas como secreta en Harpoon. Las plantillas como `Bearer {{token}}` sí se aceptan.
Lo que se guarda pasa por el mismo formato que la app: conserva comentarios y es
atómico.

```sh
claude mcp add harpoon -- harpoon mcp --allow-write -C ~/mi-api
```

Ejemplo real: con solo dos herramientas, `save_folder` y `save_request`, un asistente
registra la API pública de Rick and Morty (14 solicitudes en `rick-and-morty/`:
personajes, ubicaciones, episodios y GraphQL, con la URL base en la carpeta) y
`harpoon run rick-and-morty` las pasa todas.

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
- «Registra en Harpoon la API de Rick and Morty, organizada por recursos.» (con `--allow-write`)
- «Añade una solicitud `DELETE /users/{id}` en `users/` siguiendo el estilo de las demás.» (con `--allow-write`)
- «¿Por qué falla `harpoon validate` en staging?»

## Próximamente

- `--allow-send`: enviar solicitudes desde el asistente (los secretos se resuelven
  dentro de Harpoon y la respuesta llega enmascarada).
- Editar auth y scripts por MCP (hoy se hace en la app).
