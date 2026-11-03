//! Detector de secretos en lo que va a Git.
//!
//! Dos reglas, sin dependencias:
//! - **Formato conocido** en cualquier texto (URL, headers, cuerpo, scripts,
//!   variables): JWT, claves de AWS, tokens de GitHub, Slack, Stripe,
//!   `OpenAI`, Anthropic, Google y claves privadas PEM.
//! - **Campo sensible con valor literal**: auth (token, password,
//!   `client_secret`…), headers como `Authorization` o `Cookie`, parámetros o
//!   variables llamadas `token`, `password`, `api_key`… cuyo valor no es una
//!   `{{variable}}`.
//!
//! Las variables marcadas como secretas no se revisan: su valor ya no va a Git.

use serde::Serialize;

use crate::{Auth, Body, Collection, Request};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Finding {
    /// Archivo relativo a la colección (`requests/users/list.toml`).
    pub file: String,
    /// Dónde dentro del archivo (`header Authorization`, `auth.token`…).
    pub location: String,
    /// Qué parece (`token de GitHub`, `valor literal en campo sensible`…).
    pub kind: String,
    /// Valor recortado para reconocerlo sin mostrarlo entero.
    pub preview: String,
    /// Nombre de variable sugerido para convertirlo en secreto.
    pub suggestion: String,
}

/// Recorta un valor: `ghp_…(40)`.
#[must_use]
pub fn redact(value: &str) -> String {
    let value = value.trim();
    let head: String = value.chars().take(4).collect();
    format!("{head}…({})", value.chars().count())
}

/// ¿El nombre de un header, parámetro o variable suele llevar un secreto?
#[must_use]
pub fn is_sensitive_name(name: &str) -> bool {
    const EXACT: [&str; 6] = [
        "authorization",
        "proxyauthorization",
        "cookie",
        "auth",
        "pwd",
        "pass",
    ];
    const PARTS: [&str; 10] = [
        "password",
        "passwd",
        "secret",
        "token",
        "apikey",
        "accesskey",
        "privatekey",
        "credential",
        "signature",
        "sessionid",
    ];
    let n: String = name
        .to_ascii_lowercase()
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .collect();
    EXACT.contains(&n.as_str()) || PARTS.iter().any(|p| n.contains(p))
}

/// Quita las `{{variables}}` y los prefijos de esquema (`Bearer`, `Basic`…);
/// lo que queda es lo que se escribió a mano.
fn literal_part(value: &str) -> String {
    let mut out = String::new();
    let mut rest = value;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        match rest[start..].find("}}") {
            Some(end) => rest = &rest[start + end + 2..],
            None => {
                rest = "";
            }
        }
    }
    out.push_str(rest);
    let trimmed = out.trim();
    for scheme in ["Bearer", "Basic", "Token", "Digest", "ApiKey"] {
        if let Some(rest) = trimmed.strip_prefix(scheme) {
            return rest.trim().to_owned();
        }
    }
    trimmed.to_owned()
}

fn is_b64url(s: &str) -> bool {
    !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'=')
}

