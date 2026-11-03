//! Tipos del modelo: lo que se guarda en disco y lo que manipulan las
//! demás capas. Todo es `serde` para que el formato en disco sea
//! literalmente este modelo.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Método HTTP.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "UPPERCASE")]
pub enum Method {
    #[default]
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
}

impl std::fmt::Display for Method {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Patch => "PATCH",
            Self::Delete => "DELETE",
            Self::Head => "HEAD",
            Self::Options => "OPTIONS",
        };
        f.write_str(s)
    }
}

/// Autenticación de una solicitud. `None` = heredar de la carpeta/colección.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(tag = "type", rename_all = "snake_case")]
#[allow(clippy::large_enum_variant)]
pub enum Auth {
    #[default]
    Inherit,
    None,
    Basic {
        username: String,
        password: String,
    },
    Bearer {
        token: String,
    },
    ApiKey {
        key: String,
        value: String,
        #[serde(default)]
        r#in: ApiKeyLocation,
    },
    /// HTTP Digest (RFC 7616): reto 401 → respuesta firmada. MD5 y SHA-256.
    Digest {
        username: String,
        password: String,
    },
    /// OAuth 2.0: se obtiene un token del `token_url` y se envía como Bearer.
    /// Los tokens se cachean en `.harpoon/tokens.toml` hasta que caducan.
    #[serde(rename = "oauth2")]
    OAuth2 {
        #[serde(default)]
        grant: OAuth2Grant,
        token_url: String,
        #[serde(default)]
        client_id: String,
        #[serde(default)]
        client_secret: String,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        scope: String,
        /// Solo para `grant = "password"`.
        #[serde(default, skip_serializing_if = "String::is_empty")]
        username: String,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        password: String,
        /// Solo para `grant = "refresh_token"`.
        #[serde(default, skip_serializing_if = "String::is_empty")]
        refresh_token: String,
        /// Cómo se envían las credenciales de cliente.
        #[serde(default)]
        client_auth: OAuth2ClientAuth,
        /// Parámetros extra del cuerpo (`audience`, `resource`...).
        #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
        extra: IndexMap<String, String>,
        /// Prefijo de la cabecera `Authorization`.
        #[serde(default = "default_bearer")]
        header_prefix: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ApiKeyLocation {
    #[default]
    Header,
    Query,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum OAuth2Grant {
    #[default]
    ClientCredentials,
    Password,
    RefreshToken,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum OAuth2ClientAuth {
    /// `Authorization: Basic base64(id:secret)` (lo más habitual).
    #[default]
    Basic,
    /// `client_id` y `client_secret` en el cuerpo del formulario.
    Body,
}

fn default_bearer() -> String {
    "Bearer".into()
}

/// Cuerpo de la solicitud.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Body {
    #[default]
    None,
    Json {
        json: String,
    },
    Graphql {
        query: String,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        schema: String,
        #[serde(default)]
        variables: String,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        operation_name: String,
    },
    Text {
        text: String,
    },
    Xml {
        xml: String,
    },
    Binary {
        path: String,
    },
    Form {
        fields: IndexMap<String, String>,
    },
    Multipart {
        #[serde(default)]
        parts: Vec<MultipartPart>,
    },
}

/// Campo de formulario multipart, con valor literal o ruta a un archivo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MultipartPart {
    Text {
        name: String,
        value: String,
    },
    File {
        name: String,
        path: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        content_type: Option<String>,
    },
}

/// Scripts asociados a una solicitud (fase 3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Scripts {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub pre: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub post: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub test: String,
}

impl Scripts {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.pre.is_empty() && self.post.is_empty() && self.test.is_empty()
    }
}

/// Protocolo de una solicitud. Los archivos antiguos se reconocen por su contenido.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RequestType {
    Http,
    Graphql,
    Websocket,
    Sse,
}

impl RequestType {
    #[must_use]
    pub fn is_stream(self) -> bool {
        matches!(self, Self::Websocket | Self::Sse)
    }
}

/// Una solicitud, tal y como se guarda en `requests/**/*.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Request {
    pub name: String,
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    pub request_type: Option<RequestType>,
    #[serde(default)]
    pub method: Method,
    pub url: String,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub headers: IndexMap<String, String>,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub header_descriptions: IndexMap<String, String>,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub query_descriptions: IndexMap<String, String>,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub query: IndexMap<String, String>,
    /// Nombres de parámetros conservados en la colección pero excluidos del envío.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub disabled_query: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub disabled_headers: Vec<String>,
    #[serde(default)]
    pub auth: Auth,
    #[serde(default)]
    pub body: Body,
    #[serde(default)]
    pub scripts: Scripts,
    #[serde(default, skip_serializing_if = "Expect::is_empty")]
    pub expect: Expect,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub docs: String,
}

