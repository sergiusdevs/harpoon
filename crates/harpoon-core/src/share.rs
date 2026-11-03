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
    let mocks = collection.root.join("mocks.toml");
    if fs::symlink_metadata(&mocks).is_ok_and(|metadata| metadata.file_type().is_file()) {
        files.insert(
            "mocks.toml".into(),
            fs::read_to_string(&mocks).map_err(|e| io(&mocks, e))?,
        );
        warnings.push("Las respuestas mock incluidas pueden contener datos sensibles".into());
    }
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
    warnings.extend(crate::scan::export_warnings(collection));
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
    let mut shared: SharedCollection = serde_json::from_slice(&data)?;
    validate(&shared)?;
    // Los avisos los calcula Harpoon: los que trae el archivo los escribió
    // quien lo exportó y no son de fiar.
    shared.warnings = import_warnings(&shared);
    Ok(shared)
}

/// Riesgos de abrir una colección ajena: scripts que se ejecutan al enviar
/// (con acceso a las variables del entorno) y TLS sin verificar.
fn import_warnings(shared: &SharedCollection) -> Vec<String> {
    let parse = |text: &str| text.parse::<toml::Table>().ok();
    let has_scripts = |table: &toml::Table| {
        table
            .get("scripts")
            .and_then(toml::Value::as_table)
            .is_some_and(|scripts| {
                scripts
                    .values()
                    .any(|v| v.as_str().is_some_and(|s| !s.trim().is_empty()))
            })
    };
    let mut warnings = Vec::new();
    let scripted: Vec<&str> = shared
        .files
        .iter()
        .filter(|(key, _)| *key == "harpoon.toml" || key.starts_with("requests/"))
        .filter(|(_, text)| parse(text).is_some_and(|t| has_scripts(&t)))
        .map(|(key, _)| key.as_str())
        .collect();
    if !scripted.is_empty() {
        let list = scripted
            .iter()
            .take(5)
            .copied()
            .collect::<Vec<_>>()
            .join(", ");
        let more = if scripted.len() > 5 {
            format!(" y {} más", scripted.len() - 5)
        } else {
            String::new()
        };
        warnings.push(format!(
            "Incluye scripts que se ejecutarán al enviar y pueden leer tus variables y secretos ({list}{more}). Revísalos si la colección no es de confianza."
        ));
    }
    let insecure = shared
        .files
        .get("harpoon.toml")
        .and_then(|text| parse(text))
        .and_then(|t| t.get("settings")?.get("insecure")?.as_bool())
        .unwrap_or(false);
    if insecure {
        warnings.push("Desactiva la verificación de certificados TLS (insecure = true). Cámbialo en Ajustes si no lo necesitas.".into());
    }
    if shared.files.contains_key("mocks.toml") {
        warnings.push("Incluye respuestas mock.".into());
    }
    warnings
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
                || key == "mocks.toml"
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
