//! `harpoon mcp`: servidor MCP (Model Context Protocol) por stdio para que
//! asistentes como Claude Code o Codex consulten la colección.
//!
//! Solo lectura y sin secretos: todo pasa por `harpoon_core::redact`. La
//! colección se relee del disco en cada llamada, así siempre está al día.
//! Protocolo: JSON-RPC 2.0, un mensaje por línea en stdin/stdout; los
//! diagnósticos van a stderr para no romper el canal.

use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

use anyhow::Context;
use harpoon_core::{Collection, redact, validate};
use serde_json::{Value, json};

/// Versiones del protocolo que entendemos (la primera es la preferida).
const PROTOCOL_VERSIONS: [&str; 3] = ["2025-06-18", "2025-03-26", "2024-11-05"];

const INSTRUCTIONS: &str = "Colección de APIs de Harpoon (archivos TOML versionados con Git). \
Usa list_requests para ver qué hay, get_request para el detalle de una solicitud y resolve_url \
para la URL final en un entorno. Los secretos nunca se muestran: aparecen como {{variable}} o ‹oculto›. \
Para crear o editar solicitudes, edita los archivos .toml de la colección (formato en docs/format.md).";

pub struct Server {
    root: PathBuf,
    environment: Option<String>,
}

impl Server {
    pub fn new(root: &Path, environment: Option<String>) -> Self {
        Self {
            root: root.to_path_buf(),
            environment,
        }
    }

    /// Atiende mensajes hasta que se cierra stdin.
    pub fn run(&self, input: impl BufRead, mut output: impl Write) -> anyhow::Result<()> {
        for line in input.lines() {
            let line = line.context("leyendo stdin")?;
            if line.trim().is_empty() {
                continue;
            }
            if let Some(reply) = self.handle_line(&line) {
                writeln!(output, "{reply}")?;
                output.flush()?;
            }
        }
        Ok(())
    }

    /// Respuesta a un mensaje (o nada si es una notificación).
    pub fn handle_line(&self, line: &str) -> Option<Value> {
        let msg: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(e) => return Some(error(&Value::Null, -32700, &format!("JSON no válido: {e}"))),
        };
        let id = msg.get("id")?.clone(); // sin id = notificación: no se responde
        let method = msg["method"].as_str().unwrap_or_default();
        let params = &msg["params"];
        Some(match method {
            "initialize" => {
                let asked = params["protocolVersion"].as_str().unwrap_or_default();
                let version = PROTOCOL_VERSIONS
                    .iter()
                    .find(|v| **v == asked)
                    .unwrap_or(&PROTOCOL_VERSIONS[0]);
                ok(
                    &id,
                    json!({
                        "protocolVersion": version,
                        "capabilities": {"tools": {}},
                        "serverInfo": {"name": "harpoon", "title": "Harpoon", "version": env!("CARGO_PKG_VERSION")},
                        "instructions": INSTRUCTIONS,
                    }),
                )
            }
            "ping" => ok(&id, json!({})),
            "tools/list" => ok(&id, json!({"tools": tools()})),
            "tools/call" => {
                let name = params["name"].as_str().unwrap_or_default();
                let args = params
                    .get("arguments")
                    .cloned()
                    .unwrap_or_else(|| json!({}));
                match self.call(name, &args) {
                    Ok(result) => ok(
                        &id,
                        json!({
                            "content": [{"type": "text", "text": serde_json::to_string_pretty(&result).unwrap_or_default()}],
                            "structuredContent": result,
                            "isError": false,
                        }),
                    ),
                    Err(e) => ok(
                        &id,
                        json!({"content": [{"type": "text", "text": e.to_string()}], "isError": true}),
                    ),
                }
            }
            _ => error(&id, -32601, &format!("método no soportado: {method}")),
        })
    }

    fn collection(&self) -> anyhow::Result<Collection> {
        Collection::load(&self.root)
            .with_context(|| format!("no se pudo cargar la colección en {}", self.root.display()))
    }

    fn call(&self, tool: &str, args: &Value) -> anyhow::Result<Value> {
        let c = self.collection()?;
        let env = args["environment"]
            .as_str()
            .map(str::to_owned)
            .or_else(|| self.environment.clone());
        match tool {
            "list_requests" => {
                let folder = args["folder"]
                    .as_str()
                    .unwrap_or_default()
                    .trim_matches('/');
                let query = args["query"].as_str().unwrap_or_default().to_lowercase();
                let requests: Vec<Value> = c
                    .requests
                    .iter()
                    .filter(|(k, _)| folder.is_empty() || k.starts_with(&format!("{folder}/")))
                    .map(|(k, r)| redact::request(&c, k, r))
                    .filter(|r| query.is_empty() || [&r.key, &r.name, &r.url].iter().any(|s| s.to_lowercase().contains(&query)))
                    .map(|r| json!({"key": r.key, "name": r.name, "type": r.kind, "method": r.method, "url": r.url}))
                    .collect();
                Ok(
                    json!({"collection": c.meta.name, "count": requests.len(), "requests": requests}),
                )
            }
            "get_request" => {
                let key = required(args, "key")?;
                let req = c
                    .requests
                    .get(key)
                    .with_context(|| format!("no existe la solicitud `{key}`"))?;
                Ok(serde_json::to_value(redact::request(&c, key, req))?)
            }
            "list_environments" => {
                let mut envs = serde_json::Map::new();
                for (name, e) in &c.environments {
                    envs.insert(
                        name.clone(),
                        json!({"vars": redact::vars(&e.vars, &e.secrets)}),
                    );
                }
                let mut folders = serde_json::Map::new();
                for (name, cfg) in &c.folder_configs {
                    if !cfg.vars.is_empty() || !cfg.secrets.is_empty() {
                        folders.insert(name.clone(), json!(redact::vars(&cfg.vars, &cfg.secrets)));
                    }
                }
                Ok(json!({
                    "default_environment": c.meta.default_environment,
                    "collection_vars": redact::vars(&c.meta.vars, &c.meta.secrets),
                    "folder_vars": folders,
                    "environments": envs,
                    "note": "Las variables con valor null son secretas: existen pero su valor no se comparte.",
                }))
            }
            "resolve_url" => {
                let key = required(args, "key")?;
                anyhow::ensure!(
                    c.requests.contains_key(key),
                    "no existe la solicitud `{key}`"
                );
                let url = redact::resolved_url(&c, key, env.as_deref())?;
                Ok(
                    json!({"key": key, "environment": env.or(c.meta.default_environment.clone()), "url": url}),
                )
            }
            "validate" => {
                let issues = validate::validate(&c, env.as_deref());
                Ok(json!({"count": issues.len(), "issues": issues}))
            }
            other => anyhow::bail!("herramienta desconocida: {other}"),
        }
    }
}

