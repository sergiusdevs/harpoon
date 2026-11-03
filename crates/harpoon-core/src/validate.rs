//! `harpoon validate`: revisa la colección sin enviar nada.
//!
//! Detecta lo que haría fallar una solicitud antes de ejecutarla: variables
//! sin definir, URLs mal formadas, JSON inválido, archivos que no existen,
//! aserciones imposibles y secretos sin valor en esta máquina.

use std::collections::BTreeSet;
use std::path::Path;

use indexmap::IndexMap;
use serde::Serialize;

use crate::collection::Collection;
use crate::model::{Auth, Body, MultipartPart, Request, RequestType};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Issue {
    pub severity: Severity,
    /// Archivo relativo a la colección (`requests/users/list.toml`).
    pub file: String,
    /// Solicitud afectada, si la hay (`users/list`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request: Option<String>,
    /// Dónde dentro del archivo (`url`, `header Authorization`, `body`…).
    pub location: String,
    pub message: String,
}

/// Nombres de `{{variables}}` usados en un texto (sin las dinámicas `{{$uuid}}`).
#[must_use]
pub fn variable_references(input: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = input;
    while let Some(start) = rest.find("{{") {
        let after = &rest[start + 2..];
        let Some(end) = after.find("}}") else { break };
        let name = after[..end].trim();
        if !name.is_empty() && !name.starts_with('$') {
            out.push(name.to_owned());
        }
        rest = &after[end + 2..];
    }
    out
}

/// Variables que algún script crea al ejecutarse (`harpoon.vars.set("token", …)`).
fn script_variables(c: &Collection) -> BTreeSet<String> {
    let mut scripts: Vec<&str> = vec![
        &c.meta.scripts.pre,
        &c.meta.scripts.post,
        &c.meta.scripts.test,
    ];
    for config in c.folder_configs.values() {
        scripts.extend([
            config.scripts.pre.as_str(),
            config.scripts.post.as_str(),
            config.scripts.test.as_str(),
        ]);
    }
    for req in c.requests.values() {
        scripts.extend([
            req.scripts.pre.as_str(),
            req.scripts.post.as_str(),
            req.scripts.test.as_str(),
        ]);
    }
    let mut out = BTreeSet::new();
    for script in scripts {
        let mut rest = script;
        while let Some(i) = rest.find("vars.set(") {
            let after = rest[i + 9..].trim_start();
            if let Some(quote) = after
                .chars()
                .next()
                .filter(|c| matches!(c, '"' | '\'' | '`'))
                && let Some(end) = after[1..].find(quote)
            {
                out.insert(after[1..=end].to_owned());
            }
            rest = &rest[i + 9..];
        }
    }
    out
}

/// Textos de una solicitud donde puede haber variables, con su ubicación.
fn texts(req: &Request) -> Vec<(String, &str)> {
    let mut out: Vec<(String, &str)> = vec![("url".into(), req.url.as_str())];
    for (k, v) in &req.headers {
        if !req.header_is_disabled(k) {
            out.push((format!("header {k}"), k.as_str()));
            out.push((format!("header {k}"), v.as_str()));
        }
    }
    for (k, v) in &req.query {
        if !req.disabled_query.contains(k) {
            out.push((format!("parámetro {k}"), v.as_str()));
        }
    }
    match &req.body {
        Body::Json { json } => out.push(("body".into(), json)),
        Body::Text { text } => out.push(("body".into(), text)),
        Body::Xml { xml } => out.push(("body".into(), xml)),
        Body::Graphql {
            query, variables, ..
        } => {
            out.push(("body (consulta)".into(), query));
            out.push(("body (variables)".into(), variables));
        }
        Body::Binary { path } => out.push(("body (archivo)".into(), path)),
        Body::Form { fields } => out.extend(
            fields
                .iter()
                .map(|(k, v)| (format!("campo {k}"), v.as_str())),
        ),
        Body::Multipart { parts } => {
            for part in parts {
                match part {
                    MultipartPart::Text { name, value } => {
                        out.push((format!("campo {name}"), value));
                    }
                    MultipartPart::File { name, path, .. } => {
                        out.push((format!("archivo {name}"), path));
                    }
                }
            }
        }
        Body::None => {}
    }
    out.extend(auth_texts(&req.auth));
    out
}

fn auth_texts(auth: &Auth) -> Vec<(String, &str)> {
    match auth {
        Auth::Basic { username, password } | Auth::Digest { username, password } => {
            vec![
                ("auth usuario".into(), username),
                ("auth contraseña".into(), password),
            ]
        }
        Auth::Bearer { token } => vec![("auth token".into(), token)],
        Auth::ApiKey { key, value, .. } => {
            vec![("auth clave".into(), key), ("auth valor".into(), value)]
        }
        Auth::OAuth2 {
            token_url,
            client_id,
            client_secret,
            scope,
            username,
            password,
            refresh_token,
            ..
        } => vec![
            ("auth token_url".into(), token_url),
            ("auth client_id".into(), client_id),
            ("auth client_secret".into(), client_secret),
            ("auth scope".into(), scope),
            ("auth usuario".into(), username),
            ("auth contraseña".into(), password),
            ("auth refresh_token".into(), refresh_token),
        ],
        Auth::Inherit | Auth::None => vec![],
    }
}