/// Nombre del formato si `token` parece una credencial conocida.
fn known_token(token: &str) -> Option<&'static str> {
    let alnum = |s: &str| s.bytes().all(|b| b.is_ascii_alphanumeric());
    let tail = |prefix: &str, min: usize| {
        token.strip_prefix(prefix).is_some_and(|t| {
            t.len() >= min
                && t.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        })
    };
    if token.starts_with("eyJ") {
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() == 3 && parts.iter().all(|p| is_b64url(p)) && parts[1].len() >= 8 {
            return Some("token JWT");
        }
    }
    if (token.starts_with("AKIA") || token.starts_with("ASIA"))
        && token.len() == 20
        && token
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
    {
        return Some("clave de acceso de AWS");
    }
    if ["ghp_", "gho_", "ghu_", "ghs_", "ghr_"]
        .iter()
        .any(|p| tail(p, 30))
        || tail("github_pat_", 30)
    {
        return Some("token de GitHub");
    }
    if tail("glpat-", 20) {
        return Some("token de GitLab");
    }
    if ["xoxb-", "xoxp-", "xoxa-", "xoxr-", "xoxs-"]
        .iter()
        .any(|p| tail(p, 10))
    {
        return Some("token de Slack");
    }
    if ["sk_live_", "rk_live_", "sk_test_", "rk_test_"]
        .iter()
        .any(|p| tail(p, 16))
    {
        return Some("clave de Stripe");
    }
    if tail("sk-ant-", 20) {
        return Some("clave de Anthropic");
    }
    if tail("sk-proj-", 20) || (tail("sk-", 32) && alnum(&token[3..])) {
        return Some("clave de OpenAI");
    }
    if token.starts_with("AIza") && token.len() == 39 && is_b64url(token) {
        return Some("clave de Google");
    }
    if tail("npm_", 30) {
        return Some("token de npm");
    }
    None
}

/// Busca credenciales con formato conocido dentro de un texto libre.
#[must_use]
pub fn known_secret_in(text: &str) -> Option<(&'static str, String)> {
    if text.contains("-----BEGIN") && text.contains("PRIVATE KEY") {
        return Some(("clave privada", "-----BEGIN … PRIVATE KEY-----".into()));
    }
    text.split(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.')))
        .find_map(|t| known_token(t.trim_matches('.')).map(|kind| (kind, redact(t))))
}

struct Scanner<'a> {
    file: &'a str,
    out: Vec<Finding>,
}

impl Scanner<'_> {
    fn push(&mut self, location: String, kind: &str, preview: String, suggestion: &str) {
        self.out.push(Finding {
            file: self.file.to_owned(),
            location,
            kind: kind.to_owned(),
            preview,
            suggestion: suggest(suggestion),
        });
    }

    /// Texto libre: solo formatos conocidos.
    fn text(&mut self, location: &str, value: &str, suggestion: &str) {
        if let Some((kind, preview)) = known_secret_in(value) {
            self.push(location.to_owned(), kind, preview, suggestion);
        }
    }

    /// Campo sensible: formato conocido o cualquier literal de 6+ caracteres.
    fn sensitive(&mut self, location: &str, value: &str, suggestion: &str) {
        if let Some((kind, preview)) = known_secret_in(value) {
            self.push(location.to_owned(), kind, preview, suggestion);
            return;
        }
        let literal = literal_part(value);
        if literal.chars().count() >= 6 {
            self.push(
                location.to_owned(),
                "valor literal en un campo sensible",
                redact(&literal),
                suggestion,
            );
        }
    }

    /// Pares nombre/valor: sensibles por nombre, el resto por formato.
    fn pairs<'k>(&mut self, what: &str, pairs: impl IntoIterator<Item = (&'k String, &'k String)>) {
        for (name, value) in pairs {
            let location = format!("{what} {name}");
            if is_sensitive_name(name) {
                self.sensitive(&location, value, name);
            } else {
                self.text(&location, value, name);
            }
        }
    }

    fn auth(&mut self, prefix: &str, auth: &Auth) {
        let at = |field: &str| format!("{prefix}auth.{field}");
        match auth {
            Auth::Bearer { token } => self.sensitive(&at("token"), token, "token"),
            Auth::Basic { password, .. } | Auth::Digest { password, .. } => {
                self.sensitive(&at("password"), password, "password");
            }
            Auth::ApiKey { key, value, .. } => self.sensitive(
                &at("value"),
                value,
                if key.is_empty() { "api_key" } else { key },
            ),
            Auth::OAuth2 {
                client_secret,
                password,
                refresh_token,
                ..
            } => {
                self.sensitive(&at("client_secret"), client_secret, "client_secret");
                self.sensitive(&at("password"), password, "password");
                self.sensitive(&at("refresh_token"), refresh_token, "refresh_token");
            }
            _ => {}
        }
    }

    fn request(&mut self, req: &Request) {
        if let Some(userinfo) = url_password(&req.url) {
            self.sensitive("url (usuario:contraseña)", userinfo, "password");
        } else {
            self.text("url", &req.url, "token");
        }
        self.pairs("parámetro", &req.query);
        self.pairs("header", &req.headers);
        self.auth("", &req.auth);
        match &req.body {
            Body::Json { json } => self.text("cuerpo", json, "token"),
            Body::Text { text } | Body::Xml { xml: text } => self.text("cuerpo", text, "token"),
            Body::Form { fields } => self.pairs("campo del formulario", fields),
            Body::Graphql {
                query, variables, ..
            } => {
                self.text("cuerpo GraphQL", query, "token");
                self.text("variables GraphQL", variables, "token");
            }
            _ => {}
        }
        for (phase, script) in [
            ("pre", &req.scripts.pre),
            ("post", &req.scripts.post),
            ("test", &req.scripts.test),
        ] {
            self.text(&format!("script {phase}"), script, "token");
        }
    }
}

