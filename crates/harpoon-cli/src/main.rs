//! `harpoon`: interfaz de línea de comandos.

mod datafile;
mod output;

use std::{path::PathBuf, process::ExitCode};

use anyhow::Context;
use clap::{Parser, Subcommand};
use harpoon_core::Collection;
use harpoon_http::Client;
use indexmap::IndexMap;

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
    /// Exporta la colección a un archivo intercambiable de Harpoon.
    ShareExport { file: PathBuf },
    /// Importa un archivo Harpoon a un directorio nuevo.
    ShareImport { file: PathBuf, dir: PathBuf },
    /// Exporta una colección Postman v2.1.
    PostmanExport { file: PathBuf },
    /// Importa una colección Postman v2.1 a un directorio nuevo.
    PostmanImport { file: PathBuf, dir: PathBuf },
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
        /// Escribe un informe `JUnit` XML en esta ruta (para CI).
        #[arg(long, value_name = "ARCHIVO")]
        junit: Option<PathBuf>,
        /// Repite la ejecución N veces (`{{iteration}}` = 1..N).
        #[arg(long, default_value_t = 1, value_name = "N")]
        iterations: usize,
        /// Archivo de datos (.csv o .json con una lista de objetos): una ejecución por fila,
        /// con sus columnas disponibles como variables.
        #[arg(long, value_name = "ARCHIVO")]
        data: Option<PathBuf>,
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
#[allow(clippy::too_many_lines)]
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
        Command::PostmanExport { file } => {
            let col = open(cli.collection.as_deref())?;
            let (document, warnings) = harpoon_core::postman::export(&col);
            std::fs::write(&file, serde_json::to_vec_pretty(&document)?)?;
            for warning in warnings {
                eprintln!("Aviso: {warning}");
            }
            println!("Colección exportada a {}", file.display());
            Ok(ExitCode::SUCCESS)
        }
        Command::PostmanImport { file, dir } => {
            let preview = harpoon_core::postman::read(&file)?;
            for warning in &preview.warnings {
                eprintln!("Aviso: {warning}");
            }
            let col = harpoon_core::postman::import(&preview, &dir)?;
            println!("Colección importada en {}", col.root.display());
            Ok(ExitCode::SUCCESS)
        }
        Command::ShareExport { file } => {
            let col = open(cli.collection.as_deref())?;
            let shared = harpoon_core::share::export(&col)?;
            std::fs::write(&file, serde_json::to_vec_pretty(&shared)?)?;
            for warning in &shared.warnings {
                eprintln!("Aviso: {warning}");
            }
            println!("Colección exportada a {}", file.display());
            Ok(ExitCode::SUCCESS)
        }
        Command::ShareImport { file, dir } => {
            let shared = harpoon_core::share::read(&file)?;
            let col = harpoon_core::share::import(&shared, &dir)?;
            for warning in &shared.warnings {
                eprintln!("Aviso: {warning}");
            }
            println!("Colección importada en {}", col.root.display());
            Ok(ExitCode::SUCCESS)
        }
        Command::List => {
            let col = open(cli.collection.as_deref())?;
            output::list(&col, cli.env.as_deref())?;
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
            let client = Client::for_collection(&col)?;
            let exec = harpoon_script::runner::execute_persistent_for_key(
                &client,
                &col,
                req,
                cli.env.as_deref(),
                &request,
            )
            .await?;
            if json {
                output::send_json(&request, &exec)?;
            } else {
                output::send_human(&exec, verbose);
            }
            Ok(exit_for(exec.passed))
        }
        Command::Run {
            prefix,
            bail,
            json,
            junit,
            iterations,
            data,
        } => {
            let col = open(cli.collection.as_deref())?;
            let client = Client::for_collection(&col)?;
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

            // Filas de datos: cada una es una ejecución completa.
            let rows: Vec<IndexMap<String, String>> = match &data {
                Some(path) => datafile::load(path)
                    .with_context(|| format!("no se pudo leer {}", path.display()))?,
                None => vec![IndexMap::new()],
            };
            anyhow::ensure!(!rows.is_empty(), "el archivo de datos está vacío");

            let mut report = output::Report::new(&col.meta.name, cli.env.as_deref());
            // Las variables de ejecución fluyen de una solicitud a la siguiente
            // y se persisten al final.
            let mut vars = col.load_runtime_vars();
            let total_runs = iterations * rows.len();
            'outer: for iteration in 1..=iterations {
                for (row_idx, row) in rows.iter().enumerate() {
                    let run_no = (iteration - 1) * rows.len() + row_idx + 1;
                    if !json && total_runs > 1 {
                        output::run_header(run_no, total_runs, iteration, row);
                    }
                    let mut run_vars = vars.clone();
                    run_vars.insert("iteration".into(), iteration.to_string());
                    for (k, v) in row {
                        run_vars.insert(k.clone(), v.clone());
                    }
                    for (key, req) in &selected {
                        let item = match harpoon_script::runner::execute_for_key(
                            &client,
                            &col,
                            req,
                            cli.env.as_deref(),
                            run_vars.clone(),
                            key,
                        )
                        .await
                        {
                            Ok(exec) => {
                                run_vars.clone_from(&exec.vars);
                                output::RunItem::done(key, run_no, &exec)
                            }
                            Err(err) => output::RunItem::failed(key, run_no, &err.to_string()),
                        };
                        if !json {
                            output::run_progress(&item);
                        }
                        let ok = item.passed();
                        report.push(item);
                        if bail && !ok {
                            break 'outer;
                        }
                    }
                    // Lo que fijen los scripts persiste; las columnas de datos no.
                    for (k, v) in run_vars {
                        if k != "iteration" && !row.contains_key(&k) {
                            vars.insert(k, v);
                        }
                    }
                }
            }
            let all_ok = report.all_passed();
            col.save_runtime_vars(&vars)?;
            if let Some(path) = &junit {
                std::fs::write(path, output::junit(&report))
                    .with_context(|| format!("no se pudo escribir {}", path.display()))?;
                if !json {
                    eprintln!("Informe JUnit escrito en {}", path.display());
                }
            }
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
