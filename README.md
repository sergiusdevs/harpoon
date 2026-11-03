# Harpoon

**Cliente de APIs 100 % offline, escrito en Rust.** Colecciones en texto plano que
viven junto a tu código, se versionan con Git y funcionan igual en la app de
escritorio, en la terminal y en CI.

Sin cuenta. Sin nube. Sin telemetría.

<a href="https://paypal.me/sergyvegas"><img src="https://img.shields.io/badge/%E2%98%95%20Buy%20me%20a%20coffee-ffdd00?style=for-the-badge&logo=paypal&logoColor=000" alt="Buy me a coffee (PayPal)"></a>

![Harpoon](docs/img/desktop.png)

## Por qué Harpoon

- **Offline por diseño.** El binario solo abre las conexiones que pides tú
  ([ADR-001](docs/adr/001-offline.md)).
- **Texto plano y Git.** Cada solicitud es un archivo TOML legible y fácil de revisar en
  un diff ([formato](docs/format.md)). Los secretos nunca entran en el repositorio
  ([secretos](docs/secrets.md)).
- **Un solo binario.** El motor de scripts (QuickJS) y el servidor mock van embebidos:
  nada de Node ni runtimes externos.
- **CLI primero.** Todo lo que hace la app se puede hacer con `harpoon` desde la terminal o en CI.
- **Listo para asistentes de IA.** Claude Code, Codex o Cursor pueden consultar tus APIs
  por MCP sin ver nunca un secreto ([MCP](docs/mcp.md)).

## Tus APIs en Claude Code y Codex, sin tus secretos

`harpoon mcp` es un servidor [MCP](docs/mcp.md) local: el asistente puede listar tus
solicitudes, ver sus cabeceras, cuerpos y auth, resolver URLs por entorno y validar
la colección. Las variables secretas se ven como `{{api_key}}` y los valores sensibles
como `‹oculto›`; Harpoon lo garantiza antes de responder. Sin puertos ni red: va por stdio.

```sh
claude mcp add harpoon -- harpoon mcp -C ~/mi-api
```

```toml
# ~/.codex/config.toml
[mcp_servers.harpoon]
command = "harpoon"
args = ["mcp", "-C", "/Users/tu-usuario/mi-api"]
```

## Qué incluye

| | |
|---|---|
| **Protocolos** | REST (todos los métodos; JSON, texto, XML, binario, formulario y multipart), GraphQL, WebSocket y SSE |
| **Auth** | Basic, Bearer, API key, Digest y OAuth2 (client credentials, password, refresh token) con caché de tokens |
| **Red** | TLS con rustls, CA propia, mTLS, proxy con `NO_PROXY`, timeouts y redirecciones |
| **Carpetas** | Cada carpeta puede definir variables, cabeceras, auth y scripts que heredan sus solicitudes y subcarpetas |
| **Variables** | Por colección, carpeta, entorno y ejecución; dinámicas (`{{$uuid}}`, `{{$timestamp}}`, `{{$randomInt 1 10}}`…) |
| **Pruebas** | Aserciones declarativas `[expect]` y scripts JS `pre` / `post` / `test` con `expect` estilo Chai |
| **Secretos** | Fuera de Git (`*.secrets.toml`), por variable de entorno `HARPOON_<NOMBRE>` y detector de secretos al guardar |
| **Importar / exportar** | Postman (con auth y scripts de carpeta), OpenAPI 3, cURL y colecciones compartidas de Harpoon |
| **Generar código** | cURL, JavaScript (fetch y axios), Python, Go, Java y C#, con variables, auth y carpetas ya resueltas |
| **Validar** | `harpoon validate` y en la app: variables sin definir, URLs, JSON, archivos y secretos sin valor |
| **Git** | Guardar conserva comentarios, orden y formato de los TOML: diffs mínimos. Escrituras atómicas |
| **IA (MCP)** | `harpoon mcp` para Claude Code, Codex, Cursor… solo lectura y sin secretos |
| **Extras** | Servidor mock local, historial de respuestas, cookies por colección y runner de colecciones |

## Instalación

Requisitos: Rust estable 1.88+. Para la app de escritorio, además Node 20+ y, en macOS,
las Xcode Command Line Tools (`xcode-select --install`).

```sh
# CLI
cargo install --path crates/harpoon-cli

# App de escritorio
cd apps/desktop
npm install
npm run tauri dev      # desarrollo con recarga en caliente
npm run tauri build    # genera el .app / instalador en target/release/bundle
```

Para probar solo la interfaz, sin Rust, `npm run dev` la abre en el navegador con datos de ejemplo.

## Empezar

