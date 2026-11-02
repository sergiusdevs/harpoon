//! `harpoon-http`: convierte una [`Request`] del modelo en una petición HTTP
//! real, la envía y devuelve una [`Response`] con la que trabajar
//! (mostrar, guardar, evaluar aserciones).
//!
//! TLS va con `rustls` y raíces `webpki` embebidas: no depende del sistema.

use std::time::{Duration, Instant};

use harpoon_core::{
    Auth, Body, Collection, Method, Request, Scope,
    model::{ApiKeyLocation, Expect, Settings},
};
use indexmap::IndexMap;
use serde::Serialize;

pub mod assert;

pub use assert::{Assertion, Outcome};

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
    #[error("fallo de red: {}", describe(.0))]
    Network(reqwest::Error),
    #[error("tiempo agotado tras {0:?}")]
    Timeout(Duration),
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
pub fn prepare(collection: &Collection, request: &Request, scope: &Scope) -> Result<Prepared> {
    let url = scope.interpolate(&request.url)?;
    let mut headers = IndexMap::new();
    for (k, v) in &request.headers {
        headers.insert(scope.interpolate(k)?, scope.interpolate(v)?);
    }
    let mut query = IndexMap::new();
    for (k, v) in &request.query {
        query.insert(scope.interpolate(k)?, scope.interpolate(v)?);
    }

    let auth = match &request.auth {
        Auth::Inherit => &collection.meta.auth,
        other => other,
    };
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
    }

    let body = match &request.body {
        Body::None => None,
        Body::Json { json } => {
            headers
                .entry("Content-Type".into())
                .or_insert_with(|| "application/json".into());
            Some(scope.interpolate(json)?)
        }
        Body::Text { text } => Some(scope.interpolate(text)?),
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
    })
}

/// Cliente reutilizable para toda una ejecución (cookies y conexiones compartidas).
pub struct Client {
    inner: reqwest::Client,
    timeout: Duration,
}

impl Client {
    pub fn new(settings: &Settings) -> Result<Self> {
        let timeout = Duration::from_millis(settings.timeout_ms);
        let mut builder = reqwest::Client::builder()
            .user_agent(concat!("harpoon/", env!("CARGO_PKG_VERSION")))
            .timeout(timeout)
            .cookie_store(true)
            .danger_accept_invalid_certs(settings.insecure)
            .redirect(if settings.follow_redirects {
                reqwest::redirect::Policy::limited(10)
            } else {
                reqwest::redirect::Policy::none()
            });
        if let Some(proxy) = &settings.proxy {
            builder = builder.proxy(reqwest::Proxy::all(proxy).map_err(Error::Client)?);
        }
        Ok(Self {
            inner: builder.build().map_err(Error::Client)?,
            timeout,
        })
    }

    pub async fn send(&self, prepared: &Prepared) -> Result<Response> {
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
        if let Some(body) = &prepared.body {
            req = req.body(body.clone());
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
        let headers = res
            .headers()
            .iter()
            .map(|(k, v)| {
                (
                    k.to_string(),
                    String::from_utf8_lossy(v.as_bytes()).into_owned(),
                )
            })
            .collect();
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

fn base64(input: &[u8]) -> String {
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

fn urlencode(s: &str) -> String {
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
