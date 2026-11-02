//! Formateo de salida: legible para humanos y JSON para máquinas.

use harpoon_core::{Collection, Scope};
use harpoon_http::{Outcome, Prepared, Response};
use serde::Serialize;

pub fn list(col: &Collection, scope: &Scope) {
    println!("{} ({})", col.meta.name, col.root.display());
    println!(
        "\nEntornos: {}",
        col.environments
            .keys()
            .cloned()
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!("\nSolicitudes:");
    for (key, req) in &col.requests {
        let url = scope
            .interpolate(&req.url)
            .unwrap_or_else(|_| req.url.clone());
        println!("  {key:<24} {:<7} {url}", req.method.to_string());
    }
}

pub fn send_human(prepared: &Prepared, res: &Response, outcomes: &[Outcome], verbose: bool) {
    if verbose {
        println!(
            "> {} {}{}",
            prepared.method,
            prepared.url,
            query_string(prepared)
        );
        for (k, v) in &prepared.headers {
            println!("> {k}: {}", redact(k, v));
        }
        if let Some(body) = &prepared.body {
            println!(">\n{body}");
        }
        println!();
    }
    println!(
        "< {} ({} ms, {})",
        res.status,
        res.elapsed_ms,
        human_size(res.size)
    );
    if verbose {
        for (k, v) in &res.headers {
            println!("< {k}: {v}");
        }
    }
    println!();
    print_body(res);
    print_outcomes(outcomes);
}

pub fn send_json(
    key: &str,
    prepared: &Prepared,
    res: &Response,
    outcomes: &[Outcome],
) -> anyhow::Result<()> {
    #[derive(Serialize)]
    struct Out<'a> {
        request: &'a str,
        sent: &'a Prepared,
        response: ResponseJson<'a>,
        assertions: &'a [Outcome],
        passed: bool,
    }
    let out = Out {
        request: key,
        sent: prepared,
        response: ResponseJson::from(res),
        assertions: outcomes,
        passed: outcomes.iter().all(|o| o.passed),
    };
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}

#[derive(Serialize)]
pub struct ResponseJson<'a> {
    status: u16,
    headers: &'a indexmap::IndexMap<String, String>,
    elapsed_ms: u128,
    size: usize,
    body: serde_json::Value,
}

impl<'a> From<&'a Response> for ResponseJson<'a> {
    fn from(res: &'a Response) -> Self {
        let body = res
            .json()
            .unwrap_or_else(|| serde_json::Value::String(res.text()));
        Self {
            status: res.status,
            headers: &res.headers,
            elapsed_ms: res.elapsed_ms,
            size: res.size,
            body,
        }
    }
}

#[derive(Serialize)]
pub struct Report<'a> {
    collection: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    environment: Option<&'a str>,
    items: Vec<RunItem>,
    total: usize,
    failed: usize,
}

impl<'a> Report<'a> {
    pub fn new(collection: &'a str, environment: Option<&'a str>) -> Self {
        Self {
            collection,
            environment,
            items: Vec::new(),
            total: 0,
            failed: 0,
        }
    }
    pub fn push(&mut self, item: RunItem) {
        self.total += 1;
        if !item.passed {
            self.failed += 1;
        }
        self.items.push(item);
    }
    pub fn all_passed(&self) -> bool {
        self.failed == 0
    }
}

#[derive(Serialize)]
pub struct RunItem {
    request: String,
    passed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    elapsed_ms: Option<u128>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    assertions: Vec<Outcome>,
    #[serde(skip)]
    pub(crate) line: String,
}

impl RunItem {
    pub fn done(key: &str, prepared: &Prepared, res: &Response, assertions: Vec<Outcome>) -> Self {
        let passed = assertions.iter().all(|o| o.passed);
        let line = format!(
            "{} {key:<24} {} {} → {} ({} ms)",
            mark(passed),
            prepared.method,
            prepared.url,
            res.status,
            res.elapsed_ms
        );
        Self {
            request: key.into(),
            passed,
            status: Some(res.status),
            elapsed_ms: Some(res.elapsed_ms),
            error: None,
            assertions,
            line,
        }
    }

    pub fn failed(key: &str, error: &str) -> Self {
        Self {
            request: key.into(),
            passed: false,
            status: None,
            elapsed_ms: None,
            error: Some(error.into()),
            assertions: Vec::new(),
            line: format!("{} {key:<24} {error}", mark(false)),
        }
    }
}

// Accesor usado por main para `--bail`.
impl RunItem {
    pub const fn passed(&self) -> bool {
        self.passed
    }
}

pub fn run_progress(item: &RunItem) {
    println!("{}", item.line);
    for o in &item.assertions {
        if !o.passed {
            println!("    {} {}{}", mark(false), o.name, detail(o));
        }
    }
}

pub fn run_summary(report: &Report<'_>) {
    println!();
    let ok = report.total - report.failed;
    if report.failed == 0 {
        println!("{} {ok}/{} solicitudes correctas", mark(true), report.total);
    } else {
        println!(
            "{} {}/{} solicitudes fallidas",
            mark(false),
            report.failed,
            report.total
        );
    }
}

fn print_body(res: &Response) {
    if res.body.is_empty() {
        return;
    }
    match res.json() {
        Some(v) if res.is_json() => {
            println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
        }
        _ => println!("{}", res.text()),
    }
}

fn print_outcomes(outcomes: &[Outcome]) {
    if outcomes.is_empty() {
        return;
    }
    println!();
    for o in outcomes {
        println!("{} {}{}", mark(o.passed), o.name, detail(o));
    }
}

fn detail(o: &Outcome) -> String {
    o.detail
        .as_ref()
        .map_or_else(String::new, |d| format!(" — {d}"))
}

fn mark(ok: bool) -> &'static str {
    if ok { "✓" } else { "✗" }
}

fn query_string(p: &Prepared) -> String {
    if p.query.is_empty() {
        return String::new();
    }
    let pairs: Vec<String> = p.query.iter().map(|(k, v)| format!("{k}={v}")).collect();
    format!("?{}", pairs.join("&"))
}

/// Oculta credenciales al imprimir la petición.
fn redact(name: &str, value: &str) -> String {
    let n = name.to_ascii_lowercase();
    if n == "authorization" || n.contains("key") || n.contains("token") || n == "cookie" {
        let (scheme, _) = value.split_once(' ').unwrap_or(("", value));
        if scheme.is_empty() {
            "••••••".into()
        } else {
            format!("{scheme} ••••••")
        }
    } else {
        value.into()
    }
}

#[allow(clippy::cast_precision_loss)]
fn human_size(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}
