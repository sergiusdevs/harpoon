//! Vista de la colección apta para compartir con un asistente (MCP): todo lo
//! necesario para entender y usar las APIs, nunca un secreto.
//!
//! Reglas:
//! - Las variables secretas nunca se resuelven: se muestran como `{{nombre}}`.
//! - Un valor escrito tal cual se oculta si su nombre suele llevar secretos
//!   (`Authorization`, `token`, `password`…) o si tiene formato de credencial.
//! - Las plantillas (`Bearer {{token}}`) se conservan: no contienen el valor.

use indexmap::IndexMap;
use serde::Serialize;

use crate::collection::Collection;
use crate::history::HIDDEN;
use crate::model::{Auth, Body, MultipartPart, Request};
use crate::scan::{is_sensitive_name, known_secret_in};

/// El texto sin sus `{{variables}}`: lo que está escrito tal cual.
#[must_use]
pub fn without_templates(value: &str) -> String {
    let mut rest = value.to_owned();
    while let (Some(a), Some(b)) = (rest.find("{{"), rest.find("}}")) {
        if b < a {
            break;
        }
        rest.replace_range(a..b + 2, "");
    }
    rest
}

/// ¿Este valor de `name` se ocultaría por parecer un secreto escrito tal cual?
#[must_use]
pub fn looks_secret(name: &str, value: &str) -> bool {
    safe_value(name, value) == HIDDEN
}

/// ¿Se puede mostrar este valor de `name`?
fn safe_value(name: &str, value: &str) -> String {
    let only_templates = without_templates(value);
    let literal = only_templates.trim();
    if known_secret_in(value).is_some() {
        return HIDDEN.into();
    }
    if is_sensitive_name(name) && !literal.is_empty() && !is_scheme_only(literal) {
        return HIDDEN.into();
    }
    value.to_owned()
}

/// `Bearer`, `Basic`… sin más: es la parte pública de una cabecera de auth.
fn is_scheme_only(text: &str) -> bool {
    ["bearer", "basic", "digest", "token", "apikey"].contains(&text.to_ascii_lowercase().as_str())
}

fn safe_text(text: &str) -> String {
    match known_secret_in(text) {
        Some((kind, _)) => format!("{HIDDEN} (posible {kind})"),
        None => text.to_owned(),
    }
}

fn safe_url(url: &str) -> String {
    let url = safe_text(url);
    let Some((base, query)) = url.split_once('?') else {
        return url;
    };
    let query: Vec<String> = query
        .split('&')
        .map(|pair| match pair.split_once('=') {
            Some((name, value)) => format!("{name}={}", safe_value(name, value)),
            None => pair.to_owned(),
        })
        .collect();
    format!("{base}?{}", query.join("&"))
}

#[derive(Debug, Serialize)]
pub struct SafeAuth {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(skip_serializing_if = "IndexMap::is_empty")]
    pub fields: IndexMap<String, String>,
}

#[must_use]
pub fn auth(a: &Auth) -> SafeAuth {
    let mut fields = IndexMap::new();
    let mut put = |name: &str, value: &str| {
        if !value.is_empty() {
            fields.insert(name.to_owned(), safe_value(name, value));
        }
    };
    let kind = match a {
        Auth::Inherit => "inherit",
        Auth::None => "none",
        Auth::Basic { username, password } => {
            put("username", username);
            put("password", password);
            "basic"
        }
        Auth::Digest { username, password } => {
            put("username", username);
            put("password", password);
            "digest"
        }
        Auth::Bearer { token } => {
            put("token", token);
            "bearer"
        }
        Auth::ApiKey { key, value, r#in } => {
            put("key", key);
            fields.insert("value".into(), safe_value("apikey", value));
            fields.insert("in".into(), format!("{in:?}").to_lowercase());
            "api_key"
        }
        Auth::OAuth2 {
            grant,
            token_url,
            client_id,
            client_secret,
            scope,
            username,
            password,
            refresh_token,
            ..
        } => {
            put("grant", &format!("{grant:?}"));
            put("token_url", token_url);
            put("client_id", client_id);
            put("client_secret", client_secret);
            put("scope", scope);
            put("username", username);
            put("password", password);
            put("refresh_token", refresh_token);
            "oauth2"
        }
    };
    SafeAuth {
        kind: kind.into(),
        fields,
    }
}

