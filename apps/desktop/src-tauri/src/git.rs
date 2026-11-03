//! Estado de Git de la colección: si está versionada y en qué rama.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct GitStatus {
    /// `git` está instalado y se puede ejecutar.
    pub available: bool,
    /// La colección está dentro de un repositorio (propio o de una carpeta superior).
    pub initialized: bool,
    /// Rama actual; `None` con HEAD separado.
    pub branch: Option<String>,
}

fn git(root: &Path, args: &[&str]) -> std::io::Result<std::process::Output> {
    Command::new("git").arg("-C").arg(root).args(args).output()
}

fn status(root: &Path) -> GitStatus {
    let Ok(inside) = git(root, &["rev-parse", "--is-inside-work-tree"]) else {
        return GitStatus::default();
    };
    let initialized =
        inside.status.success() && String::from_utf8_lossy(&inside.stdout).trim() == "true";
    // `symbolic-ref` también funciona en un repositorio sin commits todavía.
    let branch = initialized
        .then(|| git(root, &["symbolic-ref", "--short", "-q", "HEAD"]).ok())
        .flatten()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
        .filter(|b| !b.is_empty());
    GitStatus {
        available: true,
        initialized,
        branch,
    }
}

#[tauri::command]
pub fn git_status(root: PathBuf) -> GitStatus {
    status(&root)
}

/// Crea un repositorio en la raíz de la colección (los `.gitignore` de
/// secretos y estado local ya existen al abrirla).
#[tauri::command]
pub fn git_init(root: PathBuf) -> Result<GitStatus, String> {
    let current = status(&root);
    if !current.available {
        return Err("Git no está instalado o no está en el PATH".into());
    }
    if current.initialized {
        return Ok(current);
    }
    let out = git(&root, &["init"]).map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(format!(
            "git init falló: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(status(&root))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detecta_e_inicializa_un_repositorio() {
        let tmp = tempfile::tempdir().unwrap();
        let before = status(tmp.path());
        if !before.available {
            return; // sin git en el entorno de pruebas
        }
        assert!(!before.initialized);
        let after = git_init(tmp.path().to_path_buf()).unwrap();
        assert!(after.initialized);
        assert!(after.branch.is_some());
    }
}
