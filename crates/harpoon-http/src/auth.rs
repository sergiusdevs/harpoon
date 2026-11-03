#![allow(
    clippy::large_enum_variant,
    clippy::doc_markdown,
    clippy::must_use_candidate,
    clippy::format_push_string
)]

//! Autenticaciones que necesitan red o un reto previo: OAuth 2.0 y Digest.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

use harpoon_core::model::{OAuth2ClientAuth, OAuth2Grant};
use indexmap::IndexMap;
use md5::{Digest as _, Md5};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

use crate::{Error, Result, base64, urlencode};

/// Auth que no se puede resolver en `prepare` porque necesita red o un reto.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pending {
    Digest { username: String, password: String },
    OAuth2(OAuth2Config),
}

/// Configuración OAuth2 ya interpolada.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OAuth2Config {
    pub grant: OAuth2Grant,
    pub token_url: String,
    pub client_id: String,
    pub client_secret: String,
    pub scope: String,
    pub username: String,
    pub password: String,
    pub refresh_token: String,
    pub client_auth: OAuth2ClientAuth,
    pub extra: IndexMap<String, String>,
    pub header_prefix: String,
}

impl OAuth2Config {
    /// Clave de caché: todo lo que identifica el token, sin el secreto en claro.
    fn cache_key(&self) -> String {
        let raw = format!(
            "{:?}|{}|{}|{}|{}|{}",
            self.grant,
            self.token_url,
            self.client_id,
            self.scope,
            self.username,
            self.refresh_token
        );
        hex::encode(Sha256::digest(raw.as_bytes()))[..32].to_owned()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedToken {
    access_token: String,
    /// Unix seconds; 0 = sin caducidad conocida (se reutiliza hasta un 401).
    expires_at: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    refresh_token: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct TokenFile {
    #[serde(default)]
    tokens: HashMap<String, CachedToken>,
}

/// Caché de tokens OAuth2: en memoria y, si hay ruta, en `.harpoon/tokens.toml`.
#[derive(Debug, Default)]
pub struct TokenCache {
    path: Option<PathBuf>,
    tokens: Mutex<HashMap<String, CachedToken>>,
}

impl TokenCache {
    #[must_use]
    pub fn new(path: Option<PathBuf>) -> Self {
        let tokens = path
            .as_ref()
            .filter(|p| p.is_file())
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| toml::from_str::<TokenFile>(&s).ok())
            .map(|f| f.tokens)
            .unwrap_or_default();
        Self {
            path,
            tokens: Mutex::new(tokens),
        }
    }

    fn get(&self, key: &str) -> Option<CachedToken> {
        let tokens = self.tokens.lock().ok()?;
        let t = tokens.get(key)?;
        // Margen de 30 s para no usar un token a punto de caducar.
        if t.expires_at != 0 && now() + 30 >= t.expires_at {
            return None;
        }
        Some(t.clone())
    }

    fn put(&self, key: String, token: CachedToken) {
        if let Ok(mut tokens) = self.tokens.lock() {
            tokens.insert(key, token);
            if let Some(path) = &self.path {
                let file = TokenFile {
                    tokens: tokens.clone(),
                };
                if let Ok(text) = toml::to_string(&file) {
                    if let Some(parent) = path.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    let _ = std::fs::write(path, text);
                }
            }
        }
    }

    /// Olvida un token (p. ej. tras un 401 con token cacheado).
    pub fn invalidate(&self, key: &str) {
        if let Ok(mut tokens) = self.tokens.lock() {
            tokens.remove(key);
        }
    }
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    expires_in: Option<u64>,
    #[serde(default)]
    refresh_token: Option<String>,
}

/// Obtiene (o reutiliza) un token y devuelve el valor de `Authorization`.
pub async fn oauth2_authorization(
    http: &reqwest::Client,
    cache: &TokenCache,
    cfg: &OAuth2Config,
    force_refresh: bool,
) -> Result<String> {
    let key = cfg.cache_key();
    if !force_refresh && let Some(t) = cache.get(&key) {
        return Ok(format!("{} {}", cfg.header_prefix, t.access_token));
    }

    let mut form: Vec<(String, String)> = Vec::new();
    match cfg.grant {
        OAuth2Grant::ClientCredentials => {
            form.push(("grant_type".into(), "client_credentials".into()));
        }
        OAuth2Grant::Password => {
            form.push(("grant_type".into(), "password".into()));
            form.push(("username".into(), cfg.username.clone()));
            form.push(("password".into(), cfg.password.clone()));
        }
        OAuth2Grant::RefreshToken => {
            form.push(("grant_type".into(), "refresh_token".into()));
            // Si ya refrescamos antes, usamos el refresh_token más reciente.
            let rt = cache
                .tokens
                .lock()
                .ok()
                .and_then(|t| t.get(&key).and_then(|c| c.refresh_token.clone()))
                .unwrap_or_else(|| cfg.refresh_token.clone());
            form.push(("refresh_token".into(), rt));
        }
    }
    if !cfg.scope.is_empty() {
        form.push(("scope".into(), cfg.scope.clone()));
    }
    for (k, v) in &cfg.extra {
        form.push((k.clone(), v.clone()));
    }

    let mut req = http
        .post(&cfg.token_url)
        .header("Accept", "application/json")
        .header("Content-Type", "application/x-www-form-urlencoded");
    match cfg.client_auth {
        OAuth2ClientAuth::Basic => {
            let raw = format!("{}:{}", cfg.client_id, cfg.client_secret);
            req = req.header("Authorization", format!("Basic {}", base64(raw.as_bytes())));
        }
        OAuth2ClientAuth::Body => {
            form.push(("client_id".into(), cfg.client_id.clone()));
            if !cfg.client_secret.is_empty() {
                form.push(("client_secret".into(), cfg.client_secret.clone()));
            }
        }
    }
    let body: Vec<String> = form
        .iter()
        .map(|(k, v)| format!("{}={}", urlencode(k), urlencode(v)))
        .collect();
    let res = req
        .body(body.join("&"))
        .send()
        .await
        .map_err(Error::Network)?;
    let status = res.status();
    let text = res.text().await.map_err(Error::Network)?;
    if !status.is_success() {
        return Err(Error::OAuth2(format!(
            "el servidor de tokens respondió {status}: {}",
            text.chars().take(300).collect::<String>()
        )));
    }
    let token: TokenResponse = serde_json::from_str(&text)
        .map_err(|e| Error::OAuth2(format!("respuesta de token no válida ({e}): {text}")))?;
    let expires_at = token.expires_in.map_or(0, |s| now() + s);
    cache.put(
        key,
        CachedToken {
            access_token: token.access_token.clone(),
            expires_at,
            refresh_token: token.refresh_token,
        },
    );
    Ok(format!("{} {}", cfg.header_prefix, token.access_token))
}

/// Clave de caché expuesta para invalidar tras un 401.
#[must_use]
pub fn oauth2_cache_key(cfg: &OAuth2Config) -> String {
    cfg.cache_key()
}

// ---------------------------------------------------------------- Digest

#[derive(Debug, Default)]
struct Challenge {
    realm: String,
    nonce: String,
    qop: Option<String>,
    opaque: Option<String>,
    algorithm: String,
}

fn parse_challenge(header: &str) -> Option<Challenge> {
    let rest = header.strip_prefix("Digest ")?;
    let mut c = Challenge {
        algorithm: "MD5".into(),
        ..Challenge::default()
    };
    // key=value pares separados por comas; los valores pueden ir entre comillas.
    let mut i = 0;
    let bytes = rest.as_bytes();
    while i < bytes.len() {
        while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b',') {
            i += 1;
        }
        let ks = i;
        while i < bytes.len() && bytes[i] != b'=' {
            i += 1;
        }
        let key = rest[ks..i].trim().to_ascii_lowercase();
        i += 1; // '='
        let value = if i < bytes.len() && bytes[i] == b'"' {
            i += 1;
            let vs = i;
            while i < bytes.len() && bytes[i] != b'"' {
                i += 1;
            }
            let v = &rest[vs..i];
            i += 1;
            v.to_owned()
        } else {
            let vs = i;
            while i < bytes.len() && bytes[i] != b',' {
                i += 1;
            }
            rest[vs..i].trim().to_owned()
        };
        match key.as_str() {
            "realm" => c.realm = value,
            "nonce" => c.nonce = value,
            "qop" => c.qop = Some(value),
            "opaque" => c.opaque = Some(value),
            "algorithm" => c.algorithm = value,
            _ => {}
        }
    }
    if c.nonce.is_empty() {
        return None;
    }
    Some(c)
}

fn hash(alg: &str, data: &str) -> String {
    if alg.to_ascii_uppercase().starts_with("SHA-256") {
        hex::encode(Sha256::digest(data.as_bytes()))
    } else {
        hex::encode(Md5::digest(data.as_bytes()))
    }
}

/// Construye la cabecera `Authorization: Digest ...` para responder a un reto.
pub fn digest_authorization(
    www_authenticate: &str,
    method: &str,
    uri: &str,
    username: &str,
    password: &str,
) -> Option<String> {
    let c = parse_challenge(www_authenticate)?;
    let alg = c.algorithm.clone();
    let cnonce = uuid_like();
    let nc = "00000001";
    let mut ha1 = hash(&alg, &format!("{username}:{}:{password}", c.realm));
    if alg.to_ascii_uppercase().ends_with("-SESS") {
        ha1 = hash(&alg, &format!("{ha1}:{}:{cnonce}", c.nonce));
    }
    let ha2 = hash(&alg, &format!("{method}:{uri}"));
    let qop = c
        .qop
        .as_deref()
        .and_then(|q| q.split(',').map(str::trim).find(|q| *q == "auth"));
    let response = if qop.is_some() {
        hash(&alg, &format!("{ha1}:{}:{nc}:{cnonce}:auth:{ha2}", c.nonce))
    } else {
        hash(&alg, &format!("{ha1}:{}:{ha2}", c.nonce))
    };
    let mut out = format!(
        "Digest username=\"{username}\", realm=\"{}\", nonce=\"{}\", uri=\"{uri}\", response=\"{response}\", algorithm={alg}",
        c.realm, c.nonce
    );
    if qop.is_some() {
        out.push_str(&format!(", qop=auth, nc={nc}, cnonce=\"{cnonce}\""));
    }
    if let Some(o) = c.opaque {
        out.push_str(&format!(", opaque=\"{o}\""));
    }
    Some(out)
}

fn uuid_like() -> String {
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    hex::encode(&Sha256::digest(t.to_le_bytes())[..8])
}

/// Ruta + query de una URL absoluta (lo que va en `uri=` del Digest).
#[must_use]
pub fn request_uri(url: &str) -> String {
    let after_scheme = url.split_once("://").map_or(url, |(_, r)| r);
    match after_scheme.find('/') {
        Some(i) => after_scheme[i..].to_owned(),
        None => "/".into(),
    }
}

#[must_use]
pub fn resolve_path(root: &Path, p: &str) -> PathBuf {
    let path = Path::new(p);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}

#[cfg(test)]
mod tests;
