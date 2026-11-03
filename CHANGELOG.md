# Cambios

## 0.0.1-alpha.1 — primera versión de prueba

Primera versión pública para probar Harpoon. Es una **alpha**: puede tener fallos
y el formato aún puede cambiar. Los binarios **no están firmados** (ver cómo
abrirlos en las notas de la versión).

### Núcleo y formato
- Colecciones en TOML versionables con Git: solicitudes, carpetas, entornos y variables.
- Herencia por carpeta en `folder.toml`: variables, cabeceras, auth, scripts y color.
- Secretos fuera de Git (`*.secrets.toml`, `HARPOON_<NOMBRE>`) y detector de secretos.
- Guardado que conserva comentarios, orden y formato de los TOML; escrituras atómicas.

### Solicitudes
- REST con todos los métodos y cuerpos (JSON, texto, XML, binario, formulario, multipart),
  GraphQL, WebSocket y SSE.
- Auth Basic, Bearer, API key, Digest y OAuth2 con caché de tokens; TLS, mTLS y proxy.
- Aserciones `[expect]` y scripts JS `pre` / `post` / `test` con QuickJS embebido.

### CLI
- `init`, `list`, `send`, `run` (JUnit, datos, iteraciones), `validate`, `check`,
  importación/exportación (Postman, OpenAPI, colecciones compartidas) y servidor `mock`.
- `harpoon mcp`: servidor MCP para Claude Code, Codex, Cursor… sin secretos; con
  `--allow-write` también crea carpetas y solicitudes.

### App de escritorio
- Árbol con arrastrar y soltar, pestañas con borradores, búsqueda global y atajos.
- Cabeceras heredadas y del runtime, edición en bloque, autocompletado de variables.
- Generar código en 7 lenguajes, validar la colección y panel de asistentes IA.
- 23 temas con modo automático, colores de carpeta y recarga al volver a la ventana.