/// Contraseña de `https://usuario:contraseña@host`.
fn url_password(url: &str) -> Option<&str> {
    let rest = url.split_once("://")?.1;
    let authority = rest.split(['/', '?', '#']).next()?;
    let userinfo = authority.rsplit_once('@')?.0;
    userinfo
        .split_once(':')
        .map(|(_, p)| p)
        .filter(|p| !p.is_empty())
}

/// Nombre de variable a partir de un nombre de campo: `X-Api-Key` → `x_api_key`.
fn suggest(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('_') && !out.is_empty() {
            out.push('_');
        }
    }
    let out = out.trim_end_matches('_').to_owned();
    if out.is_empty() { "secret".into() } else { out }
}

/// Revisa una solicitud (antes de guardarla).
#[must_use]
pub fn scan_request(key: &str, req: &Request) -> Vec<Finding> {
    let file = format!("requests/{key}.toml");
    let mut s = Scanner {
        file: &file,
        out: Vec::new(),
    };
    s.request(req);
    s.out
}

/// Revisa todo lo que la colección guarda en archivos versionados.
#[must_use]
pub fn scan_collection(c: &Collection) -> Vec<Finding> {
    let mut out = Vec::new();
    let public =
        |vars: &indexmap::IndexMap<String, String>, secrets: &[String]| -> Vec<(String, String)> {
            vars.iter()
                .filter(|(k, _)| !secrets.contains(k))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()
        };
    {
        let mut s = Scanner {
            file: "harpoon.toml",
            out: Vec::new(),
        };
        let vars = public(&c.meta.vars, &c.meta.secrets);
        s.pairs("variable", vars.iter().map(|(k, v)| (k, v)));
        s.auth("", &c.meta.auth);
        for (phase, script) in [
            ("pre", &c.meta.scripts.pre),
            ("post", &c.meta.scripts.post),
            ("test", &c.meta.scripts.test),
        ] {
            s.text(&format!("script {phase}"), script, "token");
        }
        out.extend(s.out);
    }
    for (folder, config) in &c.folder_configs {
        let file = format!("requests/{folder}/folder.toml");
        let mut s = Scanner {
            file: &file,
            out: Vec::new(),
        };
        let vars = public(&config.vars, &config.secrets);
        s.pairs("variable", vars.iter().map(|(k, v)| (k, v)));
        s.pairs("header", &config.headers);
        s.auth("", &config.auth);
        for (phase, script) in [
            ("pre", &config.scripts.pre),
            ("post", &config.scripts.post),
            ("test", &config.scripts.test),
        ] {
            s.text(&format!("script {phase}"), script, "token");
        }
        out.extend(s.out);
    }
    for (name, env) in &c.environments {
        let file = format!("environments/{name}.toml");
        let mut s = Scanner {
            file: &file,
            out: Vec::new(),
        };
        let vars = public(&env.vars, &env.secrets);
        s.pairs("variable", vars.iter().map(|(k, v)| (k, v)));
        out.extend(s.out);
    }
    for (key, req) in &c.requests {
        out.extend(scan_request(key, req));
    }
    out
}

