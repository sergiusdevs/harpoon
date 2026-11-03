//! Secretos locales: valores que nunca van a Git.
//!
//! En los archivos versionados solo queda el *nombre* de cada secreto
//! (`secrets = ["token"]` en `harpoon.toml`, `folder.toml` o
//! `environments/<env>.toml`). Los valores se leen, por orden, de:
//!
//! 1. El archivo local ignorado: `environments/<env>.secrets.toml` para los
//!    entornos y `.harpoon/secrets.toml` para colección y carpetas.
//! 2. Una variable de entorno del sistema `HARPOON_<NOMBRE>` (útil en CI):
//!    `api-key` → `HARPOON_API_KEY`.
//!
//! Cada carpeta que guarda secretos o estado lleva su propio `.gitignore`, así
//! que siguen ignorados aunque la colección esté dentro de otro repositorio.

use std::path::Path;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::Result;

/// Archivo de secretos de colección y carpetas, dentro de `.harpoon/`.
pub const LOCAL_FILE: &str = "secrets.toml";

/// `.harpoon/secrets.toml`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalSecrets {
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub collection: IndexMap<String, String>,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub folders: IndexMap<String, IndexMap<String, String>>,
}

impl LocalSecrets {
    pub fn is_empty(&self) -> bool {
        self.collection.is_empty() && self.folders.values().all(IndexMap::is_empty)
    }
}

/// Nombre de la variable de entorno del sistema para un secreto.
#[must_use]
pub fn env_var_name(name: &str) -> String {
    let mut out = String::from("HARPOON_");
    for c in name.chars() {
        out.push(if c.is_ascii_alphanumeric() {
            c.to_ascii_uppercase()
        } else {
            '_'
        });
    }
    out
}

/// Valor de `HARPOON_<NOMBRE>` si existe y no está vacío.
#[must_use]
pub fn from_process_env(name: &str) -> Option<String> {
    std::env::var(env_var_name(name))
        .ok()
        .filter(|v| !v.is_empty())
}

/// Copia de `vars` donde los secretos sin valor toman `HARPOON_<NOMBRE>`.
#[must_use]
pub fn with_env_fallback(
    vars: &IndexMap<String, String>,
    secrets: &[String],
) -> IndexMap<String, String> {
    let mut out = vars.clone();
    for name in secrets {
        if out.get(name).is_none_or(String::is_empty)
            && let Some(value) = from_process_env(name)
        {
            out.insert(name.clone(), value);
        }
    }
    out
}

/// Un secreto declarado que no tiene valor en esta máquina.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MissingSecret {
    /// `collection`, `folder` o `environment`.
    pub scope: &'static str,
    /// Carpeta o entorno (vacío para la colección).
    pub owner: String,
    pub name: String,
    /// Variable del sistema que también lo resolvería.
    pub env_var: String,
}

const STATE_IGNORE: &str = "# Generado por Harpoon: estado local y secretos, nunca en Git.\n*\n";
const ENV_IGNORE: &str =
    "# Generado por Harpoon: los valores secretos no van a Git.\n*.secrets.toml\n";

/// Asegura los `.gitignore` anidados de `.harpoon/` y `environments/`.
/// Solo escribe si falta la regla; no toca otras líneas del archivo.
pub fn ensure_gitignores(root: &Path) -> Result<()> {
    let state = root.join(crate::collection::STATE_DIR);
    if state.is_dir() {
        ensure_rule(&state.join(".gitignore"), "*", STATE_IGNORE)?;
    }
    let envs = root.join(crate::collection::ENVIRONMENTS_DIR);
    if envs.is_dir() {
        ensure_rule(&envs.join(".gitignore"), "*.secrets.toml", ENV_IGNORE)?;
    }
    Ok(())
}

fn ensure_rule(path: &Path, rule: &str, template: &str) -> Result<()> {
    let current = std::fs::read_to_string(path).unwrap_or_default();
    if current.lines().any(|l| l.trim() == rule) {
        return Ok(());
    }
    let next = if current.trim().is_empty() {
        template.to_owned()
    } else {
        format!("{}\n{rule}\n", current.trim_end())
    };
    std::fs::write(path, next).map_err(|source| crate::Error::Io {
        path: path.to_path_buf(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nombre_de_variable_de_entorno() {
        assert_eq!(env_var_name("token"), "HARPOON_TOKEN");
        assert_eq!(env_var_name("api-key.v2"), "HARPOON_API_KEY_V2");
    }

    #[test]
    fn gitignore_anidado_se_crea_y_respeta_lo_existente() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".harpoon")).unwrap();
        std::fs::create_dir_all(dir.path().join("environments")).unwrap();
        std::fs::write(dir.path().join("environments/.gitignore"), "local.toml\n").unwrap();
        ensure_gitignores(dir.path()).unwrap();
        ensure_gitignores(dir.path()).unwrap();
        let state = std::fs::read_to_string(dir.path().join(".harpoon/.gitignore")).unwrap();
        assert!(state.lines().any(|l| l == "*"));
        let envs = std::fs::read_to_string(dir.path().join("environments/.gitignore")).unwrap();
        assert_eq!(envs, "local.toml\n*.secrets.toml\n");
    }
}
