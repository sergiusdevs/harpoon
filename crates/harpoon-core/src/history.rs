//! Historial local de respuestas por solicitud: `.harpoon/history/<clave>.jsonl`
//! (fuera de Git). Se conservan las últimas [`MAX_ENTRIES`] por solicitud y
//! los cuerpos se truncan a [`MAX_BODY`] bytes.

use std::{
    fs,
    io::Write as _,
    path::{Path, PathBuf},
};

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{Error, Result, collection::STATE_DIR};

pub const MAX_ENTRIES: usize = 50;
pub const MAX_BODY: usize = 256 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HistoryEntry {
    /// Marca de tiempo ISO-8601 (UTC).
    pub at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env: Option<String>,
    pub method: String,
    pub url: String,
    pub status: u16,
    pub elapsed_ms: u64,
    pub size: usize,
    #[serde(default)]
    pub headers: IndexMap<String, String>,
    /// Cuerpo (texto; truncado si supera `MAX_BODY`).
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub truncated: bool,
    /// Comprobaciones pasadas / totales.
    #[serde(default)]
    pub checks_passed: usize,
    #[serde(default)]
    pub checks_total: usize,
}

fn file_for(root: &Path, key: &str) -> PathBuf {
    root.join(STATE_DIR)
        .join("history")
        .join(format!("{}.jsonl", key.replace('/', "__")))
}

/// Añade una entrada y recorta el archivo a las últimas `MAX_ENTRIES`.
pub fn append(root: &Path, key: &str, mut entry: HistoryEntry) -> Result<()> {
    redact(&mut entry);
    if entry.body.len() > MAX_BODY {
        let mut cut = MAX_BODY;
        while !entry.body.is_char_boundary(cut) {
            cut -= 1;
        }
        entry.body.truncate(cut);
        entry.truncated = true;
    }
    let path = file_for(root, key);
    let mut entries = load(root, key);
    entries.push(entry);
    if entries.len() > MAX_ENTRIES {
        entries.drain(0..entries.len() - MAX_ENTRIES);
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| Error::Io {
            path: parent.into(),
            source,
        })?;
    }
    let mut file = fs::File::create(&path).map_err(|source| Error::Io {
        path: path.clone(),
        source,
    })?;
    for e in &entries {
        let line = serde_json::to_string(e)?;
        writeln!(file, "{line}").map_err(|source| Error::Io {
            path: path.clone(),
            source,
        })?;
    }
    crate::secrets::ensure_gitignores(root)
}

/// Texto que sustituye a un valor sensible en el historial.
pub const HIDDEN: &str = "‹oculto›";

/// El historial no guarda credenciales: oculta headers sensibles de la
/// respuesta (`Set-Cookie`, `Authorization`…) y parámetros sensibles de la URL.
pub fn redact(entry: &mut HistoryEntry) {
    for (name, value) in &mut entry.headers {
        if crate::scan::is_sensitive_name(name) || name.eq_ignore_ascii_case("set-cookie") {
            HIDDEN.clone_into(value);
        }
    }
    entry.url = redact_url(&entry.url);
}

fn redact_url(url: &str) -> String {
    let (base, fragment) = url
        .split_once('#')
        .map_or((url, None), |(b, f)| (b, Some(f)));
    let Some((path, query)) = base.split_once('?') else {
        return url.to_owned();
    };
    let query: Vec<String> = query
        .split('&')
        .map(|pair| match pair.split_once('=') {
            Some((name, _)) if crate::scan::is_sensitive_name(name) => format!("{name}={HIDDEN}"),
            _ => pair.to_owned(),
        })
        .collect();
    let mut out = format!("{path}?{}", query.join("&"));
    if let Some(f) = fragment {
        out.push('#');
        out.push_str(f);
    }
    out
}

/// Entradas de más antigua a más reciente (vacío si no hay archivo o está corrupto).
#[must_use]
pub fn load(root: &Path, key: &str) -> Vec<HistoryEntry> {
    let path = file_for(root, key);
    let Ok(text) = fs::read_to_string(path) else {
        return Vec::new();
    };
    text.lines()
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect()
}

pub fn clear(root: &Path, key: &str) -> Result<()> {
    let path = file_for(root, key);
    if path.exists() {
        fs::remove_file(&path).map_err(|source| Error::Io { path, source })?;
    }
    Ok(())
}

/// Marca de tiempo actual en ISO-8601 UTC (`YYYY-MM-DDTHH:MM:SSZ`).
#[must_use]
pub fn now_iso() -> String {
    crate::vars::iso_now()
}

#[cfg(test)]
mod tests;
