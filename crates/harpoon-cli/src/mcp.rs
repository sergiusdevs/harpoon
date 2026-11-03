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
use indexmap::IndexMap;
use serde_json::{Value, json};

/// Versiones del protocolo que entendemos (la primera es la preferida).
const PROTOCOL_VERSIONS: [&str; 3] = ["2025-06-18", "2025-03-26", "2024-11-05"];

const INSTRUCTIONS: &str = "Colección de APIs de Harpoon (archivos TOML versionados con Git). \
Usa list_requests para ver qué hay, get_request para el detalle de una solicitud y resolve_url \
para la URL final en un entorno. Los secretos nunca se muestran: aparecen como {{variable}} o ‹oculto›. \
Si el servidor tiene escritura activada (--allow-write), crea carpetas con save_folder y solicitudes \
con save_request; pon la URL base en las variables de la carpeta y usa {{variable}} en las URLs. \
Nunca escribas credenciales: usa {{variable}} y pide al usuario que la marque como secreta en Harpoon.";

pub struct Server {
    root: PathBuf,
    environment: Option<String>,
    /// Activa `save_folder` y `save_request` (`harpoon mcp --allow-write`).
    allow_write: bool,
}

impl Server {
    pub fn new(root: &Path, environment: Option<String>, allow_write: bool) -> Self {
        Self {
            root: root.to_path_buf(),
            environment,
            allow_write,
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
            "tools/list" => ok(&id, json!({"tools": tools(self.allow_write)})),
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
            "save_folder" | "save_request" if !self.allow_write => {
                anyhow::bail!(
                    "escritura desactivada: arranca el servidor con `harpoon mcp --allow-write`"
                )
            }
            "save_folder" => save_folder(c, args),
            "save_request" => save_request(c, args),
            other => anyhow::bail!("herramienta desconocida: {other}"),
        }
    }
}

/// Rechaza valores que parecen credenciales: se guardarían en Git.
fn refuse_secrets(pairs: &IndexMap<String, String>, what: &str) -> anyhow::Result<()> {
    for (name, value) in pairs {
        if redact::looks_secret(name, value) {
            anyhow::bail!(
                "{what} `{name}` parece un secreto: usa {{{{{}}}}} y pide al usuario que la marque como secreta en Harpoon",
                name.to_lowercase().replace('-', "_")
            );
        }
    }
    Ok(())
}

fn map_arg(args: &Value, name: &str) -> anyhow::Result<IndexMap<String, String>> {
    match &args[name] {
        Value::Null => Ok(IndexMap::new()),
        v => serde_json::from_value(v.clone())
            .with_context(|| format!("`{name}` debe ser un objeto de textos")),
    }
}

fn save_folder(mut c: Collection, args: &Value) -> anyhow::Result<Value> {
    let folder = required(args, "folder")?.trim_matches('/').to_owned();
    let vars = map_arg(args, "vars")?;
    let headers = map_arg(args, "headers")?;
    refuse_secrets(&vars, "la variable")?;
    refuse_secrets(&headers, "la cabecera")?;
    c.create_folder(&folder)?;
    let current = c.folder_configs.get(&folder).cloned().unwrap_or_default();
    if !vars.is_empty() {
        let mut merged = current.vars.clone();
        merged.extend(vars);
        c.save_folder_config(&folder, merged, current.secrets.clone())?;
    }
    if !headers.is_empty() {
        let mut merged = current.headers.clone();
        merged.extend(headers);
        c.save_folder_settings(
            &folder,
            merged,
            current.auth.clone(),
            current.scripts.clone(),
        )?;
    }
    let c = Collection::load(&c.root)?;
    Ok(json!({
        "saved": format!("requests/{folder}/folder.toml"),
        "vars": c.folder_configs.get(&folder).map(|f| redact::vars(&f.vars, &f.secrets)),
    }))
}

