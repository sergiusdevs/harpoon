//! Backend de la app de escritorio: comandos finos sobre `harpoon-core` y
//! `harpoon-http`. Toda la lógica vive en esos crates; aquí solo se
//! serializa hacia el frontend.

// Los comandos de Tauri reciben sus argumentos por valor.
#![allow(clippy::needless_pass_by_value)]

use std::path::{Path, PathBuf};

use harpoon_core::{Collection, Environment, Request, model::CollectionMeta};
use harpoon_http::{Client, Outcome, Prepared};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Vista serializable de una colección para el frontend.
#[derive(Serialize)]
struct CollectionDto {
    root: PathBuf,
    meta: CollectionMeta,
    environments: IndexMap<String, Environment>,
    requests: IndexMap<String, Request>,
}

impl From<Collection> for CollectionDto {
    fn from(c: Collection) -> Self {
        Self {
            root: c.root,
            meta: c.meta,
            environments: c.environments,
            requests: c.requests,
        }
    }
}

#[derive(Serialize)]
struct ResponseDto {
    status: u16,
    headers: IndexMap<String, String>,
    body: String,
    is_json: bool,
    elapsed_ms: u128,
    size: usize,
}

#[derive(Serialize)]
struct ExchangeDto {
    sent: Prepared,
    response: ResponseDto,
    assertions: Vec<Outcome>,
    passed: bool,
}

#[derive(Deserialize)]
struct SendArgs {
    root: PathBuf,
    env: Option<String>,
    request: Request,
}

fn load(root: &Path) -> Result<Collection, String> {
    Collection::discover(root).map_err(|e| e.to_string())
}

#[tauri::command]
fn open_collection(path: PathBuf) -> Result<CollectionDto, String> {
    load(&path).map(Into::into)
}

#[tauri::command]
fn init_collection(path: PathBuf, name: String) -> Result<CollectionDto, String> {
    Collection::init(&path, &name)
        .map(Into::into)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn save_request(root: PathBuf, key: String, request: Request) -> Result<(), String> {
    let mut col = load(&root)?;
    col.save_request(&key, request).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_request(root: PathBuf, key: String) -> Result<CollectionDto, String> {
    let mut col = load(&root)?;
    col.delete_request(&key).map_err(|e| e.to_string())?;
    Ok(col.into())
}

#[tauri::command]
fn rename_request(root: PathBuf, from: String, to: String) -> Result<CollectionDto, String> {
    let mut col = load(&root)?;
    col.rename_request(&from, &to).map_err(|e| e.to_string())?;
    Ok(col.into())
}

#[tauri::command]
fn create_folder(root: PathBuf, folder: String) -> Result<CollectionDto, String> {
    let col = load(&root)?;
    col.create_folder(&folder).map_err(|e| e.to_string())?;
    Ok(col.into())
}

#[tauri::command]
fn rename_folder(root: PathBuf, from: String, to: String) -> Result<CollectionDto, String> {
    let mut col = load(&root)?;
    col.rename_folder(&from, &to).map_err(|e| e.to_string())?;
    Ok(col.into())
}

#[tauri::command]
fn delete_folder(root: PathBuf, folder: String) -> Result<CollectionDto, String> {
    let mut col = load(&root)?;
    col.delete_folder(&folder).map_err(|e| e.to_string())?;
    Ok(col.into())
}

#[tauri::command]
fn save_environment(
    root: PathBuf,
    name: String,
    env: Environment,
) -> Result<CollectionDto, String> {
    let mut col = load(&root)?;
    col.save_environment(&name, env)
        .map_err(|e| e.to_string())?;
    Ok(col.into())
}

#[tauri::command]
fn delete_environment(root: PathBuf, name: String) -> Result<CollectionDto, String> {
    let mut col = load(&root)?;
    col.delete_environment(&name).map_err(|e| e.to_string())?;
    Ok(col.into())
}

#[tauri::command]
fn save_meta(root: PathBuf, meta: CollectionMeta) -> Result<CollectionDto, String> {
    let mut col = load(&root)?;
    col.save_meta(meta).map_err(|e| e.to_string())?;
    Ok(col.into())
}

fn walk_folders(base: &Path, dir: &Path, out: &mut Vec<String>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<_> = rd
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    entries.sort();
    for p in entries {
        if let Ok(rel) = p.strip_prefix(base) {
            out.push(rel.to_string_lossy().replace('\\', "/"));
        }
        walk_folders(base, &p, out);
    }
}

/// Lista las carpetas (incluidas las vacías) bajo `requests/`.
#[tauri::command]
fn list_folders(root: PathBuf) -> Result<Vec<String>, String> {
    let col = load(&root)?;
    let base = col.root.join(harpoon_core::collection::REQUESTS_DIR);
    let mut out = Vec::new();
    walk_folders(&base, &base, &mut out);
    Ok(out)
}

#[tauri::command]
async fn send_request(args: SendArgs) -> Result<ExchangeDto, String> {
    let col = load(&args.root)?;
    let scope = col.scope(args.env.as_deref()).map_err(|e| e.to_string())?;
    let client = Client::new(&col.meta.settings).map_err(|e| e.to_string())?;
    let (sent, res, assertions) = harpoon_http::execute(&client, &col, &args.request, &scope)
        .await
        .map_err(|e| e.to_string())?;
    let passed = assertions.iter().all(|o| o.passed);
    Ok(ExchangeDto {
        sent,
        response: ResponseDto {
            status: res.status,
            is_json: res.is_json(),
            body: res.text(),
            headers: res.headers.clone(),
            elapsed_ms: res.elapsed_ms,
            size: res.size,
        },
        assertions,
        passed,
    })
}

/// Punto de entrada de Tauri.
///
/// # Panics
/// Si la ventana no puede crearse (webview del sistema ausente).
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            open_collection,
            init_collection,
            save_request,
            delete_request,
            rename_request,
            create_folder,
            rename_folder,
            delete_folder,
            save_environment,
            delete_environment,
            save_meta,
            list_folders,
            send_request
        ])
        .run(tauri::generate_context!())
        .expect("error al arrancar Harpoon");
}
