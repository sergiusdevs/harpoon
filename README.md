# Arpón

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
4. **CLI primero.** Todo lo que haga la interfaz lo puede hacer `arpon` desde la terminal.

## Uso (fase 0)

```sh
cargo install --path crates/arpon-cli

arpon init mi-api          # crea arpon.toml, environments/dev.toml, requests/health.toml
cd mi-api
arpon list                 # lista solicitudes con las URLs resueltas
arpon list --env prod      # con otro entorno
```

## Estructura del repositorio

```
crates/
  arpon-core/   modelo de datos, formato en disco, variables      (fase 1)
  arpon-cli/    binario `arpon`                                   (fase 2)
  arpon-http/   ejecución REST: auth, proxy, certificados         (fase 2)
  arpon-script/ motor de scripts y aserciones                     (fase 3)
  arpon-tui/    interfaz de terminal                              (fase 4)
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
