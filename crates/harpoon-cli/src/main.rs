//! `harpoon`: interfaz de línea de comandos.
//!
//! Fase 0: `init` y `list`. `send`/`run` llegan en la fase 2.

use std::path::PathBuf;

use anyhow::Context;
use clap::{Parser, Subcommand};
use harpoon_core::Collection;

#[derive(Parser)]
#[command(name = "harpoon", version, about = "Cliente de APIs 100% offline")]
struct Cli {
    /// Directorio de la colección (por defecto: se busca `harpoon.toml` hacia arriba).
    #[arg(short = 'C', long, global = true)]
    collection: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Crea una colección nueva en el directorio indicado.
    Init {
        /// Directorio destino (se crea si no existe).
        #[arg(default_value = ".")]
        dir: PathBuf,
        /// Nombre de la colección (por defecto, el nombre del directorio).
        #[arg(short, long)]
        name: Option<String>,
    },
    /// Lista las solicitudes y entornos de la colección.
    List {
        /// Entorno con el que resolver las URLs.
        #[arg(short, long)]
        env: Option<String>,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Init { dir, name } => {
            let dir = std::path::absolute(&dir)?;
            let name = name.unwrap_or_else(|| {
                dir.file_name()
                    .map_or_else(|| "api".into(), |n| n.to_string_lossy().into_owned())
            });
            let col = Collection::init(&dir, &name)
                .with_context(|| format!("no se pudo crear la colección en {}", dir.display()))?;
            println!(
                "Colección `{}` creada en {}",
                col.meta.name,
                col.root.display()
            );
            println!("Prueba: cd {} && harpoon list", dir.display());
        }
        Command::List { env } => {
            let col = open(cli.collection.as_deref())?;
            let scope = col.scope(env.as_deref())?;
            println!("{} ({})", col.meta.name, col.root.display());
            println!(
                "\nEntornos: {}",
                col.environments
                    .keys()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            println!("\nSolicitudes:");
            for (key, req) in &col.requests {
                let url = scope
                    .interpolate(&req.url)
                    .unwrap_or_else(|_| req.url.clone());
                println!("  {key:<24} {:<7} {url}", req.method.to_string());
            }
        }
    }
    Ok(())
}

fn open(dir: Option<&std::path::Path>) -> anyhow::Result<Collection> {
    let start = match dir {
        Some(d) => d.to_path_buf(),
        None => std::env::current_dir()?,
    };
    Ok(Collection::discover(&start)?)
}
