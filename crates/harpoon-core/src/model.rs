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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ApiKeyLocation {
    #[default]
    Header,
    Query,
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
    Text {
        text: String,
    },
    Form {
        fields: IndexMap<String, String>,
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

/// Una solicitud, tal y como se guarda en `requests/**/*.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Request {
    pub name: String,
    #[serde(default)]
    pub method: Method,
    pub url: String,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub headers: IndexMap<String, String>,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub query: IndexMap<String, String>,
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
    pub fn new(name: impl Into<String>, method: Method, url: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            method,
            url: url.into(),
            headers: IndexMap::new(),
            query: IndexMap::new(),
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
    /// Proxy explícito (`http://host:puerto`). Si no se indica, se respetan
    /// `HTTP_PROXY`/`HTTPS_PROXY`/`NO_PROXY` del entorno.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proxy: Option<String>,
    /// No verificar el certificado TLS del servidor (solo desarrollo).
    #[serde(default)]
    pub insecure: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            timeout_ms: default_timeout_ms(),
            follow_redirects: true,
            proxy: None,
            insecure: false,
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
