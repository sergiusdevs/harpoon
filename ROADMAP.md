# Roadmap de Harpoon

Plan por fases. Cada fase termina con algo usable y publicable como release.
Las decisiones abiertas están marcadas con ❓ y se cierran al empezar la fase.

## Qué cambia respecto al documento de partida

El documento original describe un producto con nube opcional, IA, SSO y planes.
Harpoon recorta y reordena para ser **totalmente offline**:

| Apartado original | En Harpoon |
|---|---|
| Asistencia de IA (15) | **Fuera.** Requiere proveedor externo. Si algún día entra, será solo contra un modelo local y como crate opcional. |
| Gestión de equipo, SSO, SCIM, auditoría, planes (16) | **Fuera.** La colaboración es Git. No hay cuentas ni ediciones. |
| Gestores de secretos externos: Vault, GCP (14) | **Opcional y tardío.** Base: archivos `*.secrets.toml` ignorados por Git + variables de entorno + llavero del sistema operativo. |
| Scripts en JavaScript (5, 6) | **Motor embebido, sin Node.** ❓ QuickJS (`rquickjs`, JS familiar) o Rhai (puro Rust, más simple). |
| Documentación HTML (11) | Generada como **un único HTML estático sin CDNs** (CSS/JS embebidos). |
| Extensión de editor (13) | Se pospone. La CLI es la integración universal; la extensión solo la invoca. |
| Funciones Git dentro de la app (9) | La app no reimplementa Git; muestra estado y delega en el `git` del sistema. |
| OpenAPI (10) | Import y export sin límites de plan. |
| Servidores mock (12) | Locales, embebidos en el binario (`harpoon mock`). |

## Fase 0 — Base del proyecto ✅

- Workspace de Cargo, CI, lints, licencia.
- `harpoon-core` con modelo, formato TOML y motor de variables (`{{var}}`).
- `harpoon init` y `harpoon list`.
- [docs/format.md](docs/format.md) y [ADR-001 offline](docs/adr/001-offline.md).

## Fase 1 — Núcleo completo

- ✅ Herencia de variables, cabeceras, `auth` y scripts por carpeta (`requests/<carpeta>/folder.toml`).
- Tipos de variable (texto, número, booleano, objeto/JSON) y funciones dinámicas
  (`{{$uuid}}`, `{{$timestamp}}`, `{{$randomInt}}`).
- Secretos: `*.secrets.toml`, `HARPOON_<VAR>`, llavero del SO (`keyring`).
- ✅ Escritura del formato preservando comentarios y orden (`toml_edit`).
- ✅ `harpoon validate` (valida la colección sin enviar nada; también en la app).

## Fase 2 — REST + CLI para CI ✅

Hecho: `harpoon-http` (reqwest + rustls), auth Basic/Bearer/API key/Digest/OAuth2
(client credentials, password, refresh token; caché en `.harpoon/tokens.toml`),
`[settings]` (timeout, redirecciones, proxy, insecure, CA propia, mTLS), `[expect]`,
`harpoon send` y `harpoon run` con `--json`, `--junit`, `--iterations`, `--data`,
`--bail` y códigos de salida; variables dinámicas `{{$uuid}}`, `{{$timestamp}}`,
`{{$isoTimestamp}}`, `{{$randomInt a b}}`, `{{$randomHex n}}`. Pendiente: historial
local de ejecuciones y reporter HTML (van con la app).

- `harpoon-http` sobre `reqwest` + `rustls` (sin OpenSSL del sistema).
- OAuth 1.0: fuera de alcance por ahora.
- Reporter HTML estático: pendiente.
- Historial local de ejecuciones (`.harpoon/history/`, ignorado por Git).

## Fase 3 — Scripts y tests ✅

Hecho: `harpoon-script` con QuickJS (`rquickjs`), límites de memoria/tiempo,
sin E/S; `pre`/`post`/`test` en solicitud y colección; `expect` estilo Chai;
variables de ejecución persistentes en `.harpoon/vars.toml`; integrado en CLI y
app (pestañas Scripts/Tests, Consola); scripts por carpeta en `folder.toml`.

Diseño original:

- Motor de scripts embebido (QuickJS) con API `harpoon.req`, `harpoon.res`,
  `harpoon.vars`, `harpoon.env`, `test(name, fn)`, `expect(x).to...` estilo Chai.
- Scripts `pre`/`post`/`test` en solicitud, carpeta y colección; orden de ejecución documentado.
- Sandbox: sin acceso a red ni a disco desde scripts, salvo lo que exponga la API.
- Aserciones declarativas sin código para los casos comunes (`[[assert]] status = 200`).

## Fase 4 — TUI (`harpoon-tui`, ratatui)

- Árbol de colección, editor de solicitud, panel de respuesta con resaltado y búsqueda JSON.
- Cambio de entorno, ejecución de carpetas, resultados de tests en vivo.
- Todo con teclado; funciona por SSH.

## Fase 5 — Más protocolos

- **GraphQL:** consultas, variables, introspección contra el servidor y esquemas `.graphql` locales; autocompletado.
- **WebSocket:** sesión interactiva y guiones (`tokio-tungstenite`).
- **gRPC:** `.proto` locales y reflexión; unary y streaming (`tonic`, `prost`).

## Fase 6 — Ciclo de vida de la API

- Import/export OpenAPI 3.x (y Postman/`.http` como importadores).
- `harpoon docs` → HTML estático autocontenido con ejemplos de petición/respuesta.
- `harpoon mock` → servidor local a partir de ejemplos guardados o de una spec OpenAPI.

## Fase 7 — Escritorio y editor

- ❓ GUI nativa: `egui`/`iced` (un binario, puro Rust) frente a `tauri` (webview, más UI).
- Extensión de VS Code fina que invoca la CLI y muestra respuestas.
- Paquetes: Homebrew, `cargo binstall`, winget, AppImage/`.deb`.

## No objetivos

- Sincronización en la nube, cuentas de usuario, telemetría.
- Gestión de licencias o funciones "enterprise".
- Reimplementar Git.

## IA sin secretos (MCP)

- ✅ `harpoon mcp`: servidor MCP local de solo lectura (listar, ver, resolver URLs,
  validar) con los secretos enmascarados por Harpoon.
- `--allow-send`: enviar solicitudes desde el asistente con respuestas enmascaradas.
- ✅ `--allow-write`: crear carpetas y solicitudes con el detector de secretos activo.
- Editar auth y scripts por MCP.
