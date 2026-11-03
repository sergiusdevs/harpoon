//! Importar y exportar entornos.
//!
//! Se exportan todas las variables con su valor **menos los secretos**, que
//! salen con valor vacío (`""`). Formatos:
//! - **Postman** (JSON de entorno): los secretos van con `type: "secret"`.
//! - **Harpoon** (TOML, igual que `environments/<env>.toml`): `secrets = [...]`
//!   lista los nombres.
//!
//! Al importar, las variables `secret` de Postman (o las listadas en `secrets`)
//! se marcan como secretas; si el archivo trae su valor, se guarda solo en local.

use std::path::Path;

use serde::Serialize;
use serde_json::{Value, json};

use crate::{Environment, Error, Result};

const MAX_BYTES: u64 = 5 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnvFormat {
    Postman,
    Harpoon,
}

/// Entorno leído de un archivo, listo para guardar.
#[derive(Debug, Clone, Serialize)]
pub struct ImportedEnv {
    /// Nombre propuesto (del archivo o de su nombre).
    pub name: String,
    pub variables: usize,
    pub secrets: Vec<String>,
    pub warnings: Vec<String>,
    #[serde(skip)]
    pub env: Environment,
}

/// Contenido del archivo exportado.
#[must_use]
pub fn export(name: &str, env: &Environment, format: EnvFormat) -> String {
    let value_of = |k: &String, v: &String| {
        if env.secrets.contains(k) {
            String::new()
        } else {
            v.clone()
        }
    };
    match format {
        EnvFormat::Postman => {
            let mut values: Vec<Value> = env
                .vars
                .iter()
                .map(|(k, v)| {
                    let kind = if env.secrets.contains(k) {
                        "secret"
                    } else {
                        "default"
                    };
                    json!({"key": k, "value": value_of(k, v), "type": kind, "enabled": true})
                })
                .collect();
            // Secretos declarados sin valor en esta máquina: también se listan.
            for s in env.secrets.iter().filter(|s| !env.vars.contains_key(*s)) {
                values.push(json!({"key": s, "value": "", "type": "secret", "enabled": true}));
            }
            let out =
                json!({"name": name, "values": values, "_postman_variable_scope": "environment"});
            serde_json::to_string_pretty(&out).unwrap_or_default()
        }
        EnvFormat::Harpoon => {
            let public = Environment {
                vars: env
                    .vars
                    .iter()
                    .map(|(k, v)| (k.clone(), value_of(k, v)))
                    .collect(),
                secrets: env.secrets.clone(),
                descriptions: env.descriptions.clone(),
            };
            toml::to_string_pretty(&public).unwrap_or_default()
        }
    }
}

/// Lee un entorno de Postman (JSON) o de Harpoon (TOML).
pub fn read(path: &Path) -> Result<ImportedEnv> {
    let io = |source| Error::Io {
        path: path.to_path_buf(),
        source,
    };
    if std::fs::metadata(path).map_err(io)?.len() > MAX_BYTES {
        return Err(Error::Exchange(
            "archivo de entorno demasiado grande (máximo 5 MB)".into(),
        ));
    }
    let text = std::fs::read_to_string(path).map_err(io)?;
    let stem = path
        .file_stem()
        .map(|s| {
            s.to_string_lossy()
                .trim_end_matches(".postman_environment")
                .to_owned()
        })
        .unwrap_or_default();
    let mut imported = if text.trim_start().starts_with('{') {
        from_postman(&text)?
    } else {
        from_harpoon(&text)?
    };
    if imported.name.is_empty() {
        imported.name = stem;
    }
    imported.name = sanitize_name(&imported.name);
    imported.variables = imported.env.vars.len();
    imported.secrets.clone_from(&imported.env.secrets);
    let with_values: Vec<&String> = imported
        .env
        .secrets
        .iter()
        .filter(|s| imported.env.vars.get(*s).is_some_and(|v| !v.is_empty()))
        .collect();
    if !with_values.is_empty() {
        let list = with_values
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        imported.warnings.push(format!(
            "El archivo trae el valor de secretos ({list}): se guardarán solo en este equipo, fuera de Git. Borra el archivo si no lo necesitas."
        ));
    }
    Ok(imported)
}

