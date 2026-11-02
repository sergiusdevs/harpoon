//! Motor de variables: resolución por ámbitos e interpolación de `{{nombre}}`.
//!
//! Prioridad (de mayor a menor): runtime (scripts) → entorno → carpetas anidadas → colección.

use indexmap::IndexMap;

use crate::{Error, Result};

/// Conjunto de variables resuelto, con sus capas en orden de prioridad.
#[derive(Debug, Default, Clone)]
pub struct Scope {
    layers: Vec<IndexMap<String, String>>,
}

impl Scope {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Añade una capa con **mayor** prioridad que las anteriores.
    #[must_use]
    pub fn with_layer(mut self, layer: IndexMap<String, String>) -> Self {
        self.layers.push(layer);
        self
    }

    /// Define o sobrescribe una variable en la capa más prioritaria.
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        if self.layers.is_empty() {
            self.layers.push(IndexMap::new());
        }
        if let Some(top) = self.layers.last_mut() {
            top.insert(key.into(), value.into());
        }
    }

    #[must_use]
    pub fn get(&self, key: &str) -> Option<&str> {
        self.layers
            .iter()
            .rev()
            .find_map(|l| l.get(key).map(String::as_str))
    }

    /// Todas las variables visibles, ya resueltas por prioridad.
    #[must_use]
    pub fn flatten(&self) -> IndexMap<String, String> {
        let mut out = IndexMap::new();
        for layer in &self.layers {
            for (k, v) in layer {
                out.insert(k.clone(), v.clone());
            }
        }
        out
    }

    /// Sustituye cada `{{nombre}}` por su valor. Falla si falta alguna.
    pub fn interpolate(&self, input: &str) -> Result<String> {
        let mut out = String::with_capacity(input.len());
        let mut rest = input;
        while let Some(start) = rest.find("{{") {
            out.push_str(&rest[..start]);
            let after = &rest[start + 2..];
            let Some(end) = after.find("}}") else {
                // `{{` sin cierre: se deja literal.
                out.push_str(&rest[start..]);
                return Ok(out);
            };
            let name = after[..end].trim();
            if let Some(dynamic) = dynamic_value(name) {
                out.push_str(&dynamic);
            } else {
                let value = self
                    .get(name)
                    .ok_or_else(|| Error::UndefinedVariable(name.to_owned()))?;
                out.push_str(value);
            }
            rest = &after[end + 2..];
        }
        out.push_str(rest);
        Ok(out)
    }
}

/// Variables dinámicas: `{{$uuid}}`, `{{$timestamp}}`, `{{$isoTimestamp}}`,
/// `{{$randomInt}}` / `{{$randomInt 1 100}}`, `{{$randomHex 16}}`.
fn dynamic_value(name: &str) -> Option<String> {
    let name = name.strip_prefix('$')?;
    let mut parts = name.split_whitespace();
    let func = parts.next()?;
    let args: Vec<&str> = parts.collect();
    match func {
        "uuid" | "guid" => Some(uuid::Uuid::new_v4().to_string()),
        "timestamp" => Some(unix_time().to_string()),
        "isoTimestamp" => Some(iso_now()),
        "randomInt" => {
            let lo: i64 = args.first().and_then(|a| a.parse().ok()).unwrap_or(0);
            let hi: i64 = args.get(1).and_then(|a| a.parse().ok()).unwrap_or(1000);
            let (lo, hi) = if lo <= hi { (lo, hi) } else { (hi, lo) };
            let span = u64::try_from(hi - lo).unwrap_or(0) + 1;
            let r = u64::from_le_bytes(uuid::Uuid::new_v4().as_bytes()[..8].try_into().ok()?);
            Some((lo + i64::try_from(r % span).unwrap_or(0)).to_string())
        }
        "randomHex" => {
            let n: usize = args.first().and_then(|a| a.parse().ok()).unwrap_or(16);
            let mut s = String::new();
            while s.len() < n {
                s.push_str(&uuid::Uuid::new_v4().simple().to_string());
            }
            s.truncate(n);
            Some(s)
        }
        _ => None,
    }
}

fn unix_time() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// `YYYY-MM-DDTHH:MM:SSZ` sin dependencias de calendario.
#[allow(clippy::many_single_char_names)]
pub(crate) fn iso_now() -> String {
    let secs = unix_time();
    let days = secs / 86_400;
    let rem = secs % 86_400;
    let (h, m, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    // Algoritmo de días civiles (Howard Hinnant).
    let z = i64::try_from(days).unwrap_or(0) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if mo <= 2 { y + 1 } else { y };
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{m:02}:{s:02}Z")
}

#[cfg(test)]
mod tests;
