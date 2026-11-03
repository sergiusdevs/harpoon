//! Backend de la app de escritorio: comandos finos sobre `harpoon-core` y
//! `harpoon-http`. Toda la lógica vive en esos crates; aquí solo se
//! serializa hacia el frontend.

// Los comandos de Tauri reciben sus argumentos por valor.
#![allow(clippy::needless_pass_by_value)]

mod devtools;
mod git;
mod http_requests;
mod mock;
mod sse;
mod stream;
mod ws_transport;

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
    folder_configs: IndexMap<String, harpoon_core::collection::FolderConfig>,
}

impl From<Collection> for CollectionDto {
    fn from(c: Collection) -> Self {
        Self {
            root: c.root,
            meta: c.meta,
            environments: c.environments,
            requests: c.requests,
            folder_configs: c.folder_configs,
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
    #[serde(default)]
    id: Option<String>,
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
    let col = load(&path)?;
    // Repara los `.gitignore` anidados (secretos y estado local) al abrir.
    let _ = col.ensure_gitignores();
    Ok(col.into())
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

/// Ruta en disco de una carpeta de solicitudes (`""` = la colección entera).
fn folder_on_disk(root: &Path, folder: &str) -> Result<PathBuf, String> {
    if !root.join("harpoon.toml").is_file() {
        return Err(format!("{} no es una colección de Harpoon", root.display()));
    }
    if folder.is_empty() {
        return Ok(root.to_path_buf());
    }
    let safe = folder
        .split('/')
        .all(|p| !p.is_empty() && p != "." && p != ".." && !p.contains('\\'));
    if !safe {
        return Err(format!("carpeta no válida: {folder}"));
    }
    let path = root
        .join(harpoon_core::collection::REQUESTS_DIR)
        .join(folder);
    if !path.is_dir() {
        return Err(format!("no existe la carpeta {}", path.display()));
    }
    // Un enlace simbólico dentro de requests/ no puede sacarnos de la colección.
    let inside = match (path.canonicalize(), root.canonicalize()) {
        (Ok(real), Ok(base)) => real.starts_with(base),
        _ => false,
    };
    if !inside {
        return Err(format!("carpeta fuera de la colección: {folder}"));
    }
    Ok(path)
}

/// Muestra la carpeta en el gestor de archivos (Finder, Explorer…).
#[tauri::command]
fn reveal_folder(root: PathBuf, folder: String) -> Result<(), String> {
    let path = folder_on_disk(&root, &folder)?;
    tauri_plugin_opener::reveal_item_in_dir(&path).map_err(|e| e.to_string())
}

/// Abre una terminal del sistema situada en la carpeta.
#[tauri::command]
fn open_terminal(root: PathBuf, folder: String) -> Result<(), String> {
    use std::process::Command;
    let path = folder_on_disk(&root, &folder)?;
    let spawned = if cfg!(target_os = "macos") {
        Command::new("open")
            .args(["-a", "Terminal"])
            .arg(&path)
            .spawn()
    } else if cfg!(target_os = "windows") {
        // La ruta va como directorio de trabajo, nunca como texto que
        // interprete `cmd` (una carpeta "a & calc" ejecutaría comandos).
        Command::new("cmd")
            .args(["/C", "start", "", "cmd"])
            .current_dir(&path)
            .spawn()
    } else {
        [
            "x-terminal-emulator",
            "gnome-terminal",
            "konsole",
            "xfce4-terminal",
            "xterm",
        ]
        .iter()
        .find_map(|t| Command::new(t).current_dir(&path).spawn().ok())
        .ok_or_else(|| std::io::Error::other("no se encontró ninguna terminal"))
    };
    spawned
        .map(|_| ())
        .map_err(|e| format!("no se pudo abrir la terminal: {e}"))
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
fn save_folder_vars(
    root: PathBuf,
    folder: String,
    vars: IndexMap<String, String>,
    secrets: Option<Vec<String>>,
) -> Result<CollectionDto, String> {
    let mut col = load(&root)?;
    match secrets {
        Some(secrets) => col.save_folder_config(&folder, vars, secrets),
        None => col.save_folder_vars(&folder, vars),
    }
    .map_err(|e| e.to_string())?;
    Ok(col.into())
}

/// Exporta un entorno: todas sus variables, los secretos con valor vacío.
#[tauri::command]
fn export_environment(
    root: PathBuf,
    name: String,
    path: PathBuf,
    format: harpoon_core::env_io::EnvFormat,
) -> Result<(), String> {
    let col = load(&root)?;
    let env = col
        .environments
        .get(&name)
        .ok_or_else(|| format!("No existe el entorno {name}"))?;
    harpoon_core::atomic::write(&path, harpoon_core::env_io::export(&name, env, format))
        .map_err(|e| e.to_string())
}

/// Lee un entorno (Postman JSON o Harpoon TOML) sin guardarlo.
#[tauri::command]
fn preview_environment(path: PathBuf) -> Result<harpoon_core::env_io::ImportedEnv, String> {
    harpoon_core::env_io::read(&path).map_err(|e| e.to_string())
}

/// Importa un entorno con el nombre indicado (no sobrescribe uno existente).
#[tauri::command]
fn import_environment(root: PathBuf, path: PathBuf, name: String) -> Result<CollectionDto, String> {
    let imported = harpoon_core::env_io::read(&path).map_err(|e| e.to_string())?;
    let name = harpoon_core::env_io::sanitize_name(&name);
    let mut col = load(&root)?;
    if col.environments.contains_key(&name) {
        return Err(format!("Ya existe un entorno llamado {name}"));
    }
    col.save_environment(&name, imported.env)
        .map_err(|e| e.to_string())?;
    Ok(col.into())
}

/// Posibles secretos en archivos versionados de la colección.
#[tauri::command]
fn scan_secrets(root: PathBuf) -> Result<Vec<harpoon_core::scan::Finding>, String> {
    Ok(harpoon_core::scan::scan_collection(&load(&root)?))
}

/// Posibles secretos en una solicitud (antes de guardarla).
#[tauri::command]
fn scan_request(key: String, request: Request) -> Vec<harpoon_core::scan::Finding> {
    harpoon_core::scan::scan_request(&key, &request)
}

/// Secretos declarados sin valor en esta máquina (tras clonar o importar).
#[tauri::command]
fn missing_secrets(
    root: PathBuf,
    env: Option<String>,
) -> Result<Vec<harpoon_core::secrets::MissingSecret>, String> {
    Ok(load(&root)?.missing_secrets(env.as_deref()))
}

/// Dónde se guarda un secreto.
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum SecretScope {
    Collection,
    Folder { path: String },
    Environment { name: String },
}

/// Guarda (o crea) una variable secreta: el valor va al archivo local ignorado
/// y en los archivos versionados solo queda su nombre.
#[tauri::command]
fn set_secret(
    root: PathBuf,
    scope: SecretScope,
    name: String,
    value: String,
) -> Result<CollectionDto, String> {
    let name = name.trim().to_owned();
    if name.is_empty() || name.contains(['{', '}']) || name.chars().any(char::is_whitespace) {
        return Err("Nombre de variable no válido".into());
    }
    let mut col = load(&root)?;
    let err = |e: harpoon_core::Error| e.to_string();
    match scope {
        SecretScope::Collection => {
            let mut meta = col.meta.clone();
            meta.vars.insert(name.clone(), value);
            if !meta.secrets.contains(&name) {
                meta.secrets.push(name);
            }
            col.save_meta(meta).map_err(err)?;
        }
        SecretScope::Folder { path } => {
            let config = col.folder_configs.get(&path).cloned().unwrap_or_default();
            let mut vars = config.vars;
            let mut secrets = config.secrets;
            vars.insert(name.clone(), value);
            if !secrets.contains(&name) {
                secrets.push(name);
            }
            col.save_folder_config(&path, vars, secrets).map_err(err)?;
        }
        SecretScope::Environment { name: env_name } => {
            let mut env = col
                .environments
                .get(&env_name)
                .cloned()
                .ok_or_else(|| format!("No existe el entorno {env_name}"))?;
            env.vars.insert(name.clone(), value);
            if !env.secrets.contains(&name) {
                env.secrets.push(name);
            }
            col.save_environment(&env_name, env).map_err(err)?;
        }
    }
    Ok(col.into())
}

#[tauri::command]
fn resolve_url(
    root: PathBuf,
    key: String,
    env: Option<String>,
    url: String,
) -> Result<String, String> {
    let col = load(&root)?;
    col.scope_for_request(env.as_deref(), &key)
        .map_err(|e| e.to_string())?
        .with_layer(col.load_runtime_vars())
        .interpolate(&url)
        .map_err(|e| e.to_string())
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

#[derive(Serialize)]
struct CurlPrepared {
    request: Prepared,
    digest: Option<DigestCredentials>,
    settings: harpoon_core::Settings,
}

#[derive(Serialize)]
struct DigestCredentials {
    username: String,
    password: String,
}

#[tauri::command]
fn prepare_curl(
    root: PathBuf,
    key: String,
    env: Option<String>,
    request: Request,
) -> Result<CurlPrepared, String> {
    let col = load(&root)?;
    let mut prepared =
        harpoon_script::runner::prepare_for_export(&col, &request, env.as_deref(), &key)
            .map_err(|e| e.to_string())?;
    if !prepared
        .headers
        .keys()
        .any(|key| key.eq_ignore_ascii_case("cookie"))
        && let Some(value) = harpoon_http::cookies::PersistentCookies::for_root(&col.root)
            .and_then(|jar| jar.header_for_url(&prepared.url))
            .map_err(|e| e.to_string())?
    {
        prepared.headers.insert("Cookie".into(), value);
    }
    let digest = match &prepared.pending_auth {
        Some(harpoon_http::auth::Pending::Digest { username, password }) => Some(DigestCredentials { username: username.clone(), password: password.clone() }),
        Some(harpoon_http::auth::Pending::OAuth2(_)) => return Err("OAuth2 necesita obtener un token al enviar. No se puede exportar automáticamente a cURL.".into()),
        None => None,
    };
    let mut settings = col.meta.settings.clone();
    for value in [
        &mut settings.ca_cert,
        &mut settings.client_cert,
        &mut settings.client_key,
    ]
    .into_iter()
    .flatten()
    {
        *value = col.root.join(&*value).to_string_lossy().into_owned();
    }
    Ok(CurlPrepared {
        request: prepared,
        digest,
        settings,
    })
}

#[tauri::command]
fn cancel_request(
    id: String,
    state: tauri::State<'_, http_requests::Requests>,
) -> Result<(), http_requests::Failure> {
    state.cancel(&id)
}

/// Registra en la consola/red un envío terminado.
fn record_exchange(
    devtools: &devtools::DevTools,
    app: &tauri::AppHandle,
    key: &str,
    exec: &harpoon_script::runner::Execution,
) {
    let res = &exec.response;
    devtools.record(
        app,
        devtools::Record {
            key,
            method: exec.prepared.method.to_string(),
            url: &exec.prepared.url,
            query: Some(&exec.prepared.query),
            status: Some(res.status),
            duration_ms: Some(u64::try_from(res.elapsed_ms).unwrap_or(u64::MAX)),
            size: Some(res.size),
            error: None,
            logs: &exec.logs,
            script_errors: exec
                .tests
                .iter()
                .filter(|t| !t.passed && t.name.starts_with("script "))
                .map(|t| match &t.detail {
                    Some(d) => format!("{}: {d}", t.name),
                    None => t.name.clone(),
                })
                .collect(),
        },
    );
}

#[tauri::command]
async fn send_request(
    app: tauri::AppHandle,
    args: SendArgs,
    state: tauri::State<'_, http_requests::Requests>,
    devtools: tauri::State<'_, devtools::DevTools>,
) -> Result<ExchangeDto, http_requests::Failure> {
    let execute = async {
        let col = load(&args.root).map_err(http_requests::Failure::from_detail)?;
        let client = Client::for_collection(&col)
            .map_err(|e| http_requests::Failure::from_detail(e.to_string()))?;
        let exec = harpoon_script::runner::execute_persistent_for_key(
            &client,
            &col,
            &args.request,
            args.env.as_deref(),
            &args.key,
        )
        .await
        .map_err(http_requests::Failure::from)?;
        Ok((col, exec))
    };
    let result = if let Some(id) = &args.id {
        state.run(id, execute).await
    } else {
        execute.await
    };
    let (col, exec) = match result {
        Ok(done) => done,
        Err(failure) => {
            devtools.record(
                &app,
                devtools::Record {
                    key: &args.key,
                    method: args.request.method.to_string(),
                    url: &args.request.url,
                    query: None,
                    status: None,
                    duration_ms: None,
                    size: None,
                    error: Some(if failure.detail.is_empty() {
                        failure.title.to_string()
                    } else {
                        format!("{}: {}", failure.title, failure.detail)
                    }),
                    logs: &[],
                    script_errors: Vec::new(),
                },
            );
            return Err(failure);
        }
    };
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
    record_exchange(&devtools, &app, &args.key, &exec);
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
fn export_harpoon(root: PathBuf, path: PathBuf) -> Result<Vec<String>, String> {
    let col = load(&root)?;
    let shared = harpoon_core::share::export(&col).map_err(|e| e.to_string())?;
    let data = serde_json::to_vec_pretty(&shared).map_err(|e| e.to_string())?;
    harpoon_core::atomic::write(path, data).map_err(|e| e.to_string())?;
    Ok(shared.warnings)
}

#[tauri::command]
fn preview_harpoon(path: PathBuf) -> Result<harpoon_core::share::SharedCollection, String> {
    harpoon_core::share::read(&path).map_err(|e| e.to_string())
}

#[tauri::command]
fn import_harpoon(path: PathBuf, parent: PathBuf, name: String) -> Result<CollectionDto, String> {
    let mut parts = Path::new(&name).components();
    if name.is_empty()
        || !matches!(parts.next(), Some(std::path::Component::Normal(_)))
        || parts.next().is_some()
        || name.contains(['/', '\\'])
    {
        return Err("Nombre de carpeta no válido".into());
    }
    let shared = harpoon_core::share::read(&path).map_err(|e| e.to_string())?;
    harpoon_core::share::import(&shared, &parent.join(name))
        .map(Into::into)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn export_postman(root: PathBuf, path: PathBuf) -> Result<Vec<String>, String> {
    let col = load(&root)?;
    let (data, warnings) = harpoon_core::postman::export(&col);
    harpoon_core::atomic::write(
        path,
        serde_json::to_vec_pretty(&data).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    Ok(warnings)
}

#[tauri::command]
fn preview_postman(path: PathBuf) -> Result<harpoon_core::postman::Preview, String> {
    harpoon_core::postman::read(&path).map_err(|e| e.to_string())
}

#[tauri::command]
fn preview_openapi(path: PathBuf) -> Result<harpoon_core::postman::Preview, String> {
    harpoon_core::openapi::read(&path).map_err(|e| e.to_string())
}

#[tauri::command]
fn import_openapi(path: PathBuf, parent: PathBuf, name: String) -> Result<CollectionDto, String> {
    let mut parts = Path::new(&name).components();
    if name.is_empty()
        || !matches!(parts.next(), Some(std::path::Component::Normal(_)))
        || parts.next().is_some()
        || name.contains(['/', '\\'])
    {
        return Err("Nombre de carpeta no válido".into());
    }
    let collection = harpoon_core::openapi::read(&path).map_err(|e| e.to_string())?;
    harpoon_core::postman::import(&collection, &parent.join(name))
        .map(Into::into)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn import_postman(path: PathBuf, parent: PathBuf, name: String) -> Result<CollectionDto, String> {
    let mut parts = Path::new(&name).components();
    if name.is_empty()
        || !matches!(parts.next(), Some(std::path::Component::Normal(_)))
        || parts.next().is_some()
        || name.contains(['/', '\\'])
    {
        return Err("Nombre de carpeta no válido".into());
    }
    let collection = harpoon_core::postman::read(&path).map_err(|e| e.to_string())?;
    harpoon_core::postman::import(&collection, &parent.join(name))
        .map(Into::into)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn list_cookies(root: PathBuf) -> Result<Vec<harpoon_http::cookies::CookieInfo>, String> {
    let col = load(&root)?;
    harpoon_http::cookies::PersistentCookies::for_root(&col.root)
        .and_then(|jar| jar.list())
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn add_cookie(root: PathBuf, url: String, cookie: String) -> Result<(), String> {
    let col = load(&root)?;
    harpoon_http::cookies::PersistentCookies::for_root(&col.root)
        .and_then(|jar| jar.add(&url, &cookie))
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn remove_cookie(root: PathBuf, domain: String, path: String, name: String) -> Result<(), String> {
    let col = load(&root)?;
    harpoon_http::cookies::PersistentCookies::for_root(&col.root)
        .and_then(|jar| jar.remove(&domain, &path, &name))
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn save_response_file(path: PathBuf, bytes: Vec<u8>) -> Result<(), String> {
    harpoon_core::atomic::write(path, bytes).map_err(|e| e.to_string())
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
        .manage(http_requests::Requests::default())
        .manage(stream::Sessions::default())
        .manage(mock::MockState::default())
        .manage(devtools::DevTools::default())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            mock::load_mock,
            mock::save_mock,
            mock::mock_status,
            mock::start_mock,
            mock::stop_mock,
            stream::start_stream,
            stream::stream_send,
            stream::stop_stream,
            open_collection,
            init_collection,
            save_request,
            delete_request,
            rename_request,
            create_folder,
            rename_folder,
            delete_folder,
            reveal_folder,
            open_terminal,
            export_environment,
            preview_environment,
            import_environment,
            scan_secrets,
            scan_request,
            missing_secrets,
            set_secret,
            git::git_status,
            git::git_init,
            devtools::devtools_snapshot,
            devtools::devtools_clear,
            save_folder_vars,
            resolve_url,
            save_environment,
            delete_environment,
            save_meta,
            list_folders,
            runtime_vars,
            save_runtime_vars,
            export_harpoon,
            preview_harpoon,
            import_harpoon,
            export_postman,
            preview_postman,
            import_postman,
            preview_openapi,
            import_openapi,
            list_cookies,
            add_cookie,
            remove_cookie,
            save_response_file,
            history,
            clear_history,
            prepare_curl,
            send_request,
            cancel_request
        ])
        .run(tauri::generate_context!())
        .expect("error al arrancar Harpoon");
}
