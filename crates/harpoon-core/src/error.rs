use std::path::PathBuf;

/// Errores del núcleo.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("no se encontró `harpoon.toml` en {0} ni en sus directorios padre")]
    CollectionNotFound(PathBuf),

    #[error("error de E/S en {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("error al leer {path}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    #[error("error al serializar: {0}")]
    Serialize(#[from] toml::ser::Error),

    #[error("variable sin definir: `{0}`")]
    UndefinedVariable(String),

    #[error("entorno `{0}` no existe")]
    UnknownEnvironment(String),

    #[error("nombre no válido: `{0}`")]
    InvalidName(String),

    #[error("ya existe: {0}")]
    AlreadyExists(PathBuf),
}
