//! `harpoon check`: comprueba que la colección no mete secretos en Git.
//!
//! Pensado para un hook de pre-commit o para CI: sale con código 1 si hay
//! algo que corregir.
//!
//! 1. Posibles secretos escritos tal cual en archivos versionados.
//! 2. Archivos de secretos o estado local que Git no ignora o ya sigue.
//! 3. Claves privadas de certificados dentro de la colección sin ignorar.
//! 4. (Solo informativo) secretos declarados sin valor en esta máquina.

use std::path::Path;
use std::process::{Command, ExitCode};

use harpoon_core::{Collection, scan};
use serde::Serialize;

#[derive(Serialize)]
struct Report {
    findings: Vec<scan::Finding>,
    /// Archivos con secretos que Git subiría (no ignorados o ya seguidos).
    exposed_files: Vec<String>,
    missing_secrets: Vec<harpoon_core::secrets::MissingSecret>,
    /// `false` si la colección no está en un repositorio Git (no se comprueba el punto 2).
    git: bool,
}

fn git(root: &Path, args: &[&str]) -> Option<std::process::Output> {
    Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .ok()
}

/// Archivos locales que nunca deberían ir a Git.
fn sensitive_files(c: &Collection) -> Vec<String> {
    let mut out = Vec::new();
    let envs = c.root.join(harpoon_core::collection::ENVIRONMENTS_DIR);
    if let Ok(entries) = std::fs::read_dir(&envs) {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.ends_with(".secrets.toml") {
                out.push(format!("environments/{name}"));
            }
        }
    }
    let state = c.root.join(harpoon_core::collection::STATE_DIR);
    if state.is_dir() {
        for entry in walk(&state) {
            if let Ok(rel) = entry.strip_prefix(&c.root) {
                out.push(rel.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    for key in [&c.meta.settings.client_key, &c.meta.settings.client_cert]
        .into_iter()
        .flatten()
    {
        let path = c.root.join(key);
        if path.is_file() && path.starts_with(&c.root) {
            out.push(key.replace('\\', "/"));
        }
    }
    out.sort();
    out.dedup();
    out
}

fn walk(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for e in entries.flatten() {
            let path = e.path();
            if path.is_dir() {
                out.extend(walk(&path));
            } else if path.file_name().is_some_and(|n| n != ".gitignore") {
                out.push(path);
            }
        }
    }
    out
}

/// De los archivos sensibles, los que Git subiría: seguidos o no ignorados.
fn exposed(root: &Path, files: &[String]) -> Option<Vec<String>> {
    let inside = git(root, &["rev-parse", "--is-inside-work-tree"])?;
    if !inside.status.success() {
        return None;
    }
    let mut out = Vec::new();
    for file in files {
        let tracked = git(root, &["ls-files", "--error-unmatch", "--", file])
            .is_some_and(|o| o.status.success());
        let ignored =
            git(root, &["check-ignore", "-q", "--", file]).is_some_and(|o| o.status.success());
        if tracked {
            out.push(format!(
                "{file} (ya está en Git: quítalo con `git rm --cached {file}`)"
            ));
        } else if !ignored {
            out.push(format!("{file} (no está ignorado)"));
        }
    }
    Some(out)
}

pub fn run(c: &Collection, env: Option<&str>, json: bool) -> anyhow::Result<ExitCode> {
    let findings = scan::scan_collection(c);
    let files = sensitive_files(c);
    let exposed_files = exposed(&c.root, &files);
    let report = Report {
        findings,
        git: exposed_files.is_some(),
        exposed_files: exposed_files.unwrap_or_default(),
        missing_secrets: c.missing_secrets(env),
    };
    let failed = !report.findings.is_empty() || !report.exposed_files.is_empty();
    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print_report(&report);
    }
    Ok(if failed {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    })
}

fn print_report(r: &Report) {
    if r.findings.is_empty() && r.exposed_files.is_empty() {
        println!("✓ Sin secretos en los archivos que van a Git.");
    }
    if !r.findings.is_empty() {
        println!(
            "✗ Posibles secretos escritos tal cual ({}):",
            r.findings.len()
        );
        for f in &r.findings {
            println!("  {} · {} · {} ({})", f.file, f.location, f.kind, f.preview);
            println!(
                "      → usa {{{{{}}}}} y márcala como secreta",
                f.suggestion
            );
        }
    }
    if !r.exposed_files.is_empty() {
        println!("✗ Archivos con secretos que Git subiría:");
        for f in &r.exposed_files {
            println!("  {f}");
        }
    }
    if !r.git {
        println!("· No es un repositorio Git: no se ha comprobado qué archivos ignora.");
    }
    if !r.missing_secrets.is_empty() {
        println!("· Secretos sin valor en esta máquina (no bloquea):");
        for m in &r.missing_secrets {
            let owner = if m.owner.is_empty() {
                String::new()
            } else {
                format!(" {}", m.owner)
            };
            println!("  {}{owner}: {} (o define {})", m.scope, m.name, m.env_var);
        }
    }
}
