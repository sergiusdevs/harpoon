//! `harpoon-http`: convierte una [`Request`] del modelo en una petición HTTP
//! real, la envía y devuelve una [`Response`] con la que trabajar
//! (mostrar, guardar, evaluar aserciones).
//!
//! TLS va con `rustls` y raíces `webpki` embebidas: no depende del sistema.

use std::time::{Duration, Instant};

use harpoon_core::{
    Auth, Body, Collection, Method, Request, Scope,
    model::{ApiKeyLocation, Expect, MultipartPart, Settings},
};
use indexmap::IndexMap;
use serde::Serialize;

pub mod assert;
pub mod auth;
pub mod cookies;

pub use assert::{Assertion, Outcome};
pub use auth::TokenCache;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Core(#[from] harpoon_core::Error),
    #[error("no se pudo crear el cliente HTTP: {0}")]
    Client(reqwest::Error),
    #[error("URL inválida `{url}`: {source}")]
    Url {
        url: String,
        #[source]
        source: reqwest::Error,
    },
    #[error("cabecera inválida `{0}`")]
    Header(String),
    #[error("no se pudo leer el archivo {path}: {source}")]
    UploadFile {
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("formulario multipart inválido: {0}")]
    Multipart(String),
    #[error("fallo de red: {}", describe(.0))]
    Network(reqwest::Error),
    #[error("tiempo agotado tras {0:?}")]
    Timeout(Duration),
    #[error("cookies: {0}")]
    Cookies(String),
    #[error("OAuth2: {0}")]
    OAuth2(String),
    #[error("no se pudo leer el certificado {path}: {reason}")]
    Certificate { path: String, reason: String },
}

pub type Result<T> = std::result::Result<T, Error>;

/// Petición ya resuelta: variables interpoladas y auth aplicada.
/// Es lo que se muestra con `--verbose` y lo que se guarda en el historial.
#[derive(Debug, Clone, Serialize)]
pub struct Prepared {
    pub method: Method,
    pub url: String,
    pub headers: IndexMap<String, String>,
    pub query: IndexMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    /// Ruta del cuerpo binario, resuelta respecto a la colección.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binary_path: Option<String>,
    /// Partes multipart resueltas; los archivos se leen al enviar.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub multipart: Option<Vec<MultipartPart>>,
    /// Auth que se resuelve al enviar (`OAuth2`, Digest).
    #[serde(skip)]
    pub pending_auth: Option<auth::Pending>,
}

/// Respuesta recibida.
#[derive(Debug, Clone, Serialize)]
pub struct Response {
    pub status: u16,
    pub headers: IndexMap<String, String>,
    #[serde(skip)]
    pub body: Vec<u8>,
    pub elapsed_ms: u128,
    pub size: usize,
}

impl Response {
    #[must_use]
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }

    #[must_use]
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    /// Heurística: `content-type` no textual o bytes no UTF-8 con NUL.
    #[must_use]
    pub fn is_binary(&self) -> bool {
        let ct = self
            .header("content-type")
            .unwrap_or("")
            .to_ascii_lowercase();
        let textual = ct.starts_with("text/")
            || ct.contains("json")
            || ct.contains("xml")
            || ct.contains("javascript")
            || ct.contains("x-www-form-urlencoded")
            || ct.contains("yaml")
            || ct.contains("csv")
            || ct.contains("html");
        if textual {
            return false;
        }
        if ct.starts_with("image/")
            || ct.starts_with("audio/")
            || ct.starts_with("video/")
            || ct.contains("octet-stream")
            || ct.contains("pdf")
            || ct.contains("zip")
        {
            return true;
        }
        std::str::from_utf8(&self.body).is_err() || self.body.contains(&0)
    }

    /// Cuerpo en base64 (para transportar binarios al frontend).
    #[must_use]
    pub fn body_base64(&self) -> String {
        base64(&self.body)
    }

    #[must_use]
    pub fn is_json(&self) -> bool {
        self.header("content-type")
            .is_some_and(|ct| ct.contains("json"))
    }

    /// Cuerpo como JSON, si lo es.
    #[must_use]
    pub fn json(&self) -> Option<serde_json::Value> {
        serde_json::from_slice(&self.body).ok()
    }
}

