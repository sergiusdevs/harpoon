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

[body]                          # none | json | text | xml | binary | form | multipart
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


### XML y cuerpos binarios

Un cuerpo XML se define con `type = "xml"` y `xml = "<root/>"`; Harpoon
utiliza `application/xml` si no se indica otra cabecera. Para enviar los bytes
completos de un archivo, usa `type = "binary"` y `path = "fixtures/data.bin"`.
La ruta se resuelve desde la raíz de la colección y puede usar variables. El
archivo se transmite al enviar, sin copiarlo a la colección. El tipo por defecto
es `application/octet-stream` y se puede cambiar en las cabeceras.

### Formularios multipart y archivos

Para enviar campos y archivos en una misma solicitud:

```toml
[body]
type = "multipart"

[[body.parts]]
type = "text"
name = "description"
value = "Informe de {{project}}"

[[body.parts]]
type = "file"
name = "attachment"
path = "fixtures/report.pdf"
content_type = "application/pdf" # opcional; se deduce de la extensión
```

Las rutas relativas parten de la raíz de la colección; también se admiten rutas
absolutas y variables. Los archivos se leen al enviar y no se copian al TOML.
Se permite repetir nombres de campo. No definas una cabecera `Content-Type`
manual: Harpoon genera el límite multipart automáticamente. En los scripts,
`harpoon.req.body` no expone ni modifica las partes multipart.

## Variables

- Sintaxis: `{{nombre}}` (espacios internos permitidos: `{{ nombre }}`).
- Prioridad, de mayor a menor: variables de ejecución (`harpoon.vars`) → entorno → carpeta (fase 1) → colección.
- Una variable sin definir es un **error**, no una cadena vacía.
- `{{` sin cerrar se deja literal.
- Funciones dinámicas: `{{$uuid}}`, `{{$timestamp}}`, `{{$isoTimestamp}}`, `{{$randomInt 1 100}}`, `{{$randomHex 16}}`.

## Reglas de compatibilidad

- `format` se incrementa solo en cambios incompatibles; `harpoon` migra con `harpoon migrate`.
- Claves desconocidas se conservan al reescribir (fase 1, `toml_edit`).

### Variables por carpeta

Cada carpeta dentro de `requests/` puede incluir un `folder.toml`:

```toml
[vars]
api_path = "/api/v1/users"
```

Estas variables se aplican a las solicitudes de esa carpeta y sus descendientes.
La prioridad, de menor a mayor, es colección, carpetas desde la raíz hasta la
más cercana, entorno y variables de ejecución. Las variables de carpeta se
versionan con la colección; guarda los secretos en los archivos de entorno privados.
En la app se editan desde el menú contextual de la carpeta.
