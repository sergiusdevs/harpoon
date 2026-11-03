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
/// Estado local (no versionado): variables de ejecución, historial.
pub const STATE_DIR: &str = ".harpoon";

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct RuntimeVarsFile {
    #[serde(default)]
    vars: IndexMap<String, String>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct FolderConfig {
    #[serde(default)]
    pub vars: IndexMap<String, String>,
    /// Variables de carpeta secretas (valor en `.harpoon/secrets.toml`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub secrets: Vec<String>,
    #[serde(flatten)]
    pub other: IndexMap<String, toml::Value>,
}

/// Una colección cargada en memoria.
#[derive(Debug, Clone)]
pub struct Collection {
    pub root: PathBuf,
    pub meta: CollectionMeta,
    /// Solicitudes indexadas por ruta relativa sin extensión (`users/list`).
    pub requests: IndexMap<String, Request>,
    pub environments: IndexMap<String, Environment>,
    pub folder_configs: IndexMap<String, FolderConfig>,
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
        let mut folder_configs: IndexMap<String, FolderConfig> = IndexMap::new();
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
                if path.file_name().is_some_and(|n| n == "folder.toml")
                    && path.parent() != Some(req_dir.as_path())
                {
                    let document: toml::Value = read_toml(path)?;
                    if document.get("method").is_some() && document.get("url").is_some() {
                        let key = path
                            .strip_prefix(&req_dir)
                            .unwrap_or(path)
                            .with_extension("")
                            .to_string_lossy()
                            .replace('\\', "/");
                        requests.insert(key, read_toml(path)?);
                        continue;
                    }
                    if let Some(folder) = path.parent().and_then(|p| p.strip_prefix(&req_dir).ok())
                    {
                        let folder = folder.to_string_lossy().replace('\\', "/");
                        folder_configs.insert(folder, read_toml(path)?);
                        continue;
                    }
                }
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
                    env.descriptions.extend(secrets.descriptions);
                }
                environments.insert(name, env);
            }
        }

        // Valores de los secretos de colección y carpetas (archivo local).
        let mut meta = meta;
        let local = read_local_secrets(root)?;
        for name in &meta.secrets {
            if let Some(value) = local.collection.get(name) {
                meta.vars.insert(name.clone(), value.clone());
            }
        }
        for (folder, config) in &mut folder_configs {
            if let Some(values) = local.folders.get(folder) {
                for name in &config.secrets {
                    if let Some(value) = values.get(name) {
                        config.vars.insert(name.clone(), value.clone());
                    }
                }
            }
        }

        Ok(Self {
            root: root.to_path_buf(),
            meta,
            requests,
            environments,
            folder_configs,
        })
    }

    /// Guarda en `.harpoon/secrets.toml` los valores secretos actuales de
    /// colección y carpetas (y asegura los `.gitignore` anidados).
    fn write_local_secrets(&self) -> Result<()> {
        let mut local = crate::secrets::LocalSecrets::default();
        for name in &self.meta.secrets {
            if let Some(value) = self.meta.vars.get(name).filter(|v| !v.is_empty()) {
                local.collection.insert(name.clone(), value.clone());
            }
        }
        for (folder, config) in &self.folder_configs {
            let values: IndexMap<String, String> = config
                .secrets
                .iter()
                .filter_map(|n| {
                    config
                        .vars
                        .get(n)
                        .filter(|v| !v.is_empty())
                        .map(|v| (n.clone(), v.clone()))
                })
                .collect();
            if !values.is_empty() {
                local.folders.insert(folder.clone(), values);
            }
        }
        let path = self.root.join(STATE_DIR).join(crate::secrets::LOCAL_FILE);
        if local.is_empty() {
            if path.exists() {
                remove_file(&path)?;
            }
        } else {
            write_toml(&path, &local)?;
        }
        crate::secrets::ensure_gitignores(&self.root)
    }

    /// Asegura los `.gitignore` anidados (llamar al abrir la colección).
    pub fn ensure_gitignores(&self) -> Result<()> {
        crate::secrets::ensure_gitignores(&self.root)
    }

    /// Secretos declarados que no tienen valor en esta máquina, ni en los
    /// archivos locales ni en `HARPOON_<NOMBRE>`.
    #[must_use]
    pub fn missing_secrets(&self, environment: Option<&str>) -> Vec<crate::secrets::MissingSecret> {
        use crate::secrets::{MissingSecret, env_var_name, from_process_env};
        let missing = |vars: &IndexMap<String, String>, name: &str| {
            vars.get(name).is_none_or(String::is_empty) && from_process_env(name).is_none()
        };
        let mut out = Vec::new();
        for name in &self.meta.secrets {
            if missing(&self.meta.vars, name) {
                out.push(MissingSecret {
                    scope: "collection",
                    owner: String::new(),
                    name: name.clone(),
                    env_var: env_var_name(name),
                });
            }
        }
        for (folder, config) in &self.folder_configs {
            for name in &config.secrets {
                if missing(&config.vars, name) {
                    out.push(MissingSecret {
                        scope: "folder",
                        owner: folder.clone(),
                        name: name.clone(),
                        env_var: env_var_name(name),
                    });
                }
            }
        }
        if let Some(env_name) = environment.or(self.meta.default_environment.as_deref())
            && let Some(env) = self.environments.get(env_name)
        {
            for name in &env.secrets {
                if missing(&env.vars, name) {
                    out.push(MissingSecret {
                        scope: "environment",
                        owner: env_name.to_owned(),
                        name: name.clone(),
                        env_var: env_var_name(name),
                    });
                }
            }
        }
        out
    }

    /// Crea en disco una colección vacía con un ejemplo.
    pub fn init(root: &Path, name: &str) -> Result<Self> {
        let meta = CollectionMeta {
            name: name.to_owned(),
            format: 1,
            default_environment: Some("dev".into()),
            vars: IndexMap::new(),
            secrets: Vec::new(),
            auth: crate::Auth::None,
            settings: crate::Settings::default(),
            scripts: crate::model::Scripts::default(),
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

        std::fs::create_dir_all(root.join(STATE_DIR)).map_err(|source| Error::Io {
            path: root.join(STATE_DIR),
            source,
        })?;
        crate::secrets::ensure_gitignores(root)?;
        let gitignore = root.join(".gitignore");
        if !gitignore.exists() {
            write_file(
                &gitignore,
                "# secretos y estado local de Harpoon\nenvironments/*.secrets.toml\n.harpoon/\n",
            )?;
        }

        Self::load(root)
    }

    /// Guarda una solicitud en disco (`requests/<key>.toml`) y en memoria.
    pub fn save_request(&mut self, key: &str, request: Request) -> Result<()> {
        let key = validate_key(key)?;
        if key.ends_with("/folder") && !self.requests.contains_key(key) {
            return Err(Error::InvalidName(key.to_owned()));
        }
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
        if to.ends_with("/folder") {
            return Err(Error::InvalidName(to.to_owned()));
        }
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
        let moved_configs: Vec<_> = self
            .folder_configs
            .iter()
            .filter(|(k, _)| *k == from || k.starts_with(&prefix))
            .map(|(k, v)| (format!("{to}{}", &k[from.len()..]), v.clone()))
            .collect();
        self.folder_configs
            .retain(|k, _| k != from && !k.starts_with(&prefix));
        self.folder_configs.extend(moved_configs);
        self.write_local_secrets()
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
        self.folder_configs
            .retain(|k, _| k != folder && !k.starts_with(&prefix));
        self.write_local_secrets()
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
            descriptions: IndexMap::new(),
            vars: IndexMap::new(),
            secrets: env.secrets.clone(),
        };
        let mut secret = Environment::default();
        for (k, v) in &env.vars {
            if env.secrets.contains(k) {
                secret.vars.insert(k.clone(), v.clone());
                if let Some(description) = env.descriptions.get(k) {
                    secret.descriptions.insert(k.clone(), description.clone());
                }
            } else {
                public.vars.insert(k.clone(), v.clone());
                if let Some(description) = env.descriptions.get(k) {
                    public.descriptions.insert(k.clone(), description.clone());
                }
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
        crate::secrets::ensure_gitignores(&self.root)
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
    /// Guarda `harpoon.toml`. Las variables listadas en `meta.secrets` no se
    /// escriben ahí sino en `.harpoon/secrets.toml`.
    pub fn save_meta(&mut self, meta: CollectionMeta) -> Result<()> {
        let mut public = meta.clone();
        public.vars.retain(|k, _| !meta.secrets.contains(k));
        write_toml(&self.root.join(META_FILE), &public)?;
        self.meta = meta;
        self.write_local_secrets()
    }

    /// Variables de ejecución (las que fijan los scripts con `harpoon.vars.set`).
    /// Viven en `.harpoon/vars.toml`, fuera de Git, y sobreviven entre ejecuciones.
    #[must_use]
    pub fn load_runtime_vars(&self) -> IndexMap<String, String> {
        let path = self.root.join(STATE_DIR).join("vars.toml");
        if !path.is_file() {
            return IndexMap::new();
        }
        read_toml::<RuntimeVarsFile>(&path)
            .map(|f| f.vars)
            .unwrap_or_default()
    }

    pub fn save_runtime_vars(&self, vars: &IndexMap<String, String>) -> Result<()> {
        let path = self.root.join(STATE_DIR).join("vars.toml");
        write_toml(&path, &RuntimeVarsFile { vars: vars.clone() })?;
        crate::secrets::ensure_gitignores(&self.root)
    }

    fn request_path(&self, key: &str) -> PathBuf {
        self.root.join(REQUESTS_DIR).join(format!("{key}.toml"))
    }

    fn sort_requests(&mut self) {
        self.requests.sort_keys();
    }

    /// Guarda variables de una carpeta sin eliminar otras secciones de su configuración.
    pub fn save_folder_vars(&mut self, folder: &str, vars: IndexMap<String, String>) -> Result<()> {
        let secrets = self
            .folder_configs
            .get(folder)
            .map(|c| c.secrets.clone())
            .unwrap_or_default();
        self.save_folder_config(folder, vars, secrets)
    }

    /// Guarda las variables de una carpeta; las de `secrets` van al archivo local.
    pub fn save_folder_config(
        &mut self,
        folder: &str,
        vars: IndexMap<String, String>,
        secrets: Vec<String>,
    ) -> Result<()> {
        let folder = validate_key(folder)?;
        let dir = self.root.join(REQUESTS_DIR).join(folder);
        if !dir.is_dir() {
            return Err(Error::Io {
                path: dir,
                source: std::io::Error::new(std::io::ErrorKind::NotFound, "la carpeta no existe"),
            });
        }
        let config_path = dir.join("folder.toml");
        if config_path.exists() && !self.folder_configs.contains_key(folder) {
            return Err(Error::AlreadyExists(config_path));
        }
        let mut config = self.folder_configs.get(folder).cloned().unwrap_or_default();
        config.vars = vars;
        config.secrets = secrets;
        let mut public = config.clone();
        public.vars.retain(|k, _| !config.secrets.contains(k));
        write_toml(&dir.join("folder.toml"), &public)?;
        self.folder_configs.insert(folder.to_owned(), config);
        self.write_local_secrets()
    }

    /// Construye el ámbito para una solicitud, con las carpetas entre colección y entorno.
    pub fn scope_for_request(&self, environment: Option<&str>, key: &str) -> Result<Scope> {
        use crate::secrets::with_env_fallback;
        let mut scope =
            Scope::new().with_layer(with_env_fallback(&self.meta.vars, &self.meta.secrets));
        let mut folder = String::new();
        let mut parts = key.split('/').peekable();
        while let Some(part) = parts.next() {
            if parts.peek().is_none() {
                break;
            }
            if !folder.is_empty() {
                folder.push('/');
            }
            folder.push_str(part);
            if let Some(config) = self.folder_configs.get(&folder) {
                scope = scope.with_layer(with_env_fallback(&config.vars, &config.secrets));
            }
        }
        let env_name = environment.or(self.meta.default_environment.as_deref());
        if let Some(name) = env_name {
            let env = self
                .environments
                .get(name)
                .ok_or_else(|| Error::UnknownEnvironment(name.to_owned()))?;
            scope = scope.with_layer(with_env_fallback(&env.vars, &env.secrets));
        }
        Ok(scope)
    }

    /// Construye el ámbito de variables para un entorno (o el predeterminado).
    pub fn scope(&self, environment: Option<&str>) -> Result<Scope> {
        use crate::secrets::with_env_fallback;
        let mut scope =
            Scope::new().with_layer(with_env_fallback(&self.meta.vars, &self.meta.secrets));
        let env_name = environment.or(self.meta.default_environment.as_deref());
        if let Some(name) = env_name {
            let env = self
                .environments
                .get(name)
                .ok_or_else(|| Error::UnknownEnvironment(name.to_owned()))?;
            scope = scope.with_layer(with_env_fallback(&env.vars, &env.secrets));
        }
        Ok(scope)
    }
}

fn read_local_secrets(root: &Path) -> Result<crate::secrets::LocalSecrets> {
    let path = root.join(STATE_DIR).join(crate::secrets::LOCAL_FILE);
    if path.is_file() {
        read_toml(&path)
    } else {
        Ok(crate::secrets::LocalSecrets::default())
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
mod tests;
