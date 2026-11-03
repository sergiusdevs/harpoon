//! Motor JavaScript embebido (`QuickJS`). Todo el intercambio con los scripts
//! se hace por JSON para mantener la superficie mínima y segura.

use std::time::{Duration, Instant};

use harpoon_core::Method;
use indexmap::IndexMap;
use rquickjs::{Context, Runtime};
use serde::{Deserialize, Serialize};

const PRELUDE: &str = include_str!("prelude.js");
const MEMORY_LIMIT: usize = 64 * 1024 * 1024;
const MAX_STACK: usize = 1024 * 1024;
const TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("error en el script ({phase}): {message}")]
    Script { phase: String, message: String },
    #[error("el script ({0}) superó el tiempo máximo de {1:?}")]
    Timeout(String, Duration),
    #[error("error interno del motor de scripts: {0}")]
    Engine(String),
}

pub type Result<T> = std::result::Result<T, Error>;

/// La solicitud tal y como la ve el script `pre` (puede modificarla).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScriptRequest {
    pub method: Method,
    pub url: String,
    pub headers: IndexMap<String, String>,
    pub query: IndexMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
}

/// La respuesta tal y como la ve el script `post`/`test`.
#[derive(Debug, Clone, Serialize)]
pub struct ScriptResponse<'a> {
    pub status: u16,
    pub headers: &'a IndexMap<String, String>,
    pub body: String,
    pub elapsed_ms: u128,
    pub size: usize,
}

/// Resultado de un `test(...)` dentro de un script.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TestResult {
    pub name: String,
    pub passed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// Lo que devuelve el script al terminar.
#[derive(Debug, Clone, Deserialize)]
pub struct Output {
    #[serde(default)]
    pub req: Option<ScriptRequest>,
    #[serde(default)]
    pub vars: IndexMap<String, String>,
    #[serde(default)]
    pub tests: Vec<TestResult>,
    #[serde(default)]
    pub logs: Vec<String>,
}

#[derive(Serialize)]
struct Input<'a> {
    req: Option<&'a ScriptRequest>,
    res: Option<&'a ScriptResponse<'a>>,
    vars: &'a IndexMap<String, String>,
    env: &'a IndexMap<String, String>,
}

/// Un contexto de ejecución. Es barato de crear; se recomienda uno por
/// solicitud para que un script no contamine al siguiente.
pub struct Engine {
    rt: Runtime,
    ctx: Context,
}

impl Engine {
    pub fn new() -> Result<Self> {
        let rt = Runtime::new().map_err(|e| Error::Engine(e.to_string()))?;
        rt.set_memory_limit(MEMORY_LIMIT);
        rt.set_max_stack_size(MAX_STACK);
        let ctx = Context::full(&rt).map_err(|e| Error::Engine(e.to_string()))?;
        ctx.with(|ctx| ctx.eval::<(), _>(PRELUDE))
            .map_err(|e| Error::Engine(format!("preludio: {e}")))?;
        Ok(Self { rt, ctx })
    }

    /// Ejecuta un script con el estado indicado y devuelve el estado resultante.
    ///
    /// `env` son las variables resueltas (colección + entorno), solo lectura;
    /// `vars` son las variables de ejecución, que el script puede cambiar.
    pub fn run(
        &self,
        phase: &str,
        script: &str,
        req: Option<&ScriptRequest>,
        res: Option<&ScriptResponse<'_>>,
        vars: &IndexMap<String, String>,
        env: &IndexMap<String, String>,
    ) -> Result<Output> {
        if script.trim().is_empty() {
            return Ok(Output {
                req: req.cloned(),
                vars: vars.clone(),
                tests: Vec::new(),
                logs: Vec::new(),
            });
        }
        let input = serde_json::to_string(&Input {
            req,
            res,
            vars,
            env,
        })
        .map_err(|e| Error::Engine(e.to_string()))?;

        let deadline = Instant::now() + TIMEOUT;
        self.rt
            .set_interrupt_handler(Some(Box::new(move || Instant::now() > deadline)));

        let result: std::result::Result<String, String> = self.ctx.with(|ctx| {
            let init: rquickjs::Function = ctx
                .globals()
                .get::<_, rquickjs::Object>("harpoon")
                .and_then(|h| h.get("__init"))
                .map_err(|e| e.to_string())?;
            init.call::<_, ()>((input.as_str(),))
                .map_err(|e| describe(&ctx, &e))?;
            ctx.eval::<(), _>(script).map_err(|e| describe(&ctx, &e))?;
            ctx.eval::<String, _>("harpoon.__state()")
                .map_err(|e| describe(&ctx, &e))
        });
        self.rt.set_interrupt_handler(None);

        match result {
            Ok(json) => serde_json::from_str(&json).map_err(|e| Error::Engine(e.to_string())),
            Err(message) if message.contains("interrupted") => {
                Err(Error::Timeout(phase.to_owned(), TIMEOUT))
            }
            Err(message) => Err(Error::Script {
                phase: phase.to_owned(),
                message,
            }),
        }
    }
}

/// Convierte una excepción de `QuickJS` en un mensaje legible (con línea si la hay).
fn describe(ctx: &rquickjs::Ctx<'_>, err: &rquickjs::Error) -> String {
    if matches!(err, rquickjs::Error::Exception) {
        let exc = ctx.catch();
        if let Some(obj) = exc.as_object() {
            let msg: Option<String> = obj.get("message").ok();
            let stack: Option<String> = obj.get("stack").ok();
            let line = stack
                .as_deref()
                .and_then(|s| s.lines().find(|l| l.contains("<eval>")))
                .map(|l| l.trim().to_owned());
            return match (msg, line) {
                (Some(m), Some(l)) => format!("{m} ({l})"),
                (Some(m), None) => m,
                _ => "excepción sin mensaje".into(),
            };
        }
        if let Some(s) = exc.as_string().and_then(|s| s.to_string().ok()) {
            return s;
        }
        return "excepción desconocida".into();
    }
    err.to_string()
}

#[cfg(test)]
mod tests;
