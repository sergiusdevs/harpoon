//! Consola y red de la app: un registro acotado en memoria, en Rust.
//!
//! Cada envío (solicitud suelta, runner…) pasa por `send_request`, que llama a
//! [`DevTools::record`]. Se guardan como mucho [`MAX`] entradas de cada tipo y
//! la interfaz recibe solo lo nuevo por el evento [`EVENT`], de una vez por
//! envío; nunca vuelve a pedir el registro entero salvo al arrancar.

use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use tauri::{AppHandle, Emitter};

/// Entradas máximas que se conservan de cada tipo.
pub const MAX: usize = 2000;
/// Evento con las entradas nuevas (`Batch`).
pub const EVENT: &str = "devtools://entries";

#[derive(Clone, Serialize)]
pub struct LogEntry {
    pub id: u64,
    /// Milisegundos desde epoch.
    pub at: u64,
    /// `log`, `info`, `warn`, `error` o `debug`.
    pub level: &'static str,
    pub message: String,
    /// Clave de la solicitud que lo generó.
    pub source: String,
}

#[derive(Clone, Serialize)]
pub struct NetEntry {
    pub id: u64,
    pub at: u64,
    pub key: String,
    pub method: String,
    pub domain: String,
    pub path: String,
    pub status: Option<u16>,
    pub duration_ms: Option<u64>,
    pub size: Option<usize>,
    pub error: Option<String>,
}

#[derive(Clone, Serialize, Default)]
pub struct Batch {
    pub logs: Vec<LogEntry>,
    pub net: Vec<NetEntry>,
}

#[derive(Default)]
struct Inner {
    seq: u64,
    logs: VecDeque<LogEntry>,
    net: VecDeque<NetEntry>,
}

#[derive(Default)]
pub struct DevTools(Mutex<Inner>);

/// Lo que se sabe de un envío al terminar.
pub struct Record<'a> {
    pub key: &'a str,
    pub method: String,
    pub url: &'a str,
    pub query: Option<&'a indexmap::IndexMap<String, String>>,
    pub status: Option<u16>,
    pub duration_ms: Option<u64>,
    pub size: Option<usize>,
    pub error: Option<String>,
    pub logs: &'a [String],
    /// Errores de scripts (`script post`, …) que conviene ver en la consola.
    pub script_errors: Vec<String>,
}

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

/// Separa el prefijo `[nivel] ` que pone el preludio de los scripts.
pub fn split_level(line: &str) -> (&'static str, &str) {
    for level in ["info", "warn", "error", "debug"] {
        if let Some(rest) = line
            .strip_prefix('[')
            .and_then(|s| s.strip_prefix(level))
            .and_then(|s| s.strip_prefix("] "))
        {
            return (level, rest);
        }
    }
    ("log", line)
}

/// `https://api.x.com:8080/a/b?c=1` → (`api.x.com:8080`, `/a/b?c=1`).
pub fn split_url(
    url: &str,
    query: Option<&indexmap::IndexMap<String, String>>,
) -> (String, String) {
    let rest = url.split_once("://").map_or(url, |(_, r)| r);
    let (domain, path) = match rest.find(['/', '?', '#']) {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, ""),
    };
    let domain = domain.rsplit('@').next().unwrap_or(domain).to_string();
    let mut path = if path.is_empty() || path.starts_with('?') {
        format!("/{path}")
    } else {
        path.to_string()
    };
    if let Some(q) = query.filter(|q| !q.is_empty()) {
        let pairs: Vec<String> = q
            .iter()
            .map(|(k, v)| {
                if v.is_empty() {
                    k.clone()
                } else {
                    format!("{k}={v}")
                }
            })
            .collect();
        path.push(if path.contains('?') { '&' } else { '?' });
        path.push_str(&pairs.join("&"));
    }
    (domain, path)
}

fn push<T>(list: &mut VecDeque<T>, item: T) {
    if list.len() == MAX {
        list.pop_front();
    }
    list.push_back(item);
}

impl DevTools {
    /// Guarda un envío y emite solo sus entradas nuevas.
    pub fn record(&self, app: &AppHandle, r: Record<'_>) {
        let at = now_ms();
        let batch = {
            let Ok(mut inner) = self.0.lock() else { return };
            let mut batch = Batch::default();
            let mut next = || {
                inner.seq += 1;
                inner.seq
            };
            for line in r.logs {
                let (level, message) = split_level(line);
                batch.logs.push(LogEntry {
                    id: next(),
                    at,
                    level,
                    message: message.to_string(),
                    source: r.key.to_string(),
                });
            }
            for message in r.script_errors {
                batch.logs.push(LogEntry {
                    id: next(),
                    at,
                    level: "error",
                    message,
                    source: r.key.to_string(),
                });
            }
            if let Some(error) = &r.error {
                batch.logs.push(LogEntry {
                    id: next(),
                    at,
                    level: "error",
                    message: error.clone(),
                    source: r.key.to_string(),
                });
            }
            let (domain, path) = split_url(r.url, r.query);
            batch.net.push(NetEntry {
                id: next(),
                at,
                key: r.key.to_string(),
                method: r.method,
                domain,
                path,
                status: r.status,
                duration_ms: r.duration_ms,
                size: r.size,
                error: r.error,
            });
            for entry in &batch.logs {
                push(&mut inner.logs, entry.clone());
            }
            for entry in &batch.net {
                push(&mut inner.net, entry.clone());
            }
            batch
        };
        let _ = app.emit(EVENT, batch);
    }
}

/// Todo lo registrado (al abrir la app o recargar la ventana).
#[tauri::command]
pub fn devtools_snapshot(state: tauri::State<'_, DevTools>) -> Batch {
    let inner = state
        .0
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Batch {
        logs: inner.logs.iter().cloned().collect(),
        net: inner.net.iter().cloned().collect(),
    }
}

/// Vacía la consola (`logs`), la red (`net`) o ambas (cualquier otro valor).
#[tauri::command]
pub fn devtools_clear(state: tauri::State<'_, DevTools>, kind: String) {
    let mut inner = state
        .0
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    match kind.as_str() {
        "logs" => inner.logs.clear(),
        "net" => inner.net.clear(),
        _ => {
            inner.logs.clear();
            inner.net.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn separa_niveles() {
        assert_eq!(split_level("[warn] ojo"), ("warn", "ojo"));
        assert_eq!(split_level("[error] x"), ("error", "x"));
        assert_eq!(split_level("hola [warn]"), ("log", "hola [warn]"));
        assert_eq!(split_level("[warning] x"), ("log", "[warning] x"));
    }

    #[test]
    fn separa_dominio_y_ruta() {
        assert_eq!(
            split_url("https://www.x.es/a/b?c=1", None),
            ("www.x.es".into(), "/a/b?c=1".into())
        );
        assert_eq!(
            split_url("http://localhost:8080", None),
            ("localhost:8080".into(), "/".into())
        );
        let mut q = indexmap::IndexMap::new();
        q.insert("id".to_string(), "3".to_string());
        assert_eq!(
            split_url("https://h/p", Some(&q)),
            ("h".into(), "/p?id=3".into())
        );
        assert_eq!(
            split_url("https://h/p?a=1", Some(&q)),
            ("h".into(), "/p?a=1&id=3".into())
        );
        assert_eq!(
            split_url("{{base_url}}/users", None),
            ("{{base_url}}".into(), "/users".into())
        );
    }
}
