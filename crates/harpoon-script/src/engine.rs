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
    Script {
        phase: &'static str,
        message: String,
    },
    #[error("el script ({0}) superó el tiempo máximo de {1:?}")]
    Timeout(&'static str, Duration),
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
        phase: &'static str,
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
            Err(message) if message.contains("interrupted") => Err(Error::Timeout(phase, TIMEOUT)),
            Err(message) => Err(Error::Script { phase, message }),
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
mod tests {
    use super::*;

    fn req() -> ScriptRequest {
        ScriptRequest {
            method: Method::Get,
            url: "{{base_url}}/x".into(),
            headers: IndexMap::new(),
            query: IndexMap::new(),
            body: None,
        }
    }

    #[test]
    fn pre_modifica_la_solicitud_y_las_variables() {
        let e = Engine::new().unwrap();
        let out = e
            .run(
                "pre",
                r#"
                harpoon.req.headers["X-Trace"] = "abc";
                harpoon.req.url = harpoon.req.url + "?ts=1";
                harpoon.vars.set("nonce", 42);
                console.log("hola", {a: 1});
                "#,
                Some(&req()),
                None,
                &IndexMap::new(),
                &IndexMap::new(),
            )
            .unwrap();
        let r = out.req.unwrap();
        assert_eq!(r.headers["X-Trace"], "abc");
        assert_eq!(r.url, "{{base_url}}/x?ts=1");
        assert_eq!(out.vars["nonce"], "42");
        assert_eq!(out.logs, vec![r#"hola {"a":1}"#]);
    }

    #[test]
    fn post_lee_json_y_tests_estilo_chai() {
        let e = Engine::new().unwrap();
        let headers: IndexMap<String, String> =
            [("content-type".to_owned(), "application/json".to_owned())]
                .into_iter()
                .collect();
        let res = ScriptResponse {
            status: 201,
            headers: &headers,
            body: r#"{"id": 42, "tags": ["a","b"], "user": {"name": "Ada"}}"#.into(),
            elapsed_ms: 12,
            size: 50,
        };
        let out = e
            .run(
                "test",
                r#"
                harpoon.vars.set("id", harpoon.res.json().id);
                test("status 201", () => expect(harpoon.res.status).to.equal(201));
                test("tiene id", () => expect(harpoon.res.json()).to.have.property("id", 42));
                test("tags", () => expect(harpoon.res.json().tags).to.include("b").and.have.length(2));
                test("deep", () => expect(harpoon.res.json().user).to.deep.equal({name: "Ada"}));
                test("header", () => expect(harpoon.res.header("Content-Type")).to.contain("json"));
                test("falla", () => expect(harpoon.res.status).to.be.below(200));
                test("not", () => expect(harpoon.res.status).to.not.equal(500));
                test("tiempo", () => expect(harpoon.res.elapsed_ms).to.be.below(100));
                test("tipo", () => expect(harpoon.res.json().id).to.be.a("number"));
                test("array", () => expect(harpoon.res.json().tags).to.be.an("array").that.is.not.empty);
                test("oneOf", () => expect(harpoon.res.status).to.be.oneOf([200, 201]));
                "#,
                Some(&req()),
                Some(&res),
                &IndexMap::new(),
                &IndexMap::new(),
            )
            .unwrap();
        assert_eq!(out.vars["id"], "42");
        let passed: Vec<bool> = out.tests.iter().map(|t| t.passed).collect();
        assert_eq!(
            passed,
            vec![
                true, true, true, true, true, false, true, true, true, true, true
            ]
        );
        assert_eq!(
            out.tests[5].detail.as_deref(),
            Some("esperaba < 200, recibido 201")
        );
    }

    #[test]
    fn error_de_sintaxis_y_timeout() {
        let e = Engine::new().unwrap();
        let err = e
            .run(
                "pre",
                "let x = ;",
                Some(&req()),
                None,
                &IndexMap::new(),
                &IndexMap::new(),
            )
            .unwrap_err();
        assert!(matches!(err, Error::Script { phase: "pre", .. }), "{err}");

        let err = e
            .run(
                "pre",
                "while(true){}",
                Some(&req()),
                None,
                &IndexMap::new(),
                &IndexMap::new(),
            )
            .unwrap_err();
        assert!(matches!(err, Error::Timeout(..)), "{err}");
    }

    #[test]
    fn sin_acceso_a_io() {
        let e = Engine::new().unwrap();
        for src in [
            "fetch('http://x')",
            "require('fs')",
            "std.open('/etc/passwd')",
        ] {
            let err = e
                .run(
                    "pre",
                    src,
                    Some(&req()),
                    None,
                    &IndexMap::new(),
                    &IndexMap::new(),
                )
                .unwrap_err();
            assert!(matches!(err, Error::Script { .. }), "{src}: {err}");
        }
    }
}
