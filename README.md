# Harpoon

Cliente de APIs **100 % offline**, escrito en Rust. Un arpón que lanzas a un endpoint.

Las colecciones viven como archivos TOML junto a tu código, se versionan con Git,
y la misma colección sirve para el trabajo manual (TUI/escritorio), la terminal y CI/CD.
Sin cuenta, sin nube, sin telemetría, sin descargas en tiempo de ejecución.

> Estado: **fase 2 en curso** (REST + CLI). Ver [ROADMAP.md](ROADMAP.md).

## Principios

1. **Offline por diseño.** El binario nunca abre conexiones que no hayas pedido tú
   (ver [ADR-001](docs/adr/001-offline.md)).
2. **Texto plano y Git.** Todo lo que define una colección es un archivo legible
   y diffeable ([formato](docs/format.md)). Los secretos nunca entran en el repo.
3. **Un binario.** Sin runtimes externos: el motor de scripts, el generador de docs
   y el servidor mock van embebidos.
4. **CLI primero.** Todo lo que haga la interfaz lo puede hacer `harpoon` desde la terminal.

## Uso

```sh
cargo install --path crates/harpoon-cli

harpoon init mi-api            # crea harpoon.toml, environments/dev.toml, requests/health.toml
cd mi-api
harpoon list                   # lista solicitudes con las URLs resueltas
harpoon send health -v         # envía una solicitud y muestra petición + respuesta
harpoon send health --json     # lo mismo, en JSON
harpoon run                    # ejecuta toda la colección y evalúa [expect]
harpoon run users --env prod   # solo la carpeta users/, con otro entorno
harpoon run --bail --json      # para CI: se detiene al primer fallo, informe JSON
```

Códigos de salida: `0` todo correcto, `1` alguna aserción falló, `2` error de uso o de red.

Aserciones sin código, en cada solicitud:

```toml
[expect]
status = 200
headers = { Content-Type = "application/json" }
body_contains = ["\"id\""]
max_time_ms = 500
```

## Estructura del repositorio

```
crates/
  harpoon-core/   modelo de datos, formato en disco, variables
  harpoon-http/   ejecución REST: auth, timeouts, proxy, aserciones [expect]
  harpoon-cli/    binario `harpoon`: init, list, send, run
  harpoon-script/ motor de scripts y tests                          (fase 3)
  harpoon-tui/    interfaz de terminal                              (fase 4)
docs/
  adr/          decisiones de arquitectura
  format.md     especificación del formato de colección
examples/
  petstore/     colección de ejemplo
```

## Desarrollo

```sh
cargo test --workspace
cargo clippy --workspace --all-targets
cargo fmt --check
```

## Licencia

MIT.