fn required<'a>(args: &'a Value, name: &str) -> anyhow::Result<&'a str> {
    args[name]
        .as_str()
        .filter(|s| !s.is_empty())
        .with_context(|| format!("falta el argumento `{name}`"))
}

fn ok(id: &Value, result: Value) -> Value {
    let mut reply = serde_json::Map::new();
    reply.insert("jsonrpc".into(), "2.0".into());
    reply.insert("id".into(), id.clone());
    reply.insert("result".into(), result);
    Value::Object(reply)
}

fn error(id: &Value, code: i64, message: &str) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
}

fn tools() -> Value {
    let read_only = json!({"readOnlyHint": true, "openWorldHint": false});
    let env = json!({"type": "string", "description": "Entorno (por defecto, el de la colección o el de --env)"});
    json!([
        {
            "name": "list_requests",
            "title": "Listar solicitudes",
            "description": "Solicitudes de la colección con su clave, método y URL (con {{variables}} sin resolver). Filtra por carpeta o por texto.",
            "inputSchema": {"type": "object", "properties": {
                "folder": {"type": "string", "description": "Solo las de esta carpeta (p. ej. `users`)"},
                "query": {"type": "string", "description": "Texto a buscar en clave, nombre o URL"}
            }},
            "annotations": read_only,
        },
        {
            "name": "get_request",
            "title": "Ver una solicitud",
            "description": "Detalle de una solicitud: método, URL, query, cabeceras (incluidas las heredadas de carpetas), auth efectiva y su origen, cuerpo, scripts y docs. Sin secretos.",
            "inputSchema": {"type": "object", "properties": {
                "key": {"type": "string", "description": "Clave de la solicitud, p. ej. `users/list`"}
            }, "required": ["key"]},
            "annotations": read_only,
        },
        {
            "name": "list_environments",
            "title": "Ver entornos y variables",
            "description": "Entornos y variables de colección y carpetas. Las secretas aparecen con valor null.",
            "inputSchema": {"type": "object", "properties": {}},
            "annotations": read_only,
        },
        {
            "name": "resolve_url",
            "title": "Resolver URL",
            "description": "URL final de una solicitud en un entorno, con query. Las variables secretas quedan como {{nombre}}.",
            "inputSchema": {"type": "object", "properties": {
                "key": {"type": "string", "description": "Clave de la solicitud"},
                "environment": env
            }, "required": ["key"]},
            "annotations": read_only,
        },
        {
            "name": "validate",
            "title": "Validar colección",
            "description": "Problemas de la colección sin enviar nada: variables sin definir, URLs, JSON, archivos y secretos sin valor.",
            "inputSchema": {"type": "object", "properties": {"environment": env}},
            "annotations": read_only,
        }
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use harpoon_core::{Method, Request};

    fn server() -> (tempfile::TempDir, Server) {
        let temp = tempfile::tempdir().unwrap();
        let mut c = Collection::init(temp.path(), "demo").unwrap();
        let mut env = c.environments["dev"].clone();
        env.vars
            .insert("api_key".into(), "valor-super-secreto".into());
        env.secrets.push("api_key".into());
        c.save_environment("dev", env).unwrap();
        let mut req = Request::new("Usuarios", Method::Get, "{{base_url}}/users?token=abc123");
        req.headers.insert("X-Api-Key".into(), "{{api_key}}".into());
        req.headers
            .insert("Authorization".into(), "Bearer literal-123".into());
        c.save_request("users/list", req).unwrap();
        let s = Server::new(temp.path(), None);
        (temp, s)
    }

    #[allow(clippy::needless_pass_by_value)]
    fn call(s: &Server, name: &str, args: Value) -> Value {
        let line = json!({"jsonrpc": "2.0", "id": 7, "method": "tools/call", "params": {"name": name, "arguments": args}});
        let reply = s.handle_line(&line.to_string()).unwrap();
        assert_eq!(reply["result"]["isError"], false, "{reply}");
        reply["result"]["structuredContent"].clone()
    }

    #[test]
    fn negocia_version_y_lista_herramientas() {
        let (_t, s) = server();
        let init = s.handle_line(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"t","version":"1"}}}"#).unwrap();
        assert_eq!(init["result"]["protocolVersion"], "2025-03-26");
        assert_eq!(init["result"]["serverInfo"]["name"], "harpoon");
        assert!(
            s.handle_line(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#)
                .is_none()
        );
        let list = s
            .handle_line(r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#)
            .unwrap();
        let names: Vec<_> = list["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["name"].as_str().unwrap())
            .collect();
        assert_eq!(
            names,
            [
                "list_requests",
                "get_request",
                "list_environments",
                "resolve_url",
                "validate"
            ]
        );
        let unknown = s
            .handle_line(r#"{"jsonrpc":"2.0","id":3,"method":"resources/list"}"#)
            .unwrap();
        assert_eq!(unknown["error"]["code"], -32601);
    }

    #[test]
    fn nunca_devuelve_secretos() {
        let (_t, s) = server();
        let mut everything = String::new();
        for (tool, args) in [
            ("list_requests", json!({})),
            ("get_request", json!({"key": "users/list"})),
            ("list_environments", json!({})),
            ("resolve_url", json!({"key": "users/list"})),
            ("validate", json!({})),
        ] {
            everything.push_str(&call(&s, tool, args).to_string());
        }
        for secret in ["valor-super-secreto", "abc123", "literal-123"] {
            assert!(
                !everything.contains(secret),
                "se ha filtrado {secret}: {everything}"
            );
        }
        let req = call(&s, "get_request", json!({"key": "users/list"}));
        assert_eq!(req["headers"]["X-Api-Key"], "{{api_key}}");
        let url = call(&s, "resolve_url", json!({"key": "users/list"}));
        assert_eq!(url["url"], "http://localhost:8080/users?token=‹oculto›");
        let envs = call(&s, "list_environments", json!({}));
        assert_eq!(envs["environments"]["dev"]["vars"]["api_key"], Value::Null);
    }

    #[test]
    fn errores_como_resultado_de_herramienta() {
        let (_t, s) = server();
        let line = json!({"jsonrpc": "2.0", "id": 9, "method": "tools/call", "params": {"name": "get_request", "arguments": {"key": "nope"}}});
        let reply = s.handle_line(&line.to_string()).unwrap();
        assert_eq!(reply["result"]["isError"], true);
        assert!(
            reply["result"]["content"][0]["text"]
                .as_str()
                .unwrap()
                .contains("nope")
        );
    }
}
