# Formato de colección (v1)

Una colección es un directorio. Todo son archivos TOML, legibles y diffeables.

```
mi-api/
├── harpoon.toml                  # metadatos, variables y auth de la colección
├── .gitignore                  # environments/*.secrets.toml, .harpoon/
├── .harpoon/vars.toml          # variables de ejecución (NO se versiona)
├── environments/
│   ├── dev.toml
│   ├── dev.secrets.toml        # NO se versiona; se fusiona sobre dev.toml
│   └── prod.toml
└── requests/
    ├── health.toml
    └── users/                  # las carpetas organizan y (fase 1) heredan config
        ├── folder.toml         # (fase 1) auth/cabeceras/vars de la carpeta
        ├── list.toml
        └── create.toml
```

El identificador de una solicitud es su ruta relativa sin extensión: `users/list`.

## `harpoon.toml`

```toml
name = "Mi API"
format = 1                      # versión del formato, para migraciones
default_environment = "dev"

[vars]
api_version = "v1"

[auth]                          # auth por defecto de toda la colección
type = "bearer"
token = "{{token}}"

[scripts]                       # opcional: se ejecutan en todas las solicitudes
test = 'test("json", () => expect(harpoon.res.header("content-type")).to.include("json"));'

[settings]                      # opcional; estos son los valores por defecto
timeout_ms = 30000
follow_redirects = true
insecure = false                # true = no verificar TLS (solo desarrollo)
# ca_cert = "certs/ca.pem"           # CA adicional; client_cert / client_key para mTLS
# proxy = "http://127.0.0.1:8888"   # si falta, se respetan HTTP(S)_PROXY / NO_PROXY
```

## `environments/<nombre>.toml`

```toml
secrets = ["token"]             # documenta qué variables son secretas

[vars]
base_url = "https://api.dev.example.com"
```

`environments/<nombre>.secrets.toml` tiene la misma forma y **sobrescribe** `vars`.
Alternativa sin archivo: variable de entorno `HARPOON_TOKEN` (fase 1).

## `requests/**/<nombre>.toml`

```toml
name = "Crear usuario"
method = "POST"                 # GET | POST | PUT | PATCH | DELETE | HEAD | OPTIONS
url = "{{base_url}}/{{api_version}}/users"
docs = """
Crea un usuario. Devuelve 201 con el recurso creado.
"""

[headers]
Content-Type = "application/json"

[query]
notify = "true"

[auth]                          # inherit (por defecto) | none | basic | bearer | api_key | digest | oauth2 (ver docs/auth.md)
type = "inherit"

[body]                          # none | json | text | form
type = "json"
json = '''
{ "name": "Ada", "email": "ada@example.com" }
'''

[expect]                        # aserciones declarativas; todas opcionales
status = 201
headers = { Content-Type = "application/json" }
body_contains = ["\"id\""]
max_time_ms = 500

[scripts]                       # ver docs/scripting.md
pre = "harpoon.vars.set('ts', Date.now())"
post = "harpoon.vars.set('user_id', harpoon.res.json().id)"
test = """
test('crea el usuario', () => {
  expect(harpoon.res.status).to.equal(201);
});
"""
```

## Variables

- Sintaxis: `{{nombre}}` (espacios internos permitidos: `{{ nombre }}`).
- Prioridad, de mayor a menor: variables de ejecución (`harpoon.vars`) → entorno → carpeta (fase 1) → colección.
- Una variable sin definir es un **error**, no una cadena vacía.
- `{{` sin cerrar se deja literal.
- Funciones dinámicas: `{{$uuid}}`, `{{$timestamp}}`, `{{$isoTimestamp}}`, `{{$randomInt 1 100}}`, `{{$randomHex 16}}`.

## Reglas de compatibilidad

- `format` se incrementa solo en cambios incompatibles; `harpoon` migra con `harpoon migrate`.
- Claves desconocidas se conservan al reescribir (fase 1, `toml_edit`).
