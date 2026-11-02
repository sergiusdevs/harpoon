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
            settings: crate::Settings::default(),
        };
        write_toml(&root.join(META_FILE), &meta)?;

        let mut dev = Environment::default();
        dev.vars
            .insert("base_url".into(), "http://localhost:8080".into());
        write_toml(&root.join(ENVIRONMENTS_DIR).join("dev.toml"), &dev)?;

        let mut req = Request::new("Health check", crate::Method::Get, "{{base_url}}/health");
        req.headers
            .insert("Accept".into(), "application/json".into());
        req.expect.status = Some(200);
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

    /// Guarda una solicitud en disco (`requests/<key>.toml`) y en memoria.
    pub fn save_request(&mut self, key: &str, request: Request) -> Result<()> {
        let path = self.root.join(REQUESTS_DIR).join(format!("{key}.toml"));
        write_toml(&path, &request)?;
        self.requests.insert(key.to_owned(), request);
        Ok(())
    }

    /// Elimina una solicitud del disco y de memoria. Borra las carpetas que
    /// queden vacías por el camino.
    pub fn delete_request(&mut self, key: &str) -> Result<()> {
        let key = validate_key(key)?;
        let path = self.request_path(key);
        remove_file(&path)?;
        prune_empty_dirs(&self.root.join(REQUESTS_DIR), path.parent());
        self.requests.shift_remove(key);
        Ok(())
    }

    /// Mueve/renombra una solicitud (`users/list` → `users/list-all`).
    pub fn rename_request(&mut self, from: &str, to: &str) -> Result<()> {
        let (from, to) = (validate_key(from)?, validate_key(to)?);
        if from == to {
            return Ok(());
        }
        let dst = self.request_path(to);
        if dst.exists() {
            return Err(Error::AlreadyExists(dst));
        }
        let src = self.request_path(from);
        rename(&src, &dst)?;
        prune_empty_dirs(&self.root.join(REQUESTS_DIR), src.parent());
        if let Some(req) = self.requests.shift_remove(from) {
            self.requests.insert(to.to_owned(), req);
        }
        self.sort_requests();
        Ok(())
    }

    /// Crea una carpeta (vacía) dentro de `requests/`.
    pub fn create_folder(&self, folder: &str) -> Result<()> {
        let folder = validate_key(folder)?;
        let path = self.root.join(REQUESTS_DIR).join(folder);
        fs::create_dir_all(&path).map_err(|source| Error::Io { path, source })
    }

    /// Renombra una carpeta y todas las solicitudes que contiene.
    pub fn rename_folder(&mut self, from: &str, to: &str) -> Result<()> {
        let (from, to) = (validate_key(from)?, validate_key(to)?);
        if from == to {
            return Ok(());
        }
        let base = self.root.join(REQUESTS_DIR);
        let dst = base.join(to);
        if dst.exists() {
            return Err(Error::AlreadyExists(dst));
        }
        rename(&base.join(from), &dst)?;
        prune_empty_dirs(&base, base.join(from).parent());
        let prefix = format!("{from}/");
        let moved: Vec<(String, Request)> = self
            .requests
            .iter()
            .filter(|(k, _)| k.starts_with(&prefix))
            .map(|(k, r)| (format!("{to}/{}", &k[prefix.len()..]), r.clone()))
            .collect();
        self.requests.retain(|k, _| !k.starts_with(&prefix));
        self.requests.extend(moved);
        self.sort_requests();
        Ok(())
    }

    /// Elimina una carpeta con todo su contenido.
    pub fn delete_folder(&mut self, folder: &str) -> Result<()> {
        let folder = validate_key(folder)?;
        let path = self.root.join(REQUESTS_DIR).join(folder);
        fs::remove_dir_all(&path).map_err(|source| Error::Io {
            path: path.clone(),
            source,
        })?;
        prune_empty_dirs(&self.root.join(REQUESTS_DIR), path.parent());
        let prefix = format!("{folder}/");
        self.requests.retain(|k, _| !k.starts_with(&prefix));
        Ok(())
    }

    /// Guarda un entorno. Las variables listadas en `secrets` van a
    /// `<nombre>.secrets.toml` (fuera de Git); el resto a `<nombre>.toml`.
    pub fn save_environment(&mut self, name: &str, env: Environment) -> Result<()> {
        let name = validate_key(name)?;
        if name.contains('/') {
            return Err(Error::InvalidName(name.to_owned()));
        }
        let dir = self.root.join(ENVIRONMENTS_DIR);
        let mut public = Environment {
            vars: IndexMap::new(),
            secrets: env.secrets.clone(),
        };
        let mut secret = Environment::default();
        for (k, v) in &env.vars {
            if env.secrets.contains(k) {
                secret.vars.insert(k.clone(), v.clone());
            } else {
                public.vars.insert(k.clone(), v.clone());
            }
        }
        write_toml(&dir.join(format!("{name}.toml")), &public)?;
        let secrets_path = dir.join(format!("{name}.secrets.toml"));
        if secret.vars.is_empty() {
            if secrets_path.exists() {
                remove_file(&secrets_path)?;
            }
        } else {
            write_toml(&secrets_path, &secret)?;
        }
        self.environments.insert(name.to_owned(), env);
        self.environments.sort_keys();
        Ok(())
    }

    /// Elimina un entorno (y su archivo de secretos si existe).
    pub fn delete_environment(&mut self, name: &str) -> Result<()> {
        let name = validate_key(name)?;
        let dir = self.root.join(ENVIRONMENTS_DIR);
        remove_file(&dir.join(format!("{name}.toml")))?;
        let secrets = dir.join(format!("{name}.secrets.toml"));
        if secrets.exists() {
            remove_file(&secrets)?;
        }
        self.environments.shift_remove(name);
        if self.meta.default_environment.as_deref() == Some(name) {
            self.meta.default_environment = None;
            self.save_meta(self.meta.clone())?;
        }
        Ok(())
    }

    /// Guarda `harpoon.toml`.
    pub fn save_meta(&mut self, meta: CollectionMeta) -> Result<()> {
        write_toml(&self.root.join(META_FILE), &meta)?;
        self.meta = meta;
        Ok(())
    }

    fn request_path(&self, key: &str) -> PathBuf {
        self.root.join(REQUESTS_DIR).join(format!("{key}.toml"))
    }

    fn sort_requests(&mut self) {
        self.requests.sort_keys();
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

/// Un identificador válido es una ruta relativa simple: sin `..`, sin
/// componentes vacíos, sin barra inicial y sin caracteres raros.
fn validate_key(key: &str) -> Result<&str> {
    let key = key.trim().trim_matches('/');
    let ok = !key.is_empty()
        && key.split('/').all(|part| {
            !part.is_empty()
                && part != "."
                && part != ".."
                && part
                    .chars()
                    .all(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | '.' | ' '))
        });
    if ok {
        Ok(key)
    } else {
        Err(Error::InvalidName(key.to_owned()))
    }
}