fn save_request(mut c: Collection, args: &Value) -> anyhow::Result<Value> {
    let key = required(args, "key")?.trim_matches('/').to_owned();
    if c.requests.contains_key(&key) && args["overwrite"] != true {
        anyhow::bail!("ya existe `{key}`; pasa overwrite: true para reemplazarla");
    }
    let method: harpoon_core::Method =
        serde_json::from_value(json!(required(args, "method")?.to_uppercase()))
            .context("método no válido (GET, POST, PUT, PATCH, DELETE, HEAD u OPTIONS)")?;
    let mut req = harpoon_core::Request::new(
        args["name"].as_str().unwrap_or(&key),
        method,
        required(args, "url")?,
    );
    req.query = map_arg(args, "query")?;
    req.headers = map_arg(args, "headers")?;
    if !args["body"].is_null() {
        req.body = serde_json::from_value(args["body"].clone())
            .context("`body` no válido: usa {\"type\": \"json\", \"json\": \"…\"} (o text, xml, graphql, form)")?;
    }
    args["docs"]
        .as_str()
        .unwrap_or_default()
        .clone_into(&mut req.docs);
    req.expect.status = args["expect_status"]
        .as_u64()
        .and_then(|s| u16::try_from(s).ok());
    refuse_secrets(&req.headers, "la cabecera")?;
    refuse_secrets(&req.query, "el parámetro")?;
    if let Some(f) = harpoon_core::scan::scan_request(&key, &req).first() {
        anyhow::bail!(
            "{} parece contener un secreto ({}): usa {{{{{}}}}}",
            f.location,
            f.kind,
            f.suggestion
        );
    }
    if let Some((parent, _)) = key.rsplit_once('/') {
        c.create_folder(parent)?;
    }
    c.save_request(&key, req)?;
    let c = Collection::load(&c.root)?;
    Ok(json!({
        "saved": format!("requests/{key}.toml"),
        "request": redact::request(&c, &key, &c.requests[&key]),
        "resolved_url": redact::resolved_url(&c, &key, None).ok(),
    }))
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

fn tools(allow_write: bool) -> Value {
    let read_only = json!({"readOnlyHint": true, "openWorldHint": false});
    let write = json!({"readOnlyHint": false, "destructiveHint": false, "idempotentHint": true, "openWorldHint": false});
    let pairs = json!({"type": "object", "additionalProperties": {"type": "string"}});
    let env = json!({"type": "string", "description": "Entorno (por defecto, el de la colección o el de --env)"});
    let mut list = json!([
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
    ]);
    if allow_write {
        list.as_array_mut().expect("lista").extend([
            json!({
                "name": "save_folder",
                "title": "Crear o completar una carpeta",
                "description": "Crea la carpeta (y sus padres) y añade variables y cabeceras que heredan sus solicitudes. Pon aquí la URL base (p. ej. vars: {\"base_url\": \"https://…\"}). Rechaza valores que parezcan secretos.",
                "inputSchema": {"type": "object", "properties": {
                    "folder": {"type": "string", "description": "Ruta de la carpeta, p. ej. `rick-and-morty/characters`"},
                    "vars": pairs.clone(),
                    "headers": pairs.clone()
                }, "required": ["folder"]},
                "annotations": write.clone(),
            }),
            json!({
                "name": "save_request",
                "title": "Crear una solicitud",
                "description": "Crea una solicitud HTTP en la colección. Usa {{variables}} en la URL. No sobrescribe salvo overwrite: true. Rechaza credenciales escritas tal cual.",
                "inputSchema": {"type": "object", "properties": {
                    "key": {"type": "string", "description": "Clave = ruta del archivo sin .toml, p. ej. `rick-and-morty/characters/list`"},
                    "name": {"type": "string", "description": "Nombre visible"},
                    "method": {"type": "string", "enum": ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"]},
                    "url": {"type": "string", "description": "URL, p. ej. `{{base_url}}/character/{{character_id}}`"},
                    "query": pairs.clone(),
                    "headers": pairs.clone(),
                    "body": {"type": "object", "description": "Cuerpo: {\"type\": \"json\", \"json\": \"{…}\"}, {\"type\": \"text\", \"text\": …}, {\"type\": \"graphql\", \"query\": …, \"variables\": \"{}\", \"operation_name\": \"\"} o {\"type\": \"form\", \"fields\": {…}}"},
                    "docs": {"type": "string", "description": "Documentación en Markdown"},
                    "expect_status": {"type": "integer", "description": "Status esperado (aserción)"},
                    "overwrite": {"type": "boolean", "description": "Reemplazar si ya existe"}
                }, "required": ["key", "method", "url"]},
                "annotations": write,
            }),
        ]);
    }
    list
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
        let s = Server::new(temp.path(), None, false);
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

    fn call_raw(s: &Server, name: &str, args: &Value) -> Value {
        let line = json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {"name": name, "arguments": args}});
        s.handle_line(&line.to_string()).unwrap()["result"].clone()
    }

    #[test]
    fn escritura_solo_con_allow_write() {
        let (temp, read_only) = server();
        let names = |s: &Server| -> Vec<String> {
            let list = s
                .handle_line(r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#)
                .unwrap();
            list["result"]["tools"]
                .as_array()
                .unwrap()
                .iter()
                .map(|t| t["name"].as_str().unwrap().to_owned())
                .collect()
        };
        assert!(!names(&read_only).contains(&"save_request".to_owned()));
        let refused = call_raw(&read_only, "save_folder", &json!({"folder": "x"}));
        assert_eq!(refused["isError"], true);
        let writer = Server::new(temp.path(), None, true);
        assert!(names(&writer).contains(&"save_request".to_owned()));
    }

    #[test]
    fn crea_carpetas_y_solicitudes_sin_secretos() {
        let (temp, _) = server();
        let s = Server::new(temp.path(), None, true);
        let folder = call_raw(
            &s,
            "save_folder",
            &json!({"folder": "rm", "vars": {"rm_url": "https://api.test"}, "headers": {"Accept": "application/json"}}),
        );
        assert_eq!(folder["isError"], false, "{folder}");
        let saved = call_raw(
            &s,
            "save_request",
            &json!({
                "key": "rm/characters/get", "name": "Personaje", "method": "get",
                "url": "{{rm_url}}/character/{{id}}", "query": {"page": "1"}, "expect_status": 200,
                "docs": "Un personaje por id"
            }),
        );
        assert_eq!(saved["isError"], false, "{saved}");
        let req = &saved["structuredContent"]["request"];
        assert_eq!(req["method"], "GET");
        assert_eq!(
            req["headers"]["Accept"], "application/json",
            "hereda de la carpeta"
        );
        assert_eq!(
            saved["structuredContent"]["resolved_url"],
            "https://api.test/character/{{id}}?page=1"
        );

        let col = Collection::load(temp.path()).unwrap();
        assert_eq!(col.requests["rm/characters/get"].expect.status, Some(200));
        assert_eq!(col.folder_configs["rm"].vars["rm_url"], "https://api.test");

        let again = call_raw(
            &s,
            "save_request",
            &json!({"key": "rm/characters/get", "method": "GET", "url": "x"}),
        );
        assert_eq!(again["isError"], true, "no sobrescribe sin overwrite");

        for args in [
            json!({"key": "rm/leak", "method": "GET", "url": "https://a.test", "headers": {"Authorization": "Bearer abc123"}}),
            json!({"key": "rm/leak", "method": "GET", "url": "https://a.test", "query": {"api_key": "s3cr3t"}}),
            json!({"key": "rm/leak", "method": "GET", "url": "https://a.test", "headers": {"X-Note": "ghp_abcdefghijklmnopqrstuvwxyz0123456789"}}),
        ] {
            let out = call_raw(&s, "save_request", &args);
            assert_eq!(out["isError"], true, "{args}");
        }
        let secret_var = call_raw(
            &s,
            "save_folder",
            &json!({"folder": "rm", "vars": {"token": "abc"}}),
        );
        assert_eq!(secret_var["isError"], true);
        assert!(
            !Collection::load(temp.path())
                .unwrap()
                .requests
                .contains_key("rm/leak")
        );
        // Con plantilla sí se permite.
        let ok = call_raw(
            &s,
            "save_request",
            &json!({"key": "rm/auth", "method": "GET", "url": "https://a.test", "headers": {"Authorization": "Bearer {{token}}"}}),
        );
        assert_eq!(ok["isError"], false, "{ok}");
    }
}
