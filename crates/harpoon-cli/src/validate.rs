//! `harpoon validate`: revisa la colección sin enviar nada (pre-commit o CI).

use std::process::ExitCode;

use harpoon_core::Collection;
use harpoon_core::validate::{Severity, validate};

pub fn run(
    c: &Collection,
    env: Option<&str>,
    json: bool,
    strict: bool,
) -> anyhow::Result<ExitCode> {
    let issues = validate(c, env);
    let errors = issues
        .iter()
        .filter(|i| i.severity == Severity::Error)
        .count();
    let warnings = issues.len() - errors;
    if json {
        println!("{}", serde_json::to_string_pretty(&issues)?);
    } else if issues.is_empty() {
        println!(
            "✓ Colección válida: {} solicitudes revisadas.",
            c.requests.len()
        );
    } else {
        for issue in &issues {
            let mark = if issue.severity == Severity::Error {
                "✗"
            } else {
                "·"
            };
            let what = issue
                .request
                .as_deref()
                .map_or_else(|| issue.file.clone(), |r| format!("{r} ({})", issue.file));
            println!("{mark} {what} · {}: {}", issue.location, issue.message);
        }
        println!(
            "\n{errors} errores, {warnings} avisos en {} solicitudes.",
            c.requests.len()
        );
    }
    Ok(if errors > 0 || (strict && warnings > 0) {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    })
}
