# Autenticación y TLS

La auth se define por solicitud (`[auth]`) o para toda la colección en
`harpoon.toml`; una solicitud con `type = "inherit"` (el valor por defecto) usa
la de la colección. Todos los valores admiten `{{variables}}`: guarda los
secretos en el entorno marcados como secretos (`*.secrets.toml`) y referéncialos.

| `type`      | Campos                                                     |
|-------------|------------------------------------------------------------|
| `none`      | —                                                          |
| `basic`     | `username`, `password`                                     |
| `bearer`    | `token`                                                    |
| `api_key`   | `key`, `value`, `in = "header" \| "query"`                 |
| `digest`    | `username`, `password`                                     |
| `oauth2`    | ver abajo                                                  |

## Digest (RFC 7616)

```toml
[auth]
type = "digest"
username = "{{user}}"
password = "{{password}}"
```

Harpoon envía la solicitud, recibe el `401` con `WWW-Authenticate: Digest`,
calcula la respuesta (MD5, MD5-sess, SHA-256, SHA-256-sess; con o sin `qop=auth`)
y reenvía. El tiempo mostrado suma ambas peticiones.

## OAuth 2.0

```toml
[auth]
type = "oauth2"
grant = "client_credentials"        # client_credentials | password | refresh_token
token_url = "https://auth.example.com/oauth/token"
client_id = "{{client_id}}"
client_secret = "{{client_secret}}"
scope = "read write"                # opcional
client_auth = "basic"               # basic (cabecera) | body (en el formulario)
header_prefix = "Bearer"            # opcional
# username / password   → grant = "password"
# refresh_token         → grant = "refresh_token"

[auth.extra]                        # parámetros extra del cuerpo, opcional
audience = "https://api.example.com"
```

- El token se cachea en `.harpoon/tokens.toml` (fuera de Git) hasta
  `expires_in` (con 30 s de margen). Si el servidor no indica caducidad, se
  reutiliza hasta recibir un `401`.
- Ante un `401` con token cacheado, Harpoon pide un token nuevo y reintenta
  **una** vez.
- Con `grant = "refresh_token"`, si el servidor devuelve un `refresh_token`
  nuevo se guarda y se usa en la siguiente renovación.
- Flujos interactivos (authorization code, PKCE, device) no están soportados:
  obtén el token una vez y pásalo como `bearer`, o usa `refresh_token`.

## TLS

En `[settings]` de `harpoon.toml` (rutas relativas a la colección, PEM):

```toml
[settings]
ca_cert = "certs/ca.pem"            # CA adicional para servidores internos
client_cert = "certs/client.pem"    # certificado de cliente (mTLS)
client_key = "certs/client-key.pem" # si la clave no va en client.pem
insecure = false                    # true = no verificar el servidor (solo dev)
```

Harpoon usa `rustls` con las raíces de Mozilla embebidas; no depende del
almacén de certificados del sistema. Un `insecure = true` se aplica a toda la
colección; úsalo solo en desarrollo.

## Ejecuciones para CI

```sh
harpoon run --env staging --junit report.xml          # informe JUnit
harpoon run users --iterations 3                       # repite 3 veces ({{iteration}})
harpoon run checkout --data casos.csv --bail           # una ejecución por fila
harpoon run --json > report.json                       # informe completo en JSON
```

Las columnas del CSV (o las claves de cada objeto del JSON) están disponibles
como `{{columna}}` durante su ejecución y no se persisten; lo que fijen los
scripts con `harpoon.vars.set` sí.
