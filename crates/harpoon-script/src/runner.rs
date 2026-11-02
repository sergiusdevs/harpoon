//! Pipeline completo de una solicitud:
//!
//! ```text
//! pre (colección) → pre (solicitud) → HTTP → post (colección) → post (solicitud)
//!   → test (colección) → test (solicitud) → aserciones [expect]
//! ```
//!
//! Las variables de ejecución (`harpoon.vars`) entran como capa de máxima
//! prioridad en la interpolación y salen actualizadas para persistirlas.

use harpoon_core::{Body, Collection, Request};
use harpoon_http::{Client, Outcome, Prepared, Response};
use indexmap::IndexMap;
use serde::Serialize;

use crate::engine::{Engine, Error as ScriptError, ScriptRequest, ScriptResponse, TestResult};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Script(#[from] ScriptError),
    #[error(transparent)]
    Http(#[from] harpoon_http::Error),
    #[error(transparent)]
    Core(#[from] harpoon_core::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

/// Resultado completo de ejecutar una solicitud.
#[derive(Debug, Clone, Serialize)]
pub struct Execution {
    pub prepared: Prepared,
    pub response: Response,
    /// Aserciones declarativas `[expect]`.
    pub assertions: Vec<Outcome>,
    /// Resultados de `test(...)` en scripts (incluye fallos del propio script).
    pub tests: Vec<TestResult>,
    pub logs: Vec<String>,
    /// Variables de ejecución tras los scripts (para persistir).
    pub vars: IndexMap<String, String>,
    pub passed: bool,
}

fn to_script_request(req: &Request) -> ScriptRequest {
    ScriptRequest {
        method: req.method,
        url: req.url.clone(),
        headers: req.headers.clone(),
        query: req.query.clone(),
        body: match &req.body {
            Body::None | Body::Form { .. } | Body::Multipart { .. } | Body::Binary { .. } => None,
            Body::Json { json } => Some(json.clone()),
            Body::Text { text } => Some(text.clone()),
            Body::Xml { xml } => Some(xml.clone()),
        },
    }
}

fn apply_script_request(req: &mut Request, s: ScriptRequest) {
    req.method = s.method;
    req.url = s.url;
    req.headers = s.headers;
    req.query = s.query;
    match (&req.body, s.body) {
        (Body::Form { .. } | Body::Multipart { .. } | Body::Binary { .. }, _) => {}
        (Body::Json { .. }, Some(b)) => req.body = Body::Json { json: b },
        (Body::Text { .. }, Some(b)) => req.body = Body::Text { text: b },
        (Body::Xml { .. }, Some(b)) => req.body = Body::Xml { xml: b },
        (Body::None, Some(b)) => {
            req.body = if serde_json::from_str::<serde_json::Value>(&b).is_ok() {
                Body::Json { json: b }
            } else {
                Body::Text { text: b }
            };
        }
        (_, None) => req.body = Body::None,
    }
}

/// Prepara una solicitud para exportarla, ejecutando scripts previos sin persistir variables ni enviar tráfico.
pub fn prepare_for_export(
    collection: &Collection,
    request: &Request,
    environment: Option<&str>,
    key: &str,
) -> Result<Prepared> {
    let base_scope = collection.scope_for_request(environment, key)?;
    let env = base_scope.flatten();
    let mut vars = collection.load_runtime_vars();
    let mut req = request.clone();
    let engine = Engine::new()?;
    let mut script_req = to_script_request(&req);
    for (phase, script) in [
        ("pre (colección)", &collection.meta.scripts.pre),
        ("pre", &request.scripts.pre),
    ] {
        let out = engine.run(phase, script, Some(&script_req), None, &vars, &env)?;
        if let Some(r) = out.req {
            script_req = r;
        }
        vars = out.vars;
    }
    apply_script_request(&mut req, script_req);
    let scope = base_scope.with_layer(vars);
    Ok(harpoon_http::prepare(collection, &req, &scope)?)
}

/// Ejecuta una solicitud con todos sus scripts. `vars` son las variables de
/// ejecución de entrada; las de salida vienen en [`Execution::vars`].
pub async fn execute(
    client: &Client,
    collection: &Collection,
    request: &Request,
    environment: Option<&str>,
    vars: IndexMap<String, String>,
) -> Result<Execution> {
    execute_for_key(client, collection, request, environment, vars, "").await
}

/// Ejecuta una solicitud aplicando las variables de sus carpetas.
pub async fn execute_for_key(
    client: &Client,
    collection: &Collection,
    request: &Request,
    environment: Option<&str>,
    vars: IndexMap<String, String>,
    key: &str,
) -> Result<Execution> {
    let base_scope = collection.scope_for_request(environment, key)?;
    let env = base_scope.flatten();
    let mut vars = vars;
    let mut logs = Vec::new();
    let mut tests = Vec::new();

    // ---- pre ---- (el motor se crea y se destruye dentro del bloque: no es
    // `Send`, así que no puede sobrevivir al `await` del envío)
    let mut req = request.clone();
    {
        let engine = Engine::new()?;
        let mut script_req = to_script_request(&req);
        for (phase, script) in [
            ("pre (colección)", &collection.meta.scripts.pre),
            ("pre", &request.scripts.pre),
        ] {
            let out = engine.run(phase, script, Some(&script_req), None, &vars, &env)?;
            if let Some(r) = out.req {
                script_req = r;
            }
            vars = out.vars;
            logs.extend(out.logs);
        }
        apply_script_request(&mut req, script_req);
    }

    // ---- http ----
    let scope = base_scope.with_layer(vars.clone());
    let prepared = harpoon_http::prepare(collection, &req, &scope)?;
    let response = client.send(&prepared).await?;

    // ---- post / test ----
    {
        let engine = Engine::new()?;
        let script_req = ScriptRequest {
            method: prepared.method,
            url: prepared.url.clone(),
            headers: prepared.headers.clone(),
            query: prepared.query.clone(),
            body: prepared.body.clone(),
        };
        let response_view = ScriptResponse {
            status: response.status,
            headers: &response.headers,
            body: response.text(),
            elapsed_ms: response.elapsed_ms,
            size: response.size,
        };
        for (phase, script) in [
            ("post (colección)", &collection.meta.scripts.post),
            ("post", &request.scripts.post),
            ("test (colección)", &collection.meta.scripts.test),
            ("test", &request.scripts.test),
        ] {
            match engine.run(
                phase,
                script,
                Some(&script_req),
                Some(&response_view),
                &vars,
                &env,
            ) {
                Ok(out) => {
                    vars = out.vars;
                    logs.extend(out.logs);
                    tests.extend(out.tests);
                }
                Err(e) => tests.push(TestResult {
                    name: format!("script {phase}"),
                    passed: false,
                    detail: Some(e.to_string()),
                }),
            }
        }
    }

    let assertions = harpoon_http::assert::evaluate(&request.expect, &response);
    let passed = assertions.iter().all(|o| o.passed) && tests.iter().all(|t| t.passed);
    Ok(Execution {
        prepared,
        response,
        assertions,
        tests,
        logs,
        vars,
        passed,
    })
}

/// Variante que carga y guarda las variables de ejecución en `.harpoon/vars.toml`.
pub async fn execute_persistent(
    client: &Client,
    collection: &Collection,
    request: &Request,
    environment: Option<&str>,
) -> Result<Execution> {
    let vars = collection.load_runtime_vars();
    let exec = execute(client, collection, request, environment, vars).await?;
    collection.save_runtime_vars(&exec.vars)?;
    Ok(exec)
}

/// Variante persistente que aplica las variables de las carpetas de la solicitud.
pub async fn execute_persistent_for_key(
    client: &Client,
    collection: &Collection,
    request: &Request,
    environment: Option<&str>,
    key: &str,
) -> Result<Execution> {
    let vars = collection.load_runtime_vars();
    let exec = execute_for_key(client, collection, request, environment, vars, key).await?;
    collection.save_runtime_vars(&exec.vars)?;
    Ok(exec)
}
