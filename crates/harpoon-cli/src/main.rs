//! `harpoon`: interfaz de línea de comandos.

mod output;

use std::{path::PathBuf, process::ExitCode};

use anyhow::Context;
use clap::{Parser, Subcommand};
use harpoon_core::Collection;
use harpoon_http::Client;

#[derive(Parser)]
#[command(name = "harpoon", version, about = "Cliente de APIs 100% offline")]
struct Cli {
    /// Directorio de la colección (por defecto: se busca `harpoon.toml` hacia arriba).
    #[arg(short = 'C', long, global = true)]
    collection: Option<PathBuf>,

    /// Entorno a usar (por defecto, `default_environment` de la colección).
    #[arg(short, long, global = true)]
    env: Option<String>,

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
    List,
    /// Envía una solicitud y muestra la respuesta.
    Send {
        /// Identificador de la solicitud (ruta relativa sin `.toml`, p. ej. `users/list`).
        request: String,
        /// Muestra también la petición enviada (cabeceras y cuerpo resueltos).
        #[arg(short, long)]
        verbose: bool,
        /// Salida en JSON (petición, respuesta y aserciones).
        #[arg(long)]
        json: bool,
    },
    /// Ejecuta en orden todas las solicitudes (o las de una carpeta) y evalúa sus aserciones.
    Run {
        /// Carpeta o prefijo a ejecutar (p. ej. `users`). Sin argumento, toda la colección.
        prefix: Option<String>,
        /// Detenerse en el primer fallo.
        #[arg(long)]
        bail: bool,
        /// Salida en JSON (informe completo).
        #[arg(long)]
        json: bool,
    },
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::from(2)
        }
    }
}

#[tokio::main]
async fn run() -> anyhow::Result<ExitCode> {
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
            Ok(ExitCode::SUCCESS)
        }
        Command::List => {
            let col = open(cli.collection.as_deref())?;
            let scope = col
                .scope(cli.env.as_deref())?
                .with_layer(col.load_runtime_vars());
            output::list(&col, &scope);
            Ok(ExitCode::SUCCESS)
        }
        Command::Send {
            request,
            verbose,
            json,
        } => {
            let col = open(cli.collection.as_deref())?;
            let req = col
                .requests
                .get(&request)
                .with_context(|| format!("no existe la solicitud `{request}`"))?;
            let client = Client::new(&col.meta.settings)?;
            let exec =
                harpoon_script::runner::execute_persistent(&client, &col, req, cli.env.as_deref())
                    .await?;
            if json {
                output::send_json(&request, &exec)?;
            } else {
                output::send_human(&exec, verbose);
            }
            Ok(exit_for(exec.passed))
        }
        Command::Run { prefix, bail, json } => {
            let col = open(cli.collection.as_deref())?;
            let client = Client::new(&col.meta.settings)?;
            let selected: Vec<(&String, &harpoon_core::Request)> = col
                .requests
                .iter()
                .filter(|(key, _)| {
                    prefix
                        .as_deref()
                        .is_none_or(|p| *key == p || key.starts_with(&format!("{p}/")))
                })
                .collect();
            anyhow::ensure!(
                !selected.is_empty(),
                "ninguna solicitud coincide con el filtro"
            );

            let mut report = output::Report::new(&col.meta.name, cli.env.as_deref());
            // Las variables de ejecución fluyen de una solicitud a la siguiente
            // y se persisten al final.
            let mut vars = col.load_runtime_vars();
            for (key, req) in selected {
                let item = match harpoon_script::execute(
                    &client,
                    &col,
                    req,
                    cli.env.as_deref(),
                    vars.clone(),
                )
                .await
                {
                    Ok(exec) => {
                        vars.clone_from(&exec.vars);
                        output::RunItem::done(key, &exec)
                    }
                    Err(err) => output::RunItem::failed(key, &err.to_string()),
                };
                if !json {
                    output::run_progress(&item);
                }
                let ok = item.passed();
                report.push(item);
                if bail && !ok {
                    break;
                }
            }
            let all_ok = report.all_passed();
            col.save_runtime_vars(&vars)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                output::run_summary(&report);
            }
            Ok(exit_for(all_ok))
        }
    }
}

fn exit_for(ok: bool) -> ExitCode {
    if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn open(dir: Option<&std::path::Path>) -> anyhow::Result<Collection> {
    let start = match dir {
        Some(d) => d.to_path_buf(),
        None => std::env::current_dir()?,
    };
    Ok(Collection::discover(&start)?)
}
