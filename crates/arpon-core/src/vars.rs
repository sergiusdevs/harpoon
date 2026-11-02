//! Motor de variables: resolución por ámbitos e interpolación de `{{nombre}}`.
//!
//! Prioridad (de mayor a menor): runtime (scripts) → entorno → colección.

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
            let value = self
                .get(name)
                .ok_or_else(|| Error::UndefinedVariable(name.to_owned()))?;
            out.push_str(value);
            rest = &after[end + 2..];
        }
        out.push_str(rest);
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layer(pairs: &[(&str, &str)]) -> IndexMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect()
    }

    #[test]
    fn interpola_y_respeta_prioridad() {
        let scope = Scope::new()
            .with_layer(layer(&[("base_url", "http://col"), ("v", "1")]))
            .with_layer(layer(&[("base_url", "http://env")]));
        assert_eq!(
            scope.interpolate("{{base_url}}/api/v{{ v }}").unwrap(),
            "http://env/api/v1"
        );
    }

    #[test]
    fn variable_sin_definir_es_error() {
        let scope = Scope::new();
        assert!(matches!(
            scope.interpolate("{{nope}}"),
            Err(Error::UndefinedVariable(n)) if n == "nope"
        ));
    }

    #[test]
    fn llaves_sin_cerrar_quedan_literales() {
        let scope = Scope::new();
        assert_eq!(scope.interpolate("a {{ b").unwrap(), "a {{ b");
    }
}
