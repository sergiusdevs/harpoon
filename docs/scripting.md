# Scripts y tests

Harpoon ejecuta JavaScript embebido (QuickJS) sin Node y sin acceso a red,
disco ni sistema: los scripts solo ven el objeto `harpoon` y las utilidades de
test. Límites: 64 MB de memoria y 5 s por script.

## Dónde se escriben

En cada solicitud (`[scripts]` del `.toml`, o pestañas **Scripts** / **Tests** en
la app) y en la colección (`[scripts]` de `harpoon.toml`, o **Ajustes → Scripts**).

```toml
[scripts]
pre = 'harpoon.req.headers["X-Request-Id"] = crypto.randomUUID?.() ?? String(Date.now());'
post = 'harpoon.vars.set("token", harpoon.res.json().token);'
test = '''
test("status 200", () => expect(harpoon.res.status).to.equal(200));
test("tiene usuario", () => expect(harpoon.res.json()).to.have.property("user"));
'''
```

## Orden de ejecución

```
pre (colección) → pre (solicitud) → interpolación {{vars}} → HTTP
  → post (colección) → post (solicitud)
  → test (colección) → test (solicitud)
  → aserciones [expect]
```

Un error en `pre` aborta el envío. Un error en `post`/`test` no: la respuesta se
muestra igual y el fallo aparece como un test llamado `script post` (o `script test`).

## API

### `harpoon.req` (en `pre`: modificable; en `post`/`test`: lo que se envió)

| Campo     | Tipo                     |
|-----------|--------------------------|
| `method`  | `"GET"`, `"POST"`, …     |
| `url`     | string (en `pre`, todavía con `{{variables}}`) |
| `headers` | objeto clave → valor     |
| `query`   | objeto clave → valor     |
| `body`    | string o `undefined`     |

Los cuerpos `form` y `multipart` no se exponen ni se modifican mediante `harpoon.req.body`.

### `harpoon.res` (solo `post` y `test`)

| Campo / método    | Descripción                               |
|-------------------|-------------------------------------------|
| `status`          | código HTTP                               |
| `headers`         | objeto (claves en minúsculas)             |
| `header(nombre)`  | cabecera sin distinguir mayúsculas        |
| `body`            | cuerpo como texto                         |
| `text()`          | igual que `body`                          |
| `json()`          | cuerpo parseado (lanza error si no es JSON) |
| `elapsed_ms`      | tiempo de respuesta                       |
| `size`            | bytes                                     |

### `harpoon.vars` — variables de ejecución

Persisten entre solicitudes y entre ejecuciones en `.harpoon/vars.toml`
(fuera de Git). Tienen la **máxima prioridad** al interpolar `{{nombre}}`.

```js
harpoon.vars.set("token", valor);   // objetos → JSON
harpoon.vars.get("token");
harpoon.vars.has("token");
harpoon.vars.unset("token");
harpoon.vars.all();
```

### `harpoon.env.get(nombre)`

Lectura de las variables resueltas de colección + entorno.

### `console.log(...)`

Se muestra en la pestaña **Consola** de la app y en `harpoon send`.

### `test(nombre, fn)` y `expect(valor)`

`expect` sigue el estilo de Chai. Encadenadores sin efecto: `to`, `be`, `been`,
`is`, `that`, `and`, `has`, `have`, `with`, `a`, `an`. `not` niega.

| Aserción                                | Ejemplo |
|-----------------------------------------|---------|
| `equal(v)` / `eq`                       | `expect(res.status).to.equal(200)` |
| `eql(v)` / `deep.equal(v)`              | `expect(obj).to.deep.equal({a: 1})` |
| `ok` / `true` / `false` / `null` / `undefined` / `exist` / `empty` | `expect(x).to.be.ok` |
| `a(tipo)` / `an(tipo)`                  | `expect(id).to.be.a("number")` (`"array"`, `"null"`, …) |
| `include(v)` / `contain(v)` / `string(s)` | `expect(tags).to.include("dog")` |
| `property(nombre[, valor])`             | `expect(body).to.have.property("id", 42)` |
| `keys(...)`                             | `expect(body).to.have.keys("id", "name")` |
| `length(n)` / `lengthOf(n)`             | `expect(items).to.have.length(3)` |
| `above(n)` / `below(n)` / `least(n)` / `most(n)` / `within(a, b)` | `expect(ms).to.be.below(500)` |
| `match(regex)`                          | `expect(email).to.match(/@/)` |
| `oneOf([...])`                          | `expect(status).to.be.oneOf([200, 201])` |
| `throw()`                               | `expect(() => JSON.parse("x")).to.throw()` |

También existen `assert.ok/equal/deepEqual/fail` y el alias `pm` (`pm.test`,
`pm.expect`, `pm.response`, `pm.variables`) para scripts traídos de Postman.

## Ejemplo: login y uso del token

`requests/auth/login.toml`

```toml
name = "Login"
method = "POST"
url = "{{base_url}}/login"

[auth]
type = "none"

[body]
type = "json"
json = '{ "user": "ada", "password": "{{password}}" }'

[scripts]
post = 'harpoon.vars.set("token", harpoon.res.json().token);'
test = 'test("devuelve token", () => expect(harpoon.vars.get("token")).to.be.a("string"));'
```

`harpoon.toml`

```toml
[auth]
type = "bearer"
token = "{{token}}"
```

Tras `harpoon send auth/login`, cualquier otra solicitud con auth heredada
enviará `Authorization: Bearer <token>`. En `harpoon run`, las variables fluyen
de una solicitud a la siguiente en orden.
