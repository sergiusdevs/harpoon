//! Backend de la app de escritorio: comandos finos sobre `harpoon-core` y
//! `harpoon-http`. Toda la lógica vive en esos crates; aquí solo se
//! serializa hacia el frontend.

// Los comandos de Tauri reciben sus argumentos por valor.
#![allow(clippy::needless_pass_by_value)]

use std::path::{Path, PathBuf};

use harpoon_core::{Collection, Request};
use harpoon_http::{Client, Outcome, Prepared};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Vista serializable de una colección para el frontend.
#[derive(Serialize)]
struct CollectionDto {
    root: PathBuf,
    meta: harpoon_core::model::CollectionMeta,
    environments: Vec<String>,
    requests: IndexMap<String, Request>,
}

impl From<Collection> for CollectionDto {
    fn from(c: Collection) -> Self {
        Self {
            root: c.root,
            meta: c.meta,
            environments: c.environments.keys().cloned().collect(),
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
            send_request
        ])
        .run(tauri::generate_context!())
        .expect("error al arrancar Harpoon");
}