```sh
harpoon init mi-api        # crea harpoon.toml, un entorno y una solicitud de ejemplo
cd mi-api
harpoon list               # solicitudes con las URLs ya resueltas
harpoon send health -v     # envía una y muestra petición y respuesta
harpoon run                # ejecuta toda la colección con sus comprobaciones
```

Una solicitud es un archivo como `requests/users/create.toml`:

```toml
name = "Crear usuario"
method = "POST"
url = "{{base_url}}/users"

[body]
type = "json"
json = '{ "name": "Ada" }'

[expect]
status = 201
max_time_ms = 500

[scripts]
post = 'harpoon.vars.set("user_id", harpoon.res.json().id);'
test = 'test("devuelve un id", () => expect(harpoon.res.json()).to.have.property("id"));'
```

### En CI

```sh
harpoon run --env staging --bail --junit informe.xml   # se detiene al primer fallo
harpoon run --data casos.csv                           # una ejecución por fila
harpoon validate                                       # variables sin definir, URLs, JSON, archivos…
harpoon check                                          # busca secretos en archivos versionados
```

Códigos de salida: `0` todo correcto, `1` alguna comprobación falló, `2` error de uso o de red.

### Más comandos

```sh
harpoon postman-import api.postman_collection.json ./api
harpoon openapi-import openapi.yaml ./api
harpoon share-export api.harpoon.json      # colección completa, sin estado local ni secretos
harpoon mock --port 8787                   # respuestas de mocks.toml, solo en localhost
harpoon mcp                                # servidor MCP para asistentes de IA (ver docs/mcp.md)
```

## App de escritorio

Tauri 2 + Svelte 5, con 23 temas (entre ellos Catppuccin, Gruvbox, Solarized, Rosé Pine,
GitHub, One Dark, Kanagawa y Tokyo Night) y modo automático que sigue al sistema.

- Árbol de la colección con arrastrar y soltar, menú contextual, búsqueda global (`⌘K`) y teclado completo.
- Pestañas de solicitudes con borradores independientes y aviso de cambios sin guardar.
- Selector de entornos con buscador y estado de Git de la colección en la barra de título.
- Ajustes por carpeta (variables, cabeceras, auth y scripts) con lo heredado a la vista.
- Cabeceras heredadas y del runtime visibles en cada solicitud; edición en bloque con `Nombre: valor`.
- Autocompletado de `{{variables}}` con su origen y aviso de las que no existen, también al crear solicitudes.
- Generar código en 7 lenguajes y validar la colección desde su menú.
- Respuesta con JSON plegable, búsqueda, vista HTML aislada, imágenes, cookies y lo que se envió de verdad.
- Historial de respuestas con comparación, consola de scripts y panel de red.
- Guardado atómico: un cierre inesperado nunca deja un archivo a medias.

Atajos principales: `⌘↩` enviar · `⌘S` guardar · `⌘N` nueva solicitud · `⌘E` entornos · `⌘K` buscar · `⌘,` ajustes.

## Documentación

| Documento | Contenido |
|---|---|
| [Formato](docs/format.md) | `harpoon.toml`, entornos, solicitudes y variables |
| [Scripts](docs/scripting.md) | API `harpoon.req` / `res` / `vars`, `test` y `expect` |
| [Auth](docs/auth.md) | Todos los tipos de auth, mTLS y uso en CI |
| [Secretos](docs/secrets.md) | Dónde vive cada valor y cómo trabajar con Git |
| [MCP](docs/mcp.md) | Conectar Claude Code, Codex, Cursor… y qué nunca se comparte |
| [Roadmap](ROADMAP.md) | Lo que viene: llavero del sistema, gRPC, docs HTML, distribución |

## Desarrollo

```
crates/
  harpoon-core/    modelo, formato en disco, variables y secretos
  harpoon-http/    ejecución HTTP, auth, TLS y aserciones
  harpoon-script/  motor JS embebido (QuickJS)
  harpoon-mock/    servidor mock local
  harpoon-cli/     binario `harpoon`
apps/desktop/      app de escritorio (src-tauri + Svelte)
docs/              documentación y ADRs
examples/          colecciones de ejemplo
```

```sh
cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace
cd apps/desktop && npm run check && npm test && npm run build
```

El CI ejecuta todo lo anterior en Linux, macOS y Windows.

## Apoya el proyecto

Harpoon es gratuito y de código abierto. Si te ahorra tiempo, puedes invitarme a un café ☕

<a href="https://paypal.me/sergyvegas"><img src="https://img.shields.io/badge/%E2%98%95%20Buy%20me%20a%20coffee-ffdd00?style=for-the-badge&logo=paypal&logoColor=000" alt="Buy me a coffee (PayPal)" height="48"></a>

## Licencia

[MIT](LICENSE)