fn remove_file(path: &Path) -> Result<()> {
    fs::remove_file(path).map_err(|source| Error::Io {
        path: path.into(),
        source,
    })
}

fn rename(src: &Path, dst: &Path) -> Result<()> {
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent).map_err(|source| Error::Io {
            path: parent.into(),
            source,
        })?;
    }
    fs::rename(src, dst).map_err(|source| Error::Io {
        path: src.into(),
        source,
    })
}

/// Borra directorios vacíos desde `start` hacia arriba, sin pasar de `stop`.
fn prune_empty_dirs(stop: &Path, start: Option<&Path>) {
    let mut dir = start;
    while let Some(d) = dir {
        if d == stop || !d.starts_with(stop) {
            break;
        }
        if fs::remove_dir(d).is_err() {
            break; // no vacío (o ya no existe)
        }
        dir = d.parent();
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

    #[test]
    fn crear_renombrar_y_borrar_solicitudes_y_carpetas() {
        let tmp = tempfile::tempdir().unwrap();
        let mut col = Collection::init(tmp.path(), "demo").unwrap();

        let req = Request::new("Listar", crate::Method::Get, "{{base_url}}/users");
        col.save_request("users/list", req.clone()).unwrap();
        assert!(tmp.path().join("requests/users/list.toml").is_file());

        col.rename_request("users/list", "users/all").unwrap();
        assert!(!tmp.path().join("requests/users/list.toml").exists());
        assert!(col.requests.contains_key("users/all"));

        col.rename_folder("users", "people").unwrap();
        assert!(tmp.path().join("requests/people/all.toml").is_file());
        assert!(!tmp.path().join("requests/users").exists());
        assert!(col.requests.contains_key("people/all"));

        col.delete_request("people/all").unwrap();
        assert!(
            !tmp.path().join("requests/people").exists(),
            "carpeta vacía podada"
        );
        assert!(!col.requests.contains_key("people/all"));

        col.create_folder("admin/audit").unwrap();
        col.save_request("admin/audit/log", req).unwrap();
        col.delete_folder("admin").unwrap();
        assert!(!tmp.path().join("requests/admin").exists());
        assert!(col.requests.is_empty() || !col.requests.keys().any(|k| k.starts_with("admin/")));

        // Recargar desde disco coincide con memoria.
        let reloaded = Collection::load(tmp.path()).unwrap();
        assert_eq!(
            reloaded.requests.keys().collect::<Vec<_>>(),
            col.requests.keys().collect::<Vec<_>>()
        );
    }

    #[test]
    fn nombres_peligrosos_se_rechazan() {
        let tmp = tempfile::tempdir().unwrap();
        let mut col = Collection::init(tmp.path(), "demo").unwrap();
        assert!(matches!(
            col.delete_request("../x"),
            Err(Error::InvalidName(_))
        ));
        assert!(matches!(
            col.create_folder("etc/../../x"),
            Err(Error::InvalidName(_))
        ));
        assert!(matches!(
            col.rename_request("health", "a//b"),
            Err(Error::InvalidName(_))
        ));
        assert!(matches!(
            col.save_environment("dev/x", Environment::default()),
            Err(Error::InvalidName(_))
        ));
    }

    #[test]
    fn guardar_entorno_separa_secretos() {
        let tmp = tempfile::tempdir().unwrap();
        let mut col = Collection::init(tmp.path(), "demo").unwrap();
        let mut env = Environment::default();
        env.vars.insert("base_url".into(), "https://api".into());
        env.vars.insert("token".into(), "s3cr3t".into());
        env.secrets.push("token".into());
        col.save_environment("prod", env).unwrap();

        let public = fs::read_to_string(tmp.path().join("environments/prod.toml")).unwrap();
        assert!(public.contains("base_url"));
        assert!(
            !public.contains("s3cr3t"),
            "el secreto no va al archivo versionado"
        );
        let secret = fs::read_to_string(tmp.path().join("environments/prod.secrets.toml")).unwrap();
        assert!(secret.contains("s3cr3t"));

        let reloaded = Collection::load(tmp.path()).unwrap();
        assert_eq!(reloaded.environments["prod"].vars["token"], "s3cr3t");

        col.delete_environment("prod").unwrap();
        assert!(!tmp.path().join("environments/prod.secrets.toml").exists());
        assert!(!col.environments.contains_key("prod"));
    }

    #[test]
    fn borrar_entorno_predeterminado_limpia_meta() {
        let tmp = tempfile::tempdir().unwrap();
        let mut col = Collection::init(tmp.path(), "demo").unwrap();
        col.delete_environment("dev").unwrap();
        assert_eq!(col.meta.default_environment, None);
        let reloaded = Collection::load(tmp.path()).unwrap();
        assert_eq!(reloaded.meta.default_environment, None);
    }
}