/// Revisa la colección con un entorno (o el predeterminado).
#[must_use]
pub fn validate(c: &Collection, environment: Option<&str>) -> Vec<Issue> {
    let mut issues = Vec::new();
    let mut push =
        |severity, file: String, request: Option<&str>, location: &str, message: String| {
            issues.push(Issue {
                severity,
                file,
                request: request.map(str::to_owned),
                location: location.to_owned(),
                message,
            });
        };

    let env = environment.or(c.meta.default_environment.as_deref());
    if let Some(name) = env.filter(|n| !c.environments.contains_key(*n)) {
        let message = format!("el entorno `{name}` no existe");
        push(
            Severity::Error,
            crate::collection::META_FILE.into(),
            None,
            "default_environment",
            message,
        );
        return issues;
    }
    let from_scripts = script_variables(c);
    let runtime = c.load_runtime_vars();
    check_files(c, &mut push);

    for (key, own) in &c.requests {
        let file = format!("{}/{key}.toml", crate::collection::REQUESTS_DIR);
        let req = c.with_folder_inheritance(key, own);
        let resolved_auth = match &req.auth {
            Auth::Inherit => &c.meta.auth,
            other => other,
        };
        let Ok(scope) = c.scope_for_request(env, key) else {
            continue;
        };
        let known = |name: &str| {
            scope.get(name).is_some() || runtime.contains_key(name) || from_scripts.contains(name)
        };

        let mut unknown: IndexMap<String, String> = IndexMap::new();
        for (location, text) in texts(&req).into_iter().chain(auth_texts(resolved_auth)) {
            for name in variable_references(text) {
                if !known(&name) {
                    unknown.entry(name).or_insert(location.clone());
                }
            }
        }
        for (name, location) in &unknown {
            push(
                Severity::Error,
                file.clone(),
                Some(key),
                location,
                format!("la variable `{{{{{name}}}}}` no está definida"),
            );
        }

        check_url(&req, &scope, &runtime, &mut |message| {
            push(Severity::Error, file.clone(), Some(key), "url", message);
        });
        check_body(c, &req, &mut |severity, location, message| {
            push(severity, file.clone(), Some(key), location, message);
        });
        if let Some(status) = req.expect.status.filter(|s| !(100..=599).contains(s)) {
            push(
                Severity::Error,
                file.clone(),
                Some(key),
                "expect.status",
                format!("{status} no es un código HTTP válido (100–599)"),
            );
        }
    }

    for missing in c.missing_secrets(env) {
        let file = match missing.scope {
            "folder" => format!(
                "{}/{}/folder.toml",
                crate::collection::REQUESTS_DIR,
                missing.owner
            ),
            "environment" => format!(
                "{}/{}.toml",
                crate::collection::ENVIRONMENTS_DIR,
                missing.owner
            ),
            _ => crate::collection::META_FILE.to_owned(),
        };
        push(
            Severity::Warning,
            file,
            None,
            &format!("secreto {}", missing.name),
            format!(
                "sin valor en esta máquina; defínelo en la app o con `{}`",
                missing.env_var
            ),
        );
    }
    issues.sort_by(|a, b| a.severity.cmp(&b.severity).then(a.file.cmp(&b.file)));
    issues
}

fn check_url(
    req: &Request,
    scope: &crate::vars::Scope,
    runtime: &IndexMap<String, String>,
    report: &mut dyn FnMut(String),
) {
    if req.url.trim().is_empty() {
        report("la URL está vacía".into());
        return;
    }
    // Solo se puede comprobar si todas sus variables tienen valor ya.
    let Ok(url) = scope
        .clone()
        .with_layer(runtime.clone())
        .interpolate(&req.url)
    else {
        return;
    };
    let schemes: &[&str] = if req.request_type() == RequestType::Websocket {
        &["ws", "wss"]
    } else {
        &["http", "https"]
    };
    match url::Url::parse(url.trim()) {
        Ok(parsed) if schemes.contains(&parsed.scheme()) => {}
        Ok(parsed) => report(format!(
            "esquema `{}` no válido; usa {}",
            parsed.scheme(),
            schemes.join(" o ")
        )),
        Err(e) => report(format!("URL no válida `{url}`: {e}")),
    }
}

fn check_body(c: &Collection, req: &Request, report: &mut dyn FnMut(Severity, &str, String)) {
    let exists = |path: &str| {
        path.contains("{{")
            || Path::new(path).is_absolute() && Path::new(path).exists()
            || c.root.join(path).exists()
    };
    match &req.body {
        Body::Json { json } if !json.trim().is_empty() && !json.contains("{{") => {
            if let Err(e) = serde_json::from_str::<serde_json::Value>(json) {
                report(Severity::Warning, "body", format!("JSON no válido: {e}"));
            }
        }
        Body::Graphql { query, .. } if query.trim().is_empty() => report(
            Severity::Error,
            "body",
            "la consulta GraphQL está vacía".into(),
        ),
        Body::Binary { path } if !exists(path) => report(
            Severity::Error,
            "body (archivo)",
            format!("no existe el archivo `{path}`"),
        ),
        Body::Multipart { parts } => {
            for part in parts {
                if let MultipartPart::File { name, path, .. } = part
                    && !exists(path)
                {
                    report(
                        Severity::Error,
                        &format!("archivo {name}"),
                        format!("no existe el archivo `{path}`"),
                    );
                }
            }
        }
        _ => {}
    }
}

/// Registra un problema: gravedad, archivo, solicitud, ubicación y mensaje.
type Report<'a> = dyn FnMut(Severity, String, Option<&str>, &str, String) + 'a;

fn check_files(c: &Collection, push: &mut Report<'_>) {
    let s = &c.meta.settings;
    for (field, path) in [
        ("settings.ca_cert", &s.ca_cert),
        ("settings.client_cert", &s.client_cert),
        ("settings.client_key", &s.client_key),
    ] {
        if let Some(path) = path
            .as_deref()
            .filter(|p| !c.root.join(p).exists() && !Path::new(p).exists())
        {
            push(
                Severity::Error,
                crate::collection::META_FILE.into(),
                None,
                field,
                format!("no existe el archivo `{path}`"),
            );
        }
    }
}