/// Avisos para mostrar antes de exportar o compartir la colección.
#[must_use]
pub fn export_warnings(c: &Collection) -> Vec<String> {
    let findings = scan_collection(c);
    if findings.is_empty() {
        return Vec::new();
    }
    let mut lines: Vec<String> = findings
        .iter()
        .take(5)
        .map(|f| format!("{} · {} ({})", f.file, f.location, f.kind))
        .collect();
    if findings.len() > 5 {
        lines.push(format!("y {} más", findings.len() - 5));
    }
    vec![format!(
        "Posibles secretos escritos tal cual; se exportarán: {}. Conviértelos en variables secretas antes de compartir.",
        lines.join("; ")
    )]
}

#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::IndexMap;

    #[test]
    fn reconoce_formatos_conocidos() {
        for (text, kind) in [
            (
                "Bearer eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U",
                "token JWT",
            ),
            ("AKIAIOSFODNN7EXAMPLE", "clave de AWS"),
            ("token ghp_1234567890abcdefghijklmnopqrstuvwxyzAB", "GitHub"),
            ("sk-ant-api03-abcdefghijklmnopqrstuvwxyz", "Anthropic"),
            ("key=AIzaSyA1234567890abcdefghijklmnopqrstuv", "Google"),
            ("-----BEGIN RSA PRIVATE KEY-----\nMIIE", "clave privada"),
        ] {
            let (found, _) = known_secret_in(text).unwrap_or_else(|| panic!("no detecta {text}"));
            assert!(
                found.contains(kind)
                    || kind.contains(found)
                    || found.contains(kind.split(' ').next_back().unwrap()),
                "{text} → {found}"
            );
        }
        for clean in [
            "https://api.ejemplo.com/v1/users?page=2",
            "{{token}}",
            "application/json",
            "sk-short",
            "AKIA-not-a-key",
        ] {
            assert!(
                known_secret_in(clean).is_none(),
                "falso positivo en {clean}"
            );
        }
    }

    #[test]
    fn campos_sensibles_con_literal() {
        let mut req = Request::new(
            "x",
            crate::Method::Get,
            "https://user:hunter2pass@api.x.com/a",
        );
        req.headers = IndexMap::from([
            ("Authorization".into(), "Bearer abcdef123456".into()),
            ("X-Api-Key".into(), "{{api_key}}".into()),
            ("Accept".into(), "application/json".into()),
        ]);
        req.query = IndexMap::from([
            ("access_token".into(), "zzzzzzzz".into()),
            ("page".into(), "2".into()),
        ]);
        req.auth = Auth::Bearer {
            token: "Bearer {{token}}".into(),
        };
        let found = scan_request("users/list", &req);
        let where_: Vec<&str> = found.iter().map(|f| f.location.as_str()).collect();
        assert_eq!(
            where_,
            vec![
                "url (usuario:contraseña)",
                "parámetro access_token",
                "header Authorization"
            ],
            "{found:#?}"
        );
        assert!(found.iter().all(|f| f.file == "requests/users/list.toml" && !f.preview.contains("hunter2pass")));
        assert_eq!(found[2].suggestion, "authorization");
    }

    #[test]
    fn nombres_sensibles() {
        for name in [
            "Authorization",
            "X-API-Key",
            "client_secret",
            "refreshToken",
            "password",
            "Cookie",
            "aws_access_key_id",
        ] {
            assert!(is_sensitive_name(name), "{name}");
        }
        for name in ["Accept", "page", "user", "content-type", "author"] {
            assert!(!is_sensitive_name(name), "{name}");
        }
    }
}