fn from_postman(text: &str) -> Result<ImportedEnv> {
    let v: Value = serde_json::from_str(text)?;
    let values = v["values"]
        .as_array()
        .ok_or_else(|| Error::Exchange("no es un entorno de Postman (falta `values`)".into()))?;
    let mut env = Environment::default();
    let mut warnings = Vec::new();
    let mut disabled = 0;
    for item in values {
        let Some(key) = item["key"].as_str().filter(|k| !k.trim().is_empty()) else {
            continue;
        };
        if item["enabled"] == false {
            disabled += 1;
            continue;
        }
        let value = match &item["value"] {
            Value::String(s) => s.clone(),
            Value::Null => String::new(),
            other => other.to_string(),
        };
        if item["type"] == "secret" {
            env.secrets.push(key.to_owned());
        }
        env.vars.insert(key.to_owned(), value);
    }
    if disabled > 0 {
        warnings.push(format!(
            "{disabled} variable(s) desactivadas en Postman no se importan"
        ));
    }
    Ok(ImportedEnv {
        name: v["name"].as_str().unwrap_or_default().to_owned(),
        variables: 0,
        secrets: Vec::new(),
        warnings,
        env,
    })
}

fn from_harpoon(text: &str) -> Result<ImportedEnv> {
    let env: Environment = toml::from_str(text)
        .map_err(|e| Error::Exchange(format!("TOML de entorno no válido: {e}")))?;
    Ok(ImportedEnv {
        name: String::new(),
        variables: 0,
        secrets: Vec::new(),
        warnings: Vec::new(),
        env,
    })
}

/// Nombre de archivo seguro para `environments/<nombre>.toml`.
#[must_use]
pub fn sanitize_name(name: &str) -> String {
    let s: String = name
        .trim()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || matches!(c, '-' | '_') {
                c
            } else {
                '-'
            }
        })
        .collect();
    let s = s.trim_matches('-').to_lowercase();
    let s = s.strip_suffix("-secrets").unwrap_or(&s).to_owned();
    if s.is_empty() {
        "importado".into()
    } else {
        s.chars().take(60).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::IndexMap;

    fn env() -> Environment {
        Environment {
            vars: IndexMap::from([
                ("base_url".to_string(), "https://api.x.com".to_string()),
                ("token".to_string(), "tok-super-secreto".to_string()),
            ]),
            secrets: vec!["token".into(), "api_key".into()],
            descriptions: IndexMap::new(),
        }
    }

    #[test]
    fn exporta_sin_valores_secretos() {
        for format in [EnvFormat::Postman, EnvFormat::Harpoon] {
            let out = export("dev", &env(), format);
            assert!(!out.contains("tok-super-secreto"), "{out}");
            assert!(
                out.contains("https://api.x.com")
                    && out.contains("token")
                    && out.contains("api_key"),
                "{out}"
            );
        }
        let postman: Value =
            serde_json::from_str(&export("dev", &env(), EnvFormat::Postman)).unwrap();
        let token = postman["values"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["key"] == "token")
            .unwrap();
        assert_eq!(
            (token["type"].as_str(), token["value"].as_str()),
            (Some("secret"), Some(""))
        );
    }

    #[test]
    fn importa_postman_y_harpoon() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("Prod API.postman_environment.json");
        std::fs::write(&p, r#"{"name":"Prod API","values":[{"key":"base_url","value":"https://p","enabled":true},{"key":"token","value":"","type":"secret","enabled":true},{"key":"old","value":"x","enabled":false}]}"#).unwrap();
        let i = read(&p).unwrap();
        assert_eq!(i.name, "prod-api");
        assert_eq!(i.env.vars["base_url"], "https://p");
        assert_eq!(i.secrets, vec!["token".to_string()]);
        assert!(i.warnings.iter().any(|w| w.contains("desactivadas")));

        let h = dir.path().join("staging.toml");
        std::fs::write(&h, export("staging", &env(), EnvFormat::Harpoon)).unwrap();
        let i = read(&h).unwrap();
        assert_eq!(i.name, "staging");
        assert_eq!(
            i.env.secrets,
            vec!["token".to_string(), "api_key".to_string()]
        );
        assert_eq!(i.env.vars["token"], "");
    }

    #[test]
    fn avisa_si_el_archivo_trae_valores_secretos() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("e.json");
        std::fs::write(
            &p,
            r#"{"name":"e","values":[{"key":"token","value":"abc","type":"secret"}]}"#,
        )
        .unwrap();
        assert!(
            read(&p)
                .unwrap()
                .warnings
                .iter()
                .any(|w| w.contains("valor de secretos"))
        );
    }

    #[test]
    fn nombres_seguros() {
        assert_eq!(sanitize_name("../../etc/passwd"), "etc-passwd");
        assert_eq!(sanitize_name("dev.secrets"), "dev");
        assert_eq!(sanitize_name("   "), "importado");
    }
}