/// Resuelve variables y auth de una solicitud dentro de su colección.
#[allow(clippy::too_many_lines)]
pub fn prepare(collection: &Collection, request: &Request, scope: &Scope) -> Result<Prepared> {
    let url = scope.interpolate(&request.url)?;
    let mut headers = IndexMap::new();
    for (k, v) in &request.headers {
        headers.insert(scope.interpolate(k)?, scope.interpolate(v)?);
    }
    let mut query = IndexMap::new();
    for (k, v) in &request.query {
        if !request.disabled_query.contains(k) {
            query.insert(scope.interpolate(k)?, scope.interpolate(v)?);
        }
    }

    let auth = match &request.auth {
        Auth::Inherit => &collection.meta.auth,
        other => other,
    };
    let mut pending_auth = None;
    match auth {
        Auth::Inherit | Auth::None => {}
        Auth::Basic { username, password } => {
            let raw = format!(
                "{}:{}",
                scope.interpolate(username)?,
                scope.interpolate(password)?
            );
            headers.insert(
                "Authorization".into(),
                format!("Basic {}", base64(raw.as_bytes())),
            );
        }
        Auth::Bearer { token } => {
            headers.insert(
                "Authorization".into(),
                format!("Bearer {}", scope.interpolate(token)?),
            );
        }
        Auth::ApiKey { key, value, r#in } => {
            let (k, v) = (scope.interpolate(key)?, scope.interpolate(value)?);
            match r#in {
                ApiKeyLocation::Header => headers.insert(k, v),
                ApiKeyLocation::Query => query.insert(k, v),
            };
        }
        Auth::Digest { username, password } => {
            pending_auth = Some(auth::Pending::Digest {
                username: scope.interpolate(username)?,
                password: scope.interpolate(password)?,
            });
        }
        Auth::OAuth2 {
            grant,
            token_url,
            client_id,
            client_secret,
            scope: oscope,
            username,
            password,
            refresh_token,
            client_auth,
            extra,
            header_prefix,
        } => {
            let mut extra_resolved = IndexMap::new();
            for (k, v) in extra {
                extra_resolved.insert(k.clone(), scope.interpolate(v)?);
            }
            pending_auth = Some(auth::Pending::OAuth2(auth::OAuth2Config {
                grant: *grant,
                token_url: scope.interpolate(token_url)?,
                client_id: scope.interpolate(client_id)?,
                client_secret: scope.interpolate(client_secret)?,
                scope: scope.interpolate(oscope)?,
                username: scope.interpolate(username)?,
                password: scope.interpolate(password)?,
                refresh_token: scope.interpolate(refresh_token)?,
                client_auth: *client_auth,
                extra: extra_resolved,
                header_prefix: header_prefix.clone(),
            }));
        }
    }

    let mut multipart = None;
    let mut binary_path = None;
    let body = match &request.body {
        Body::None => None,
        Body::Json { json } => {
            headers
                .entry("Content-Type".into())
                .or_insert_with(|| "application/json".into());
            Some(scope.interpolate(json)?)
        }
        Body::Text { text } => Some(scope.interpolate(text)?),
        Body::Xml { xml } => {
            if !headers
                .keys()
                .any(|name| name.eq_ignore_ascii_case("content-type"))
            {
                headers.insert("Content-Type".into(), "application/xml".into());
            }
            Some(scope.interpolate(xml)?)
        }
        Body::Binary { path } => {
            let path = scope.interpolate(path)?;
            if path.trim().is_empty() {
                return Err(Error::Multipart("la ruta del archivo está vacía".into()));
            }
            binary_path = Some(collection.root.join(path).to_string_lossy().into_owned());
            if !headers
                .keys()
                .any(|name| name.eq_ignore_ascii_case("content-type"))
            {
                headers.insert("Content-Type".into(), "application/octet-stream".into());
            }
            None
        }
        Body::Multipart { parts } => {
            if headers
                .keys()
                .any(|name| name.eq_ignore_ascii_case("content-type"))
            {
                return Err(Error::Multipart(
                    "elimina Content-Type para generar el límite multipart automáticamente".into(),
                ));
            }
            let mut resolved = Vec::with_capacity(parts.len());
            for part in parts {
                resolved.push(match part {
                    MultipartPart::Text { name, value } => MultipartPart::Text {
                        name: scope.interpolate(name)?,
                        value: scope.interpolate(value)?,
                    },
                    MultipartPart::File {
                        name,
                        path,
                        content_type,
                    } => {
                        let path = scope.interpolate(path)?;
                        if path.trim().is_empty() {
                            return Err(Error::Multipart("la ruta del archivo está vacía".into()));
                        }
                        let path = collection.root.join(path).to_string_lossy().into_owned();
                        MultipartPart::File {
                            name: scope.interpolate(name)?,
                            path,
                            content_type: content_type
                                .as_deref()
                                .map(|value| scope.interpolate(value))
                                .transpose()?,
                        }
                    }
                });
            }
            if resolved.iter().any(|part| match part {
                MultipartPart::Text { name, .. } | MultipartPart::File { name, .. } => {
                    name.trim().is_empty()
                }
            }) {
                return Err(Error::Multipart("el nombre de un campo está vacío".into()));
            }
            multipart = Some(resolved);
            None
        }
        Body::Form { fields } => {
            headers
                .entry("Content-Type".into())
                .or_insert_with(|| "application/x-www-form-urlencoded".into());
            let pairs: Vec<String> = fields
                .iter()
                .map(|(k, v)| {
                    Ok(format!(
                        "{}={}",
                        urlencode(k),
                        urlencode(&scope.interpolate(v)?)
                    ))
                })
                .collect::<Result<_>>()?;
            Some(pairs.join("&"))
        }
    };

    Ok(Prepared {
        method: request.method,
        url,
        headers,
        query,
        body,
        binary_path,
        multipart,
        pending_auth,
    })
}

