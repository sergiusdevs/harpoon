//! Servidor HTTP mock vinculado únicamente al bucle local.
use axum::{
    Router,
    body::Body,
    extract::{Request, State},
    http::{HeaderName, HeaderValue, StatusCode},
    response::Response,
    routing::any,
};
use harpoon_core::Method;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    net::{Ipv4Addr, SocketAddr},
    path::Path,
    sync::Arc,
};
use tokio::{net::TcpListener, sync::oneshot, task::JoinHandle};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Route {
    #[serde(default = "yes")]
    pub enabled: bool,
    pub method: Method,
    pub path: String,
    #[serde(default = "status_ok")]
    pub status: u16,
    #[serde(default)]
    pub headers: IndexMap<String, String>,
    #[serde(default)]
    pub body: String,
}
const fn yes() -> bool {
    true
}
const fn status_ok() -> u16 {
    200
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub routes: Vec<Route>,
}
impl Config {
    pub fn validate(&self) -> Result<(), String> {
        let mut seen = HashSet::new();
        for (i, route) in self.routes.iter().enumerate() {
            let label = format!("Ruta {}", i + 1);
            if !route.path.starts_with('/')
                || route.path.contains(['?', '#'])
                || route.path.chars().any(char::is_whitespace)
            {
                return Err(format!(
                    "{label}: usa una ruta que empiece por / y no incluya query ni espacios"
                ));
            }
            for part in route.path.split('/') {
                if part.contains(['{', '}'])
                    && !(part.starts_with('{')
                        && part.ends_with('}')
                        && part.len() > 2
                        && !part[1..part.len() - 1].contains(['{', '}']))
                {
                    return Err(format!("{label}: parámetro de ruta no válido"));
                }
            }
            if !(200..=599).contains(&route.status) {
                return Err(format!("{label}: el estado debe estar entre 200 y 599"));
            }
            if route.enabled && !seen.insert((route.method.to_string(), route.path.clone())) {
                return Err(format!("{label}: método y ruta duplicados"));
            }
            for (name, value) in &route.headers {
                let header = HeaderName::from_bytes(name.as_bytes())
                    .map_err(|_| format!("{label}: cabecera {name} no válida"))?;
                HeaderValue::from_str(value)
                    .map_err(|_| format!("{label}: valor de cabecera {name} no válido"))?;
                if matches!(
                    header.as_str(),
                    "content-length" | "transfer-encoding" | "connection"
                ) {
                    return Err(format!(
                        "{label}: la cabecera {name} la gestiona el servidor"
                    ));
                }
            }
        }
        Ok(())
    }
}
pub fn load(root: &Path) -> Result<Config, String> {
    let path = root.join("mocks.toml");
    if !path.exists() {
        return Ok(Config::default());
    }
    let data = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let config: Config = toml::from_str(&data).map_err(|e| e.to_string())?;
    config.validate()?;
    Ok(config)
}
pub fn save(root: &Path, config: &Config) -> Result<(), String> {
    config.validate()?;
    let data = toml::to_string_pretty(config).map_err(|e| e.to_string())?;
    std::fs::write(root.join("mocks.toml"), data).map_err(|e| e.to_string())
}
fn matches_path(pattern: &str, path: &str) -> bool {
    let expected: Vec<_> = pattern.split('/').collect();
    let actual: Vec<_> = path.split('/').collect();
    expected.len() == actual.len()
        && expected
            .iter()
            .zip(actual)
            .all(|(a, b)| (*a == b) || (a.starts_with('{') && a.ends_with('}') && !b.is_empty()))
}
async fn handle(State(config): State<Arc<Config>>, request: Request) -> Response {
    let found = config.routes.iter().find(|route| {
        route.enabled
            && route.method.to_string() == request.method().as_str()
            && matches_path(&route.path, request.uri().path())
    });
    let Some(route) = found else {
        return Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("No hay una ruta mock configurada"))
            .expect("respuesta estática válida");
    };
    let mut response = Response::builder().status(route.status);
    for (name, value) in &route.headers {
        response = response.header(name, value);
    }
    response
        .body(Body::from(route.body.clone()))
        .expect("configuración validada")
}
pub struct Server {
    pub address: SocketAddr,
    shutdown: Option<oneshot::Sender<()>>,
    task: JoinHandle<std::io::Result<()>>,
}
impl Server {
    #[must_use]
    pub fn finished(&self) -> bool {
        self.task.is_finished()
    }
    pub async fn stop(mut self) -> Result<(), String> {
        if let Some(sender) = self.shutdown.take() {
            let _ = sender.send(());
        }
        if let Ok(result) =
            tokio::time::timeout(std::time::Duration::from_secs(3), &mut self.task).await
        {
            result
                .map_err(|e| e.to_string())?
                .map_err(|e| e.to_string())
        } else {
            self.task.abort();
            Ok(())
        }
    }
}
pub async fn start(config: Config, port: u16) -> Result<Server, String> {
    config.validate()?;
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, port))
        .await
        .map_err(|e| format!("No se pudo abrir el puerto {port}: {e}"))?;
    let address = listener.local_addr().map_err(|e| e.to_string())?;
    let app = Router::new()
        .fallback(any(handle))
        .with_state(Arc::new(config));
    let (sender, receiver) = oneshot::channel();
    let task = tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = receiver.await;
            })
            .await
    });
    Ok(Server {
        address,
        shutdown: Some(sender),
        task,
    })
}
