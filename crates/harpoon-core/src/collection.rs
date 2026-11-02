//! Lectura y escritura de una colección en disco.
//!
//! ```text
//! mi-api/
//! ├── harpoon.toml                  # metadatos, variables y auth de la colección
//! ├── environments/
//! │   ├── dev.toml                # variables del entorno
//! │   └── dev.secrets.toml        # secretos (ignorado por Git)
//! └── requests/
//!     ├── health.toml
//!     └── users/                  # las carpetas organizan las solicitudes
//!         └── list.toml
//! ```

use std::{
    fs,
    path::{Path, PathBuf},
};

use indexmap::IndexMap;
use walkdir::WalkDir;

use crate::{
    Error, Result,
    model::{CollectionMeta, Environment, Request},
    vars::Scope,
};

pub const META_FILE: &str = "harpoon.toml";
pub const REQUESTS_DIR: &str = "requests";
pub const ENVIRONMENTS_DIR: &str = "environments";

/// Una colección cargada en memoria.
#[derive(Debug, Clone)]
pub struct Collection {
    pub root: PathBuf,
    pub meta: CollectionMeta,
    /// Solicitudes indexadas por ruta relativa sin extensión (`users/list`).
    pub requests: IndexMap<String, Request>,
    pub environments: IndexMap<String, Environment>,
}

impl Collection {
    /// Busca `harpoon.toml` desde `start` hacia arriba y carga la colección.
    pub fn discover(start: &Path) -> Result<Self> {
        let mut dir = Some(start);
        while let Some(d) = dir {
            if d.join(META_FILE).is_file() {
                return Self::load(d);
            }
            dir = d.parent();
        }
        Err(Error::CollectionNotFound(start.to_path_buf()))
    }

    /// Carga la colección cuyo raíz es `root`.
    pub fn load(root: &Path) -> Result<Self> {
        let meta: CollectionMeta = read_toml(&root.join(META_FILE))?;

        let mut requests = IndexMap::new();
        let req_dir = root.join(REQUESTS_DIR);
        if req_dir.is_dir() {
            for entry in WalkDir::new(&req_dir)
                .sort_by_file_name()
                .into_iter()
                .filter_map(std::result::Result::ok)
                .filter(|e| e.file_type().is_file())
                .filter(|e| e.path().extension().is_some_and(|x| x == "toml"))
            {
                let path = entry.path();
                let key = path
                    .strip_prefix(&req_dir)
                    .unwrap_or(path)
                    .with_extension("")
                    .to_string_lossy()
                    .replace('\\', "/");
                requests.insert(key, read_toml(path)?);
            }
        }

        let mut environments = IndexMap::new();
        let env_dir = root.join(ENVIRONMENTS_DIR);
        if env_dir.is_dir() {
            let mut files: Vec<PathBuf> = fs::read_dir(&env_dir)
                .map_err(|source| Error::Io {
                    path: env_dir.clone(),
                    source,
                })?
                .filter_map(std::result::Result::ok)
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|x| x == "toml"))
                .filter(|p| !p.to_string_lossy().ends_with(".secrets.toml"))
                .collect();
            files.sort();
            for path in files {
                let name = path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
                let mut env: Environment = read_toml(&path)?;
                // Los secretos se fusionan desde el archivo local si existe.
                let secrets_path = env_dir.join(format!("{name}.secrets.toml"));
                if secrets_path.is_file() {
                    let secrets: Environment = read_toml(&secrets_path)?;
                    env.vars.extend(secrets.vars);
                }
                environments.insert(name, env);
            }
        }

        Ok(Self {
            root: root.to_path_buf(),
            meta,
            requests,
            environments,
        })
    }

    /// Crea en disco una colección vacía con un ejemplo.
    pub fn init(root: &Path, name: &str) -> Result<Self> {
        let meta = CollectionMeta {
            name: name.to_owned(),
            format: 1,
            default_environment: Some("dev".into()),
            vars: IndexMap::new(),
            auth: crate::Auth::None,
        };
        write_toml(&root.join(META_FILE), &meta)?;

        let mut dev = Environment::default();
        dev.vars
            .insert("base_url".into(), "http://localhost:8080".into());
        write_toml(&root.join(ENVIRONMENTS_DIR).join("dev.toml"), &dev)?;

        let mut req = Request::new("Health check", crate::Method::Get, "{{base_url}}/health");
        req.headers
            .insert("Accept".into(), "application/json".into());
        req.docs = "Comprueba que el servicio responde.".into();
        write_toml(&root.join(REQUESTS_DIR).join("health.toml"), &req)?;

        let gitignore = root.join(".gitignore");
        if !gitignore.exists() {
            write_file(
                &gitignore,
                "# secretos locales de Harpoon\nenvironments/*.secrets.toml\n",
            )?;
        }

        Self::load(root)
    }

    /// Construye el ámbito de variables para un entorno (o el predeterminado).
    pub fn scope(&self, environment: Option<&str>) -> Result<Scope> {
        let mut scope = Scope::new().with_layer(self.meta.vars.clone());
        let env_name = environment.or(self.meta.default_environment.as_deref());
        if let Some(name) = env_name {
            let env = self
                .environments
                .get(name)
                .ok_or_else(|| Error::UnknownEnvironment(name.to_owned()))?;
            scope = scope.with_layer(env.vars.clone());
        }
        Ok(scope)
    }
}

fn read_toml<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    let text = fs::read_to_string(path).map_err(|source| Error::Io {
        path: path.into(),
        source,
    })?;
    toml::from_str(&text).map_err(|source| Error::Parse {
        path: path.into(),
        source,
    })
}

fn write_toml<T: serde::Serialize>(path: &Path, value: &T) -> Result<()> {
    write_file(path, &toml::to_string_pretty(value)?)
}

fn write_file(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| Error::Io {
            path: parent.into(),
            source,
        })?;
    }
    fs::write(path, contents).map_err(|source| Error::Io {
        path: path.into(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_y_load_son_simetricos() {
        let tmp = tempfile::tempdir().unwrap();
        let col = Collection::init(tmp.path(), "demo").unwrap();
        assert_eq!(col.meta.name, "demo");
        assert_eq!(col.requests.len(), 1);
        assert!(col.requests.contains_key("health"));
        assert!(col.environments.contains_key("dev"));

        let scope = col.scope(None).unwrap();
        assert_eq!(
            scope.interpolate(&col.requests["health"].url).unwrap(),
            "http://localhost:8080/health"
        );
    }

    #[test]
    fn discover_sube_directorios_y_fusiona_secretos() {
        let tmp = tempfile::tempdir().unwrap();
        Collection::init(tmp.path(), "demo").unwrap();
        fs::write(
            tmp.path().join("environments/dev.secrets.toml"),
            "[vars]\ntoken = \"s3cr3t\"\n",
        )
        .unwrap();
        let nested = tmp.path().join("requests/users");
        fs::create_dir_all(&nested).unwrap();

        let col = Collection::discover(&nested).unwrap();
        assert_eq!(col.root, tmp.path());
        assert_eq!(col.environments["dev"].vars["token"], "s3cr3t");
    }
}