#[derive(Debug, Serialize)]
pub struct SafeRequest {
    pub key: String,
    pub name: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub method: String,
    /// URL con `{{variables}}` sin resolver.
    pub url: String,
    #[serde(skip_serializing_if = "IndexMap::is_empty")]
    pub query: IndexMap<String, String>,
    /// Cabeceras efectivas: las heredadas de carpetas y las propias.
    #[serde(skip_serializing_if = "IndexMap::is_empty")]
    pub headers: IndexMap<String, String>,
    pub auth: SafeAuth,
    /// De dónde sale la auth cuando la solicitud la hereda.
    pub auth_source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "IndexMap::is_empty")]
    pub scripts: IndexMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expect_status: Option<u16>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub docs: String,
}

/// Solicitud con lo heredado de sus carpetas y sin secretos.
#[must_use]
pub fn request(c: &Collection, key: &str, own: &Request) -> SafeRequest {
    let req = c.with_folder_inheritance(key, own);
    let (effective, source) = match &req.auth {
        Auth::Inherit => (&c.meta.auth, "colección".to_owned()),
        a if matches!(own.auth, Auth::Inherit) => {
            let folder = c
                .folder_chain(key)
                .into_iter()
                .rev()
                .find(|(_, cfg)| &cfg.auth == a)
                .map_or_else(String::new, |(f, _)| f);
            (a, format!("carpeta {folder}"))
        }
        a => (a, "solicitud".to_owned()),
    };
    let headers = req
        .headers
        .iter()
        .filter(|(k, _)| !req.header_is_disabled(k))
        .map(|(k, v)| (k.clone(), safe_value(k, v)))
        .collect();
    let query = req
        .query
        .iter()
        .filter(|(k, _)| !req.disabled_query.contains(k))
        .map(|(k, v)| (k.clone(), safe_value(k, v)))
        .collect();
    let body = match &req.body {
        Body::None => None,
        Body::Json { json } => Some(serde_json::json!({"type": "json", "json": safe_text(json)})),
        Body::Text { text } => Some(serde_json::json!({"type": "text", "text": safe_text(text)})),
        Body::Xml { xml } => Some(serde_json::json!({"type": "xml", "xml": safe_text(xml)})),
        Body::Graphql {
            query,
            variables,
            operation_name,
            ..
        } => Some(serde_json::json!({
            "type": "graphql", "query": safe_text(query), "variables": safe_text(variables), "operation_name": operation_name,
        })),
        Body::Binary { path } => Some(serde_json::json!({"type": "binary", "path": path})),
        Body::Form { fields } => Some(serde_json::json!({
            "type": "form",
            "fields": fields.iter().map(|(k, v)| (k.clone(), safe_value(k, v))).collect::<IndexMap<_, _>>(),
        })),
        Body::Multipart { parts } => Some(serde_json::json!({
            "type": "multipart",
            "parts": parts.iter().map(|p| match p {
                MultipartPart::Text { name, value } => serde_json::json!({"name": name, "value": safe_value(name, value)}),
                MultipartPart::File { name, path, .. } => serde_json::json!({"name": name, "file": path}),
            }).collect::<Vec<_>>(),
        })),
    };
    let mut scripts = IndexMap::new();
    for (phase, text) in [
        ("pre", &own.scripts.pre),
        ("post", &own.scripts.post),
        ("test", &own.scripts.test),
    ] {
        if !text.trim().is_empty() {
            scripts.insert(phase.to_owned(), safe_text(text));
        }
    }
    SafeRequest {
        key: key.to_owned(),
        name: req.name.clone(),
        kind: format!("{:?}", req.request_type()).to_lowercase(),
        method: format!("{:?}", req.method).to_uppercase(),
        url: safe_url(&req.url),
        query,
        headers,
        auth: auth(effective),
        auth_source: source,
        body,
        scripts,
        expect_status: req.expect.status,
        docs: req.docs.clone(),
    }
}

