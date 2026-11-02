//! Backend de la app de escritorio: comandos finos sobre `harpoon-core` y
//! `harpoon-http`. Toda la lógica vive en esos crates; aquí solo se
//! serializa hacia el frontend.

// Los comandos de Tauri reciben sus argumentos por valor.
#![allow(clippy::needless_pass_by_value)]

use std::path::{Path, PathBuf};

use harpoon_core::{Collection, Environment, Request, model::CollectionMeta};
use harpoon_http::{Client, Outcome, Prepared};
use harpoon_script::TestResult;
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
    is_binary: bool,
    /// Solo cuando `is_binary`: el cuerpo completo en base64.
    #[serde(skip_serializing_if = "Option::is_none")]
    body_base64: Option<String>,
    elapsed_ms: u128,
    size: usize,
}

#[derive(Serialize)]
struct ExchangeDto {
    sent: Prepared,
    response: ResponseDto,
    assertions: Vec<Outcome>,
    tests: Vec<TestResult>,
    logs: Vec<String>,
    vars: IndexMap<String, String>,
    passed: bool,
}

#[derive(Deserialize)]
struct SendArgs {
    root: PathBuf,
    env: Option<String>,
    /// Identificador de la solicitud (para el historial).
    #[serde(default)]
    key: String,
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
    let client = Client::for_collection(&col).map_err(|e| e.to_string())?;
    let exec = harpoon_script::runner::execute_persistent(
        &client,
        &col,
        &args.request,
        args.env.as_deref(),
    )
    .await
    .map_err(|e| e.to_string())?;
    let res = &exec.response;
    let binary = res.is_binary();
    let checks_total = exec.assertions.len() + exec.tests.len();
    let checks_passed = exec.assertions.iter().filter(|a| a.passed).count()
        + exec.tests.iter().filter(|t| t.passed).count();
    // El historial no bloquea el envío: si falla, se ignora.
    let _ = harpoon_core::history::append(
        &col.root,
        &args.key,
        harpoon_core::history::HistoryEntry {
            at: harpoon_core::history::now_iso(),
            env: args.env.clone(),
            method: exec.prepared.method.to_string(),
            url: exec.prepared.url.clone(),
            status: res.status,
            elapsed_ms: u64::try_from(res.elapsed_ms).unwrap_or(u64::MAX),
            size: res.size,
            headers: res.headers.clone(),
            body: if binary { String::new() } else { res.text() },
            truncated: binary,
            checks_passed,
            checks_total,
        },
    );
    Ok(ExchangeDto {
        response: ResponseDto {
            status: res.status,
            is_json: !binary && res.is_json(),
            is_binary: binary,
            body_base64: binary.then(|| res.body_base64()),
            body: if binary { String::new() } else { res.text() },
            headers: res.headers.clone(),
            elapsed_ms: res.elapsed_ms,
            size: res.size,
        },
        sent: exec.prepared,
        assertions: exec.assertions,
        tests: exec.tests,
        logs: exec.logs,
        vars: exec.vars,
        passed: exec.passed,
    })
}

#[tauri::command]
fn history(root: PathBuf, key: String) -> Result<Vec<harpoon_core::history::HistoryEntry>, String> {
    let col = load(&root)?;
    let mut entries = harpoon_core::history::load(&col.root, &key);
    entries.reverse(); // más reciente primero
    Ok(entries)
}

#[tauri::command]
fn clear_history(root: PathBuf, key: String) -> Result<(), String> {
    let col = load(&root)?;
    harpoon_core::history::clear(&col.root, &key).map_err(|e| e.to_string())
}

/// Variables de ejecución actuales (`.harpoon/vars.toml`).
#[tauri::command]
fn runtime_vars(root: PathBuf) -> Result<IndexMap<String, String>, String> {
    Ok(load(&root)?.load_runtime_vars())
}

#[tauri::command]
fn save_runtime_vars(root: PathBuf, vars: IndexMap<String, String>) -> Result<(), String> {
    load(&root)?
        .save_runtime_vars(&vars)
        .map_err(|e| e.to_string())
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
            runtime_vars,
            save_runtime_vars,
            history,
            clear_history,
            send_request
        ])
        .run(tauri::generate_context!())
        .expect("error al arrancar Harpoon");
}