/// Cliente reutilizable para toda una ejecución (cookies y conexiones compartidas).
pub struct Client {
    inner: reqwest::Client,
    timeout: Duration,
    tokens: TokenCache,
}

impl Client {
    /// Cliente a partir de los ajustes, con rutas de certificados relativas a `root`
    /// y caché de tokens en `root/.harpoon/tokens.toml`.
    pub fn for_collection(collection: &Collection) -> Result<Self> {
        Self::build(&collection.meta.settings, Some(&collection.root))
    }

    /// Cliente sin colección (rutas de certificados relativas al directorio actual,
    /// tokens solo en memoria).
    pub fn new(settings: &Settings) -> Result<Self> {
        Self::build(settings, None)
    }

    fn build(settings: &Settings, root: Option<&std::path::Path>) -> Result<Self> {
        let timeout = Duration::from_millis(settings.timeout_ms);
        let base = root.unwrap_or_else(|| std::path::Path::new("."));
        let jar = if let Some(root) = root {
            cookies::PersistentCookies::for_root(root)?
        } else {
            std::sync::Arc::new(cookies::PersistentCookies::default())
        };
        let mut builder = reqwest::Client::builder()
            .user_agent(concat!("harpoon/", env!("CARGO_PKG_VERSION")))
            .timeout(timeout)
            .cookie_provider(jar)
            .danger_accept_invalid_certs(settings.insecure)
            .redirect(if settings.follow_redirects {
                reqwest::redirect::Policy::limited(10)
            } else {
                reqwest::redirect::Policy::none()
            });
        if let Some(proxy) = &settings.proxy {
            builder = builder.proxy(reqwest::Proxy::all(proxy).map_err(Error::Client)?);
        }
        if let Some(ca) = &settings.ca_cert {
            let pem = read_pem(base, ca)?;
            for cert in
                reqwest::Certificate::from_pem_bundle(&pem).map_err(|e| Error::Certificate {
                    path: ca.clone(),
                    reason: e.to_string(),
                })?
            {
                builder = builder.add_root_certificate(cert);
            }
        }
        if let Some(cert) = &settings.client_cert {
            let mut pem = read_pem(base, cert)?;
            if let Some(key) = &settings.client_key {
                pem.push(b'\n');
                pem.extend(read_pem(base, key)?);
            }
            let identity = reqwest::Identity::from_pem(&pem).map_err(|e| Error::Certificate {
                path: cert.clone(),
                reason: e.to_string(),
            })?;
            builder = builder.identity(identity);
        }
        let tokens = TokenCache::new(root.map(|r| r.join(".harpoon").join("tokens.toml")));
        Ok(Self {
            inner: builder.build().map_err(Error::Client)?,
            timeout,
            tokens,
        })
    }

