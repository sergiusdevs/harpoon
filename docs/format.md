# Formato de colección (v1)

Una colección es un directorio. Todo son archivos TOML, legibles y diffeables.

```
mi-api/
├── arpon.toml                  # metadatos, variables y auth de la colección
├── .gitignore                  # environments/*.secrets.toml
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

## `arpon.toml`

```toml
name = "Mi API"
format = 1                      # versión del formato, para migraciones
default_environment = "dev"

[vars]
api_version = "v1"

[auth]                          # auth por defecto de toda la colección
type = "bearer"
token = "{{token}}"
```

## `environments/<nombre>.toml`

```toml
secrets = ["token"]             # documenta qué variables son secretas

[vars]
base_url = "https://api.dev.example.com"
```

`environments/<nombre>.secrets.toml` tiene la misma forma y **sobrescribe** `vars`.
Alternativa sin archivo: variable de entorno `ARPON_TOKEN` (fase 1).

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

[auth]                          # inherit (por defecto) | none | basic | bearer | api_key
type = "inherit"

[body]                          # none | json | text | form
type = "json"
json = '''
{ "name": "Ada", "email": "ada@example.com" }
'''

[scripts]                       # fase 3
pre = "arpon.vars.set('ts', Date.now())"
post = "arpon.vars.set('user_id', arpon.res.json().id)"
test = """
test('crea el usuario', () => {
  expect(arpon.res.status).to.equal(201);
});
"""
```

## Variables

- Sintaxis: `{{nombre}}` (espacios internos permitidos: `{{ nombre }}`).
- Prioridad, de mayor a menor: scripts en tiempo de ejecución → entorno → carpeta (fase 1) → colección.
- Una variable sin definir es un **error**, no una cadena vacía.
- `{{` sin cerrar se deja literal.
- Fase 1: funciones dinámicas `{{$uuid}}`, `{{$timestamp}}`, `{{$randomInt 1 100}}`.

## Reglas de compatibilidad

- `format` se incrementa solo en cambios incompatibles; `arpon` migra con `arpon migrate`.
- Claves desconocidas se conservan al reescribir (fase 1, `toml_edit`).