/// Variables visibles: las secretas sin valor (`null`), el resto enmascaradas si hace falta.
#[must_use]
pub fn vars(
    values: &IndexMap<String, String>,
    secrets: &[String],
) -> IndexMap<String, Option<String>> {
    let mut out: IndexMap<String, Option<String>> = values
        .iter()
        .map(|(k, v)| (k.clone(), (!secrets.contains(k)).then(|| safe_value(k, v))))
        .collect();
    for name in secrets {
        out.entry(name.clone()).or_insert(None);
    }
    out
}

/// URL de una solicitud con las variables no secretas resueltas en un entorno;
/// las secretas (y las que no tienen valor) quedan como `{{nombre}}`.
pub fn resolved_url(c: &Collection, key: &str, environment: Option<&str>) -> crate::Result<String> {
    let req = c
        .requests
        .get(key)
        .ok_or_else(|| crate::Error::UndefinedVariable(key.to_owned()))?;
    let req = c.with_folder_inheritance(key, req);
    let env = environment.or(c.meta.default_environment.as_deref());
    let mut values: IndexMap<String, Option<String>> = vars(&c.meta.vars, &c.meta.secrets);
    for (_, cfg) in c.folder_chain(key) {
        values.extend(vars(&cfg.vars, &cfg.secrets));
    }
    if let Some(name) = env {
        let e = c
            .environments
            .get(name)
            .ok_or_else(|| crate::Error::UnknownEnvironment(name.to_owned()))?;
        values.extend(vars(&e.vars, &e.secrets));
    }
    let mut url = req.url.clone();
    for (name, value) in &values {
        if let Some(value) = value.as_deref().filter(|v| *v != HIDDEN) {
            url = url.replace(&format!("{{{{{name}}}}}"), value);
        }
    }
    let query: Vec<String> = req
        .query
        .iter()
        .filter(|(k, _)| !req.disabled_query.contains(k))
        .map(|(k, v)| format!("{k}={v}"))
        .collect();
    if !query.is_empty() {
        url.push(if url.contains('?') { '&' } else { '?' });
        url.push_str(&query.join("&"));
    }
    Ok(safe_url(&url))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plantillas_si_valores_no() {
        assert_eq!(
            safe_value("Authorization", "Bearer {{token}}"),
            "Bearer {{token}}"
        );
        assert_eq!(safe_value("Authorization", "Bearer abc123"), HIDDEN);
        assert_eq!(safe_value("X-Api-Key", "{{api_key}}"), "{{api_key}}");
        assert_eq!(safe_value("X-Api-Key", "s3cr3t"), HIDDEN);
        assert_eq!(safe_value("Accept", "application/json"), "application/json");
        // Formato de credencial conocido aunque el nombre sea inocente.
        assert_eq!(
            safe_value(
                "X-Note",
                concat!("ghp", "_abcdefghijklmnopqrstuvwxyz0123456789")
            ),
            HIDDEN
        );
    }

    #[test]
    fn urls_ocultan_parametros_sensibles() {
        assert_eq!(
            safe_url("https://a.test/x?page=1&access_token=abc&key={{k}}"),
            format!("https://a.test/x?page=1&access_token={HIDDEN}&key={{{{k}}}}")
        );
    }

    #[test]
    fn variables_secretas_sin_valor() {
        let values: IndexMap<String, String> = [
            ("base_url".to_owned(), "https://a.test".to_owned()),
            ("password".to_owned(), "hunter2".to_owned()),
            ("api_key".to_owned(), "valor-real".to_owned()),
        ]
        .into();
        let out = vars(&values, &["api_key".into(), "solo_local".into()]);
        assert_eq!(out["base_url"].as_deref(), Some("https://a.test"));
        assert_eq!(out["password"].as_deref(), Some(HIDDEN));
        assert_eq!(out["api_key"], None);
        assert_eq!(out["solo_local"], None);
    }
}