    pub async fn send(&self, prepared: &Prepared) -> Result<Response> {
        match &prepared.pending_auth {
            None => self.send_raw(prepared, None).await,
            Some(auth::Pending::OAuth2(cfg)) => {
                let header =
                    auth::oauth2_authorization(&self.inner, &self.tokens, cfg, false).await?;
                let res = self
                    .send_raw(prepared, Some(("Authorization", header)))
                    .await?;
                if res.status != 401 {
                    return Ok(res);
                }
                // Token cacheado pero rechazado: pedimos uno nuevo y reintentamos una vez.
                self.tokens.invalidate(&auth::oauth2_cache_key(cfg));
                let header =
                    auth::oauth2_authorization(&self.inner, &self.tokens, cfg, true).await?;
                self.send_raw(prepared, Some(("Authorization", header)))
                    .await
            }
            Some(auth::Pending::Digest { username, password }) => {
                let first = self.send_raw(prepared, None).await?;
                if first.status != 401 {
                    return Ok(first);
                }
                let Some(challenge) = first.header("www-authenticate") else {
                    return Ok(first);
                };
                let Some(header) = auth::digest_authorization(
                    challenge,
                    &prepared.method.to_string(),
                    &auth::request_uri(&full_url(prepared)),
                    username,
                    password,
                ) else {
                    return Ok(first);
                };
                let mut second = self
                    .send_raw(prepared, Some(("Authorization", header)))
                    .await?;
                second.elapsed_ms += first.elapsed_ms;
                Ok(second)
            }
        }
    }

    async fn send_raw(
        &self,
        prepared: &Prepared,
        extra_header: Option<(&str, String)>,
    ) -> Result<Response> {
        let method = reqwest::Method::from_bytes(prepared.method.to_string().as_bytes())
            .map_err(|_| Error::Header(format!("método {}", prepared.method)))?;
        let mut req = self
            .inner
            .request(method, &prepared.url)
            .query(&prepared.query.iter().collect::<Vec<_>>());
        for (k, v) in &prepared.headers {
            req = req.header(
                reqwest::header::HeaderName::from_bytes(k.as_bytes())
                    .map_err(|_| Error::Header(k.clone()))?,
                reqwest::header::HeaderValue::from_str(v).map_err(|_| Error::Header(k.clone()))?,
            );
        }
        if let Some((k, v)) = extra_header {
            req = req.header(k, v);
        }
        if let Some(body) = &prepared.body {
            req = req.body(body.clone());
        }
        if let Some(path) = &prepared.binary_path {
            req = with_binary_body(req, path, &prepared.headers).await?;
        }
        if let Some(parts) = &prepared.multipart {
            let mut form = reqwest::multipart::Form::new();
            for part in parts {
                match part {
                    MultipartPart::Text { name, value } => {
                        form = form.text(name.clone(), value.clone());
                    }
                    MultipartPart::File {
                        name,
                        path,
                        content_type,
                    } => {
                        let file_path = std::path::Path::new(path);
                        if file_path.file_name().is_none() {
                            return Err(Error::Multipart(format!(
                                "ruta de archivo inválida: {path}"
                            )));
                        }
                        let mut field =
                            reqwest::multipart::Part::file(file_path)
                                .await
                                .map_err(|source| Error::UploadFile {
                                    path: file_path.to_path_buf(),
                                    source,
                                })?;
                        if let Some(mime) = content_type.as_deref().filter(|s| !s.is_empty()) {
                            field = field
                                .mime_str(mime)
                                .map_err(|e| Error::Multipart(e.to_string()))?;
                        }
                        form = form.part(name.clone(), field);
                    }
                }
            }
            req = req.multipart(form);
        }

        let start = Instant::now();
        let res = req.send().await.map_err(|e| {
            if e.is_timeout() {
                Error::Timeout(self.timeout)
            } else {
                Error::Network(e)
            }
        })?;
        let status = res.status().as_u16();
        let mut headers: IndexMap<String, String> = IndexMap::new();
        for (k, v) in res.headers() {
            let name = k.to_string();
            let value = String::from_utf8_lossy(v.as_bytes()).into_owned();
            match headers.get_mut(&name) {
                // Varias `Set-Cookie` se separan con salto de línea; el resto con coma.
                Some(existing) if name == "set-cookie" => {
                    existing.push('\n');
                    existing.push_str(&value);
                }
                Some(existing) => {
                    existing.push_str(", ");
                    existing.push_str(&value);
                }
                None => {
                    headers.insert(name, value);
                }
            }
        }
        let body = res.bytes().await.map_err(Error::Network)?.to_vec();
        let elapsed_ms = start.elapsed().as_millis();
        Ok(Response {
            status,
            headers,
            size: body.len(),
            body,
            elapsed_ms,
        })
    }
}

