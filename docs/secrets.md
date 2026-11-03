# Secretos y Git

Una colección de Harpoon está pensada para vivir en un repositorio Git. La regla
es simple: **en los archivos versionados solo queda el nombre de cada secreto;
el valor vive en tu máquina** (o en una variable del sistema en CI).

## Dónde se declara y dónde se guarda

| Nivel | Se declara en (va a Git) | El valor vive en (NO va a Git) |
|---|---|---|
| Entorno | `environments/<env>.toml` → `secrets = ["token"]` | `environments/<env>.secrets.toml` |
| Colección | `harpoon.toml` → `secrets = ["api_key"]` | `.harpoon/secrets.toml` → `[collection]` |
| Carpeta | `requests/<carpeta>/folder.toml` → `secrets = ["admin_pass"]` | `.harpoon/secrets.toml` → `[folders."<carpeta>"]` |

En la app basta con marcar la variable como secreta (icono de candado); Harpoon
reparte nombre y valor entre los dos archivos al guardar.

Si un secreto no tiene valor en el archivo local, se usa la variable del sistema
`HARPOON_<NOMBRE>` (mayúsculas; lo que no sea letra o número pasa a `_`):

```sh
HARPOON_TOKEN=… HARPOON_API_KEY=… harpoon run
```

## Qué se ignora y cómo

Harpoon escribe un `.gitignore` **dentro** de cada carpeta sensible, así que
funciona aunque la colección esté dentro de otro repositorio con su propio
`.gitignore`:

- `.harpoon/.gitignore` → `*` (secretos de colección y carpeta, variables de
  ejecución, historial, cookies).
- `environments/.gitignore` → `*.secrets.toml`.

Se revisan cada vez que la app abre la colección y cada vez que se guarda un
secreto. Solo se añade la regla que falte; el resto del archivo no se toca.

## Antes de que llegue a Git

- **Al guardar una solicitud**, la app revisa si lleva algo que parezca un
  secreto escrito tal cual: un token en `Authorization`, el password de Basic,
  un `client_secret`, `?api_key=…`, `usuario:contraseña@` en la URL, o formatos
  conocidos (JWT, AWS, GitHub, GitLab, Slack, Stripe, OpenAI, Anthropic, Google,
  npm, claves PEM). Ofrece convertirlo en variable secreta: el valor se guarda en
  local y en la solicitud queda `{{nombre}}` (respetando `Bearer ` y otras
  variables).
- **Al exportar** (`.harpoon.json` o Postman) se avisa de esos mismos hallazgos.
  Los valores secretos nunca se exportan: en Postman salen vacíos con tipo
  `secret`; al importar de Postman, las variables `secret` se marcan como
  secretas.
- **El historial** de respuestas oculta `Authorization`, `Cookie`, `Set-Cookie`
  y parámetros sensibles de la URL.

## `harpoon check` (pre-commit y CI)

```sh
harpoon check          # sale con 1 si hay algo que corregir
harpoon check --json   # informe para CI
```

Comprueba:

1. Posibles secretos escritos tal cual en archivos versionados.
2. Archivos de secretos o estado local que Git no ignora o que **ya están en
   Git** (sugiere `git rm --cached`).
3. Claves de certificados (`client_key`, `client_cert`) dentro de la colección
   sin ignorar.
4. Secretos declarados sin valor en esta máquina (informativo, no falla).

Hook de pre-commit:

```sh
# .git/hooks/pre-commit  (chmod +x)
#!/bin/sh
harpoon -C ruta/a/la/coleccion check
```

## Al clonar o importar

Tras un `git clone` los valores no están. La app lista los secretos que faltan
("Faltan secretos en esta máquina") y permite rellenarlos; `harpoon check`
también los muestra. Cada persona guarda los suyos: no hay un mecanismo para
pasar valores secretos entre personas a propósito.

## Al importar colecciones ajenas

Una colección puede traer **scripts** que se ejecutan al enviar y que pueden leer
tus variables y secretos. Al importar un `.harpoon.json`, Harpoon avisa de qué
archivos llevan scripts y de si la colección desactiva la verificación TLS
(`insecure = true`). Los avisos los calcula Harpoon: se ignoran los que vengan
escritos en el propio archivo.
