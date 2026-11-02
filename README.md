# Harpoon

Cliente de APIs **100 % offline**, escrito en Rust. Un arpón que lanzas a un endpoint.

Las colecciones viven como archivos TOML junto a tu código, se versionan con Git,
y la misma colección sirve para el trabajo manual (TUI/escritorio), la terminal y CI/CD.
Sin cuenta, sin nube, sin telemetría, sin descargas en tiempo de ejecución.

> Estado: **fase 0** (base del proyecto). Ver [ROADMAP.md](ROADMAP.md).

## Principios

1. **Offline por diseño.** El binario nunca abre conexiones que no hayas pedido tú
   (ver [ADR-001](docs/adr/001-offline.md)).
2. **Texto plano y Git.** Todo lo que define una colección es un archivo legible
   y diffeable ([formato](docs/format.md)). Los secretos nunca entran en el repo.
3. **Un binario.** Sin runtimes externos: el motor de scripts, el generador de docs
   y el servidor mock van embebidos.
4. **CLI primero.** Todo lo que haga la interfaz lo puede hacer `harpoon` desde la terminal.

## Uso (fase 0)

```sh
cargo install --path crates/harpoon-cli

harpoon init mi-api          # crea harpoon.toml, environments/dev.toml, requests/health.toml
cd mi-api
harpoon list                 # lista solicitudes con las URLs resueltas
harpoon list --env prod      # con otro entorno
```

## Estructura del repositorio

```
crates/
  harpoon-core/   modelo de datos, formato en disco, variables      (fase 1)
  harpoon-cli/    binario `harpoon`                                   (fase 2)
  harpoon-http/   ejecución REST: auth, proxy, certificados         (fase 2)
  harpoon-script/ motor de scripts y aserciones                     (fase 3)
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
