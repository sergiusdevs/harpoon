//! Intercambio de colecciones Harpoon como JSON local, sin estado ni secretos privados.
use std::{
    fs,
    path::{Component, Path, PathBuf},
};

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::{Body, Collection, Error, Result};

const FORMAT: &str = "harpoon-collection-v1";
const MAX_BYTES: u64 = 20 * 1024 * 1024;

#[derive(Debug, Serialize, Deserialize)]
pub struct SharedCollection {
    format: String,
    pub name: String,
    files: IndexMap<String, String>,
    #[serde(default)]
    folders: Vec<String>,
    pub warnings: Vec<String>,
}

fn io(path: &Path, source: std::io::Error) -> Error {
    Error::Io {
        path: path.to_path_buf(),
        source,
    }
}

/// Reúne solo archivos de colección; excluye estado local y secretos del entorno.
pub fn export(collection: &Collection) -> Result<SharedCollection> {
    let mut files = IndexMap::new();
    let mut folders = Vec::new();
    let mut warnings = Vec::new();
    let meta = collection.root.join("harpoon.toml");
    files.insert(
        "harpoon.toml".into(),
        fs::read_to_string(&meta).map_err(|e| io(&meta, e))?,
    );
    for directory in ["requests", "environments"] {
        let base = collection.root.join(directory);
        if !base.is_dir() {
            continue;
        }
        for entry in WalkDir::new(&base).follow_links(false).sort_by_file_name() {
            let entry = entry.map_err(|e| Error::Exchange(e.to_string()))?;
            let path = entry.path();
            if directory == "requests" && entry.file_type().is_dir() && path != base {
                let relative = path
                    .strip_prefix(&collection.root)
                    .map_err(|e| Error::Exchange(e.to_string()))?
                    .to_string_lossy()
                    .replace('\\', "/");
                folders.push(relative);
            }
            if !entry.file_type().is_file()
                || path.extension().is_none_or(|ext| ext != "toml")
                || path.to_string_lossy().ends_with(".secrets.toml")
            {
                continue;
            }
            let relative = path
                .strip_prefix(&collection.root)
                .map_err(|e| Error::Exchange(e.to_string()))?
                .to_string_lossy()
                .replace('\\', "/");
            files.insert(relative, fs::read_to_string(path).map_err(|e| io(path, e))?);
        }
    }
    for (key, request) in &collection.requests {
        match &request.body {
            Body::Binary { .. } => {
                warnings.push(format!("{key}: los archivos adjuntos no se incluyen"));
            }
            Body::Multipart { parts }
                if parts
                    .iter()
                    .any(|part| matches!(part, crate::model::MultipartPart::File { .. })) =>
            {
                warnings.push(format!("{key}: los archivos adjuntos no se incluyen"));
            }
            _ => {}
        }
    }
    if collection.meta.settings.ca_cert.is_some()
        || collection.meta.settings.client_cert.is_some()
        || collection.meta.settings.client_key.is_some()
    {
        warnings.push("Los certificados y claves de conexión no se incluyen".into());
    }
    let shared = SharedCollection {
        format: FORMAT.into(),
        name: collection.meta.name.clone(),
        files,
        folders,
        warnings,
    };
    if serde_json::to_vec(&shared)?.len() as u64 > MAX_BYTES {
        return Err(Error::Exchange(
            "colección demasiado grande para compartir (máximo 20 MB)".into(),
        ));
    }
    Ok(shared)
}

/// Lee y valida un archivo de intercambio sin modificar ninguna colección.
pub fn read(path: &Path) -> Result<SharedCollection> {
    if fs::metadata(path).map_err(|e| io(path, e))?.len() > MAX_BYTES {
        return Err(Error::Exchange(
            "archivo de intercambio demasiado grande (máximo 20 MB)".into(),
        ));
    }
    let data = fs::read(path).map_err(|e| io(path, e))?;
    let shared: SharedCollection = serde_json::from_slice(&data)?;
    validate(&shared)?;
    Ok(shared)
}

fn validate(shared: &SharedCollection) -> Result<()> {
    if shared.format != FORMAT || !shared.files.contains_key("harpoon.toml") {
        return Err(Error::Exchange(
            "formato de colección Harpoon no válido".into(),
        ));
    }
    for folder in &shared.folders {
        let safe = folder.starts_with("requests/")
            && !folder.contains('\\')
            && Path::new(folder)
                .components()
                .all(|c| matches!(c, Component::Normal(_)));
        if !safe {
            return Err(Error::Exchange(format!(
                "carpeta no permitida en la colección: {folder}"
            )));
        }
    }
    for key in shared.files.keys() {
        let path = Path::new(key);
        let components: Vec<_> = path.components().collect();
        let safe = !key.contains('\\')
            && components.iter().all(|c| matches!(c, Component::Normal(_)))
            && (key == "harpoon.toml"
                || ((key.starts_with("requests/") || key.starts_with("environments/"))
                    && path.extension().is_some_and(|ext| ext == "toml")
                    && !key.ends_with(".secrets.toml")));
        if !safe {
            return Err(Error::Exchange(format!(
                "ruta no permitida en la colección: {key}"
            )));
        }
    }
    Ok(())
}

/// Importa a un directorio nuevo, sin sobrescribir archivos existentes.
pub fn import(shared: &SharedCollection, target: &Path) -> Result<Collection> {
    validate(shared)?;
    fs::create_dir(target).map_err(|e| io(target, e))?;
    let result = (|| {
        for folder in &shared.folders {
            let path = target.join(folder);
            fs::create_dir_all(&path).map_err(|e| io(&path, e))?;
        }
        for (name, contents) in &shared.files {
            let path: PathBuf = target.join(name);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(|e| io(parent, e))?;
            }
            fs::write(&path, contents).map_err(|e| io(&path, e))?;
        }
        let ignore = target.join(".gitignore");
        fs::write(&ignore, "environments/*.secrets.toml\n.harpoon/\n")
            .map_err(|e| io(&ignore, e))?;
        Collection::load(target)
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(target);
    }
    result
}
