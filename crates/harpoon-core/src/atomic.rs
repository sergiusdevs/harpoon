//! Escritura atómica: el archivo queda con el contenido anterior o con el
//! nuevo completo, nunca a medias (corte de luz, disco lleno, cierre forzado).

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Escribe `contents` en `path` a través de un temporal en la misma carpeta
/// que después se renombra encima. Conserva los permisos del archivo previo
/// y, si `path` es un enlace simbólico, escribe en su destino.
pub fn write(path: impl AsRef<Path>, contents: impl AsRef<[u8]>) -> io::Result<()> {
    let target = resolve(path.as_ref());
    let dir = target
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let name = target
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "ruta sin nombre de archivo"))?;
    let temp = dir.join(format!(
        ".{}.{}.tmp",
        name.to_string_lossy(),
        uuid::Uuid::new_v4().simple()
    ));

    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?;
        file.write_all(contents.as_ref())?;
        if let Ok(previous) = fs::metadata(&target) {
            file.set_permissions(previous.permissions())?;
        }
        file.sync_all()?;
        drop(file);
        fs::rename(&temp, &target)?;
        sync_dir(dir);
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

fn resolve(path: &Path) -> PathBuf {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => {
            fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
        }
        _ => path.to_path_buf(),
    }
}

/// Persiste la entrada del directorio tras el renombrado (solo Unix; en
/// Windows no se pueden abrir directorios así y el renombrado ya es durable).
fn sync_dir(dir: &Path) {
    #[cfg(unix)]
    if let Ok(d) = fs::File::open(dir) {
        let _ = d.sync_all();
    }
    #[cfg(not(unix))]
    let _ = dir;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn leftovers(dir: &Path) -> Vec<String> {
        fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|n| Path::new(n).extension().is_some_and(|e| e == "tmp"))
            .collect()
    }

    #[test]
    fn crea_y_sustituye_sin_dejar_temporales() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.toml");
        write(&path, "uno").unwrap();
        write(&path, "dos").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "dos");
        assert_eq!(leftovers(dir.path()), Vec::<String>::new());
    }

    #[test]
    fn si_falla_conserva_el_original_y_limpia() {
        let dir = tempfile::tempdir().unwrap();
        // Un directorio en el destino hace fallar el renombrado.
        let path = dir.path().join("ocupado");
        fs::create_dir(&path).unwrap();
        fs::write(path.join("dentro"), "x").unwrap();
        assert!(write(&path, "nuevo").is_err());
        assert!(path.is_dir());
        assert_eq!(leftovers(dir.path()), Vec::<String>::new());
    }

    #[cfg(unix)]
    #[test]
    fn conserva_permisos_y_sigue_enlaces() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let real = dir.path().join("secrets.toml");
        fs::write(&real, "a").unwrap();
        fs::set_permissions(&real, fs::Permissions::from_mode(0o600)).unwrap();
        let link = dir.path().join("enlace.toml");
        std::os::unix::fs::symlink(&real, &link).unwrap();

        write(&link, "b").unwrap();
        assert!(
            fs::symlink_metadata(&link)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(fs::read_to_string(&real).unwrap(), "b");
        assert_eq!(
            fs::metadata(&real).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
}