async fn with_binary_body(
    mut request: reqwest::RequestBuilder,
    path: &str,
    headers: &IndexMap<String, String>,
) -> Result<reqwest::RequestBuilder> {
    let file_path = std::path::Path::new(path);
    let file = tokio::fs::File::open(file_path)
        .await
        .map_err(|source| Error::UploadFile {
            path: file_path.to_path_buf(),
            source,
        })?;
    if !headers
        .keys()
        .any(|name| name.eq_ignore_ascii_case("content-length"))
    {
        let size = file
            .metadata()
            .await
            .map_err(|source| Error::UploadFile {
                path: file_path.to_path_buf(),
                source,
            })?
            .len();
        request = request.header(reqwest::header::CONTENT_LENGTH, size);
    }
    Ok(request.body(reqwest::Body::from(file)))
}

fn read_pem(base: &std::path::Path, p: &str) -> Result<Vec<u8>> {
    let path = auth::resolve_path(base, p);
    std::fs::read(&path).map_err(|e| Error::Certificate {
        path: p.to_owned(),
        reason: e.to_string(),
    })
}

fn full_url(p: &Prepared) -> String {
    if p.query.is_empty() {
        return p.url.clone();
    }
    let qs: Vec<String> = p
        .query
        .iter()
        .map(|(k, v)| format!("{}={}", urlencode(k), urlencode(v)))
        .collect();
    let sep = if p.url.contains('?') { '&' } else { '?' };
    format!("{}{sep}{}", p.url, qs.join("&"))
}

/// Atajo: prepara y envía una solicitud, y evalúa sus aserciones.
pub async fn execute(
    client: &Client,
    collection: &Collection,
    request: &Request,
    scope: &Scope,
) -> Result<(Prepared, Response, Vec<Outcome>)> {
    let prepared = prepare(collection, request, scope)?;
    let response = client.send(&prepared).await?;
    let outcomes = assert::evaluate(&request.expect, &response);
    Ok((prepared, response, outcomes))
}

/// Deja a la vista cuáles de las aserciones existen para una solicitud.
#[must_use]
pub fn assertions(expect: &Expect) -> Vec<Assertion> {
    assert::list(expect)
}

/// Mensaje de red con la causa raíz (`connection refused`, DNS, TLS...).
fn describe(err: &reqwest::Error) -> String {
    let mut parts = vec![err.to_string()];
    let mut source = std::error::Error::source(err);
    while let Some(s) = source {
        parts.push(s.to_string());
        source = s.source();
    }
    parts.dedup();
    parts.join(": ")
}

#[must_use]
pub fn base64(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0u32, |acc, (i, &b)| acc | (u32::from(b) << (16 - 8 * i)));
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(TABLE[((n >> (18 - 6 * i)) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

pub(crate) fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            b' ' => out.push('+'),
            _ => {
                use std::fmt::Write as _;
                let _ = write!(out, "%{b:02X}");
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_estandar() {
        assert_eq!(base64(b"user:pass"), "dXNlcjpwYXNz");
        assert_eq!(base64(b"a"), "YQ==");
        assert_eq!(base64(b"ab"), "YWI=");
    }

    #[test]
    fn urlencode_basico() {
        assert_eq!(urlencode("a b&c=d/é"), "a+b%26c%3Dd%2F%C3%A9");
    }
}
