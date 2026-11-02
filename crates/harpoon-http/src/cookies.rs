//! Almacén local de cookies compartido por los clientes de una colección.
use std::{
    collections::HashMap,
    fs,
    io::{BufReader, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, OnceLock, RwLock, Weak},
};

use reqwest::{Url, cookie::CookieStore as _, header::HeaderValue};
use serde::Serialize;

use crate::{Error, Result};

type SharedJars = Mutex<HashMap<PathBuf, Weak<PersistentCookies>>>;
static JARS: OnceLock<SharedJars> = OnceLock::new();

#[derive(Debug, Serialize)]
pub struct CookieInfo {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    pub secure: bool,
    pub http_only: bool,
    pub session: bool,
}

#[derive(Debug, Default)]
pub struct PersistentCookies {
    store: RwLock<cookie_store::CookieStore>,
    path: Option<PathBuf>,
}

impl PersistentCookies {
    /// Abre o reutiliza el almacén de una colección en el proceso actual.
    pub fn for_root(root: &Path) -> Result<Arc<Self>> {
        let root = fs::canonicalize(root).map_err(|e| Error::Cookies(e.to_string()))?;
        let path = root.join(".harpoon").join("cookies.json");
        let mut jars = JARS
            .get_or_init(|| Mutex::new(HashMap::new()))
            .lock()
            .map_err(|e| Error::Cookies(e.to_string()))?;
        if let Some(jar) = jars.get(&path).and_then(Weak::upgrade) {
            return Ok(jar);
        }
        let store = match fs::File::open(&path) {
            Ok(file) => cookie_store::serde::json::load(BufReader::new(file))
                .map_err(|e| Error::Cookies(format!("{}: {e}", path.display())))?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                cookie_store::CookieStore::default()
            }
            Err(e) => return Err(Error::Cookies(format!("{}: {e}", path.display()))),
        };
        let jar = Arc::new(Self {
            store: RwLock::new(store),
            path: Some(path.clone()),
        });
        jars.insert(path, Arc::downgrade(&jar));
        Ok(jar)
    }

    fn save(&self, store: &cookie_store::CookieStore) -> Result<()> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        let parent = path
            .parent()
            .ok_or_else(|| Error::Cookies("ruta inválida".into()))?;
        fs::create_dir_all(parent).map_err(|e| Error::Cookies(e.to_string()))?;
        let temp = parent.join(format!("cookies-{}.tmp", uuid::Uuid::new_v4()));
        let result = (|| -> Result<()> {
            let mut options = fs::OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            let mut file = options
                .open(&temp)
                .map_err(|e| Error::Cookies(e.to_string()))?;
            cookie_store::serde::json::save_incl_expired_and_nonpersistent(store, &mut file)
                .map_err(|e| Error::Cookies(e.to_string()))?;
            file.flush().map_err(|e| Error::Cookies(e.to_string()))?;
            fs::rename(&temp, path).map_err(|e| Error::Cookies(e.to_string()))?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(temp);
        }
        result
    }

    pub fn list(&self) -> Result<Vec<CookieInfo>> {
        let store = self
            .store
            .read()
            .map_err(|e| Error::Cookies(e.to_string()))?;
        Ok(store
            .iter_unexpired()
            .map(|c| CookieInfo {
                name: c.name().into(),
                value: c.value().into(),
                domain: c
                    .domain
                    .as_cow()
                    .map_or_else(String::new, std::borrow::Cow::into_owned),
                path: c.path.as_ref().into(),
                secure: c.secure().unwrap_or(false),
                http_only: c.http_only().unwrap_or(false),
                session: matches!(c.expires, cookie_store::CookieExpiration::SessionEnd),
            })
            .collect())
    }

    pub fn add(&self, url: &str, header: &str) -> Result<()> {
        let url = Url::parse(url).map_err(|e| Error::Cookies(e.to_string()))?;
        if !matches!(url.scheme(), "http" | "https") {
            return Err(Error::Cookies("la URL debe ser HTTP o HTTPS".into()));
        }
        let mut store = self
            .store
            .write()
            .map_err(|e| Error::Cookies(e.to_string()))?;
        store
            .parse(header, &url)
            .map_err(|e| Error::Cookies(e.to_string()))?;
        self.save(&store)
    }

    pub fn remove(&self, domain: &str, path: &str, name: &str) -> Result<()> {
        let mut store = self
            .store
            .write()
            .map_err(|e| Error::Cookies(e.to_string()))?;
        store.remove(domain, path, name);
        self.save(&store)
    }

    pub fn header_for_url(&self, url: &str) -> Result<Option<String>> {
        let url = Url::parse(url).map_err(|e| Error::Cookies(e.to_string()))?;
        Ok(self
            .cookies(&url)
            .and_then(|value| value.to_str().ok().map(str::to_owned)))
    }
}

impl reqwest::cookie::CookieStore for PersistentCookies {
    fn set_cookies(&self, headers: &mut dyn Iterator<Item = &HeaderValue>, url: &Url) {
        let Ok(mut store) = self.store.write() else {
            return;
        };
        for header in headers {
            if let Ok(value) = header.to_str() {
                let _ = store.parse(value, url);
            }
        }
        if let Err(error) = self.save(&store) {
            eprintln!("No se pudieron guardar las cookies: {error}");
        }
    }
    fn cookies(&self, url: &Url) -> Option<HeaderValue> {
        let store = self.store.read().ok()?;
        let value = store
            .get_request_values(url)
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join("; ");
        if value.is_empty() {
            None
        } else {
            HeaderValue::from_str(&value).ok()
        }
    }
}
