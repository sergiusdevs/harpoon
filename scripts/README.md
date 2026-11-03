# Backend local y pruebas de integración

Desde la raíz del repositorio, con Node 22 o superior y Rust:

```sh
cargo build -p harpoon
npm --prefix apps/desktop run test:integration
```

La batería arranca su propio servidor en un puerto libre de `127.0.0.1`, crea
colecciones temporales y las elimina al terminar. Usa el binario real de Harpoon
para métodos HTTP, CRUD JSON, validación, Bearer, Basic, OAuth2, cookies, formularios,
archivos multipart, GraphQL, redirecciones, timeout, aserciones y runner mixto.
También comprueba el eco WebSocket y eventos SSE del backend con clientes de Node.
El mock GraphQL cubre el transporte y las variables; no implementa un motor de esquemas.

Para usarlo manualmente desde la app:

```sh
npm --prefix apps/desktop run test:backend
```

Abre la colección `examples/local-api` en Harpoon. El servidor escucha solo en
`127.0.0.1:8788`; se puede cambiar el puerto con `PORT` (ajusta también el entorno
de la colección). Ejecuta primero Login para los ejemplos de Bearer y cookie.
El runner omite WS/SSE con un aviso; ábrelas individualmente con Conectar.

Endpoints: `/health`, `/echo`, `/users`, `/users/:id`, `/login`, `/private`,
`/session`, `/basic`, `/oauth/token`, `/redirect`, `/slow`, `/graphql`, `/socket`
(WebSocket) y `/events` (SSE). Todos los datos y credenciales son ficticios.

Pruebas del gestor de conexiones y del backend Rust:

```sh
npm --prefix apps/desktop test
cargo test --workspace
```

## Spring Boot real

Consulta [el backend Spring](../examples/spring-api/README.md). Tras compilar su
JAR con Maven, `npm --prefix apps/desktop run test:spring` levanta Spring en un
puerto libre y comprueba MVC, validación, Spring Security (Basic y sesión con
CSRF), multipart y GraphQL con esquema real.