impl Request {
    #[must_use]
    pub fn header_is_disabled(&self, name: &str) -> bool {
        self.disabled_headers
            .iter()
            .any(|key| key.eq_ignore_ascii_case(name))
    }
    #[must_use]
    pub fn request_type(&self) -> RequestType {
        if let Some(kind) = self.request_type {
            return kind;
        }
        let url = self.url.to_ascii_lowercase();
        if url.starts_with("ws://") || url.starts_with("wss://") {
            return RequestType::Websocket;
        }
        if self.headers.iter().any(|(key, value)| {
            !self.header_is_disabled(key)
                && key.eq_ignore_ascii_case("accept")
                && value.split(',').any(|part| {
                    part.split(';')
                        .next()
                        .unwrap_or("")
                        .trim()
                        .eq_ignore_ascii_case("text/event-stream")
                })
        }) {
            return RequestType::Sse;
        }
        if matches!(self.body, Body::Graphql { .. }) {
            RequestType::Graphql
        } else {
            RequestType::Http
        }
    }

    #[must_use]
    pub fn new(name: impl Into<String>, method: Method, url: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            request_type: None,
            method,
            url: url.into(),
            headers: IndexMap::new(),
            header_descriptions: IndexMap::new(),
            query_descriptions: IndexMap::new(),
            query: IndexMap::new(),
            disabled_query: Vec::new(),
            disabled_headers: Vec::new(),
            auth: Auth::Inherit,
            body: Body::None,
            scripts: Scripts::default(),
            expect: Expect::default(),
            docs: String::new(),
        }
    }
}

/// Un entorno: un conjunto de variables con nombre (`environments/<nombre>.toml`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Environment {
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub descriptions: IndexMap<String, String>,
    #[serde(default)]
    pub vars: IndexMap<String, String>,
    /// Nombres de variables cuyo valor NO se guarda en el repo: se leen de
    /// `environments/<nombre>.secrets.toml` (ignorado por Git) o de variables
    /// de entorno `HARPOON_<NOMBRE>`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub secrets: Vec<String>,
}

/// Metadatos de la colección (`harpoon.toml`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollectionMeta {
    pub name: String,
    /// Versión del formato en disco, para migraciones futuras.
    #[serde(default = "default_format_version")]
    pub format: u32,
    #[serde(default)]
    pub default_environment: Option<String>,
    #[serde(default)]
    pub vars: IndexMap<String, String>,
    #[serde(default)]
    pub auth: Auth,
    #[serde(default)]
    pub settings: Settings,
    /// Scripts a nivel de colección: se ejecutan antes de los de cada solicitud.
    #[serde(default, skip_serializing_if = "Scripts::is_empty")]
    pub scripts: Scripts,
}

/// Ajustes de conexión de la colección (`[settings]` en `harpoon.toml`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    /// Tiempo máximo por solicitud, en milisegundos.
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
    /// Seguir redirecciones (máximo 10).
    #[serde(default = "default_true")]
    pub follow_redirects: bool,
    /// Proxy explícito (`http://host:puerto`), que respeta `NO_PROXY` del entorno.
    /// Si no se indica, se usan `HTTP_PROXY`/`HTTPS_PROXY`/`NO_PROXY`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proxy: Option<String>,
    /// No verificar el certificado TLS del servidor (solo desarrollo).
    #[serde(default)]
    pub insecure: bool,
    /// CA adicional en PEM (ruta relativa a la colección) para servidores internos.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ca_cert: Option<String>,
    /// Certificado de cliente (PEM) para mTLS; si la clave va aparte, `client_key`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_cert: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_key: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            timeout_ms: default_timeout_ms(),
            follow_redirects: true,
            proxy: None,
            insecure: false,
            ca_cert: None,
            client_cert: None,
            client_key: None,
        }
    }
}

/// Aserciones declarativas sobre la respuesta (`[expect]` en una solicitud).
/// Son la forma sin código de validar; los scripts de test llegan en la fase 3.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Expect {
    /// Código de estado exacto.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
    /// Cabeceras que deben estar presentes con ese valor exacto.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub headers: IndexMap<String, String>,
    /// Fragmentos que deben aparecer en el cuerpo.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub body_contains: Vec<String>,
    /// Tiempo máximo de respuesta, en milisegundos.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_time_ms: Option<u64>,
}

impl Expect {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

fn default_format_version() -> u32 {
    1
}

fn default_timeout_ms() -> u64 {
    30_000
}

fn default_true() -> bool {
    true
}
