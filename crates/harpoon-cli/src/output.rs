//! Formateo de salida: legible para humanos y JSON para máquinas.

use harpoon_core::Collection;
use harpoon_http::{Outcome, Prepared, Response};
use harpoon_script::{Execution, TestResult};
use serde::Serialize;

pub fn list(col: &Collection, env: Option<&str>) -> harpoon_core::Result<()> {
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
        let scope = col
            .scope_for_request(env, key)?
            .with_layer(col.load_runtime_vars());
        let url = scope
            .interpolate(&req.url)
            .unwrap_or_else(|_| req.url.clone());
        println!("  {key:<24} {:<7} {url}", req.method.to_string());
    }
    Ok(())
}

pub fn send_human(x: &Execution, verbose: bool) {
    let (prepared, res) = (&x.prepared, &x.response);
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
        if let Some(parts) = &prepared.multipart {
            for part in parts {
                match part {
                    harpoon_core::model::MultipartPart::Text { name, value } => {
                        println!("> multipart {name}={value}");
                    }
                    harpoon_core::model::MultipartPart::File { name, path, .. } => {
                        println!("> multipart {name}=@{path}");
                    }
                }
            }
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
    print_checks(&x.assertions, &x.tests);
    print_logs(&x.logs);
}

pub fn send_json(key: &str, x: &Execution) -> anyhow::Result<()> {
    #[derive(Serialize)]
    struct Out<'a> {
        request: &'a str,
        sent: &'a Prepared,
        response: ResponseJson<'a>,
        assertions: &'a [Outcome],
        tests: &'a [TestResult],
        logs: &'a [String],
        passed: bool,
    }
    let out = Out {
        request: key,
        sent: &x.prepared,
        response: ResponseJson::from(&x.response),
        assertions: &x.assertions,
        tests: &x.tests,
        logs: &x.logs,
        passed: x.passed,
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
    /// Número de ejecución (iteración × fila de datos), empezando en 1.
    run: usize,
    passed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    elapsed_ms: Option<u128>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    assertions: Vec<Outcome>,
    tests: Vec<TestResult>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    logs: Vec<String>,
    #[serde(skip)]
    pub(crate) line: String,
}

impl RunItem {
    pub fn done(key: &str, run: usize, x: &Execution) -> Self {
        let line = format!(
            "{} {key:<24} {} {} → {} ({} ms)",
            mark(x.passed),
            x.prepared.method,
            x.prepared.url,
            x.response.status,
            x.response.elapsed_ms
        );
        Self {
            request: key.into(),
            run,
            passed: x.passed,
            status: Some(x.response.status),
            elapsed_ms: Some(x.response.elapsed_ms),
            error: None,
            assertions: x.assertions.clone(),
            tests: x.tests.clone(),
            logs: x.logs.clone(),
            line,
        }
    }

    pub fn failed(key: &str, run: usize, error: &str) -> Self {
        Self {
            request: key.into(),
            run,
            passed: false,
            status: None,
            elapsed_ms: None,
            error: Some(error.into()),
            assertions: Vec::new(),
            tests: Vec::new(),
            logs: Vec::new(),
            line: format!("{} {key:<24} {error}", mark(false)),
        }
    }

    pub const fn passed(&self) -> bool {
        self.passed
    }
}

pub fn run_header(
    run_no: usize,
    total: usize,
    iteration: usize,
    row: &indexmap::IndexMap<String, String>,
) {
    let data: Vec<String> = row.iter().map(|(k, v)| format!("{k}={v}")).collect();
    if data.is_empty() {
        println!("\n── ejecución {run_no}/{total} (iteración {iteration})");
    } else {
        println!("\n── ejecución {run_no}/{total} · {}", data.join(" "));
    }
}

/// Informe `JUnit` XML: un `testcase` por solicitud ejecutada.
#[allow(clippy::format_push_string, clippy::cast_precision_loss)]
pub fn junit(report: &Report<'_>) -> String {
    fn esc(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
    }
    let multi = report.items.iter().any(|i| i.run > 1);
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    let errors = report.items.iter().filter(|i| i.error.is_some()).count();
    let failures = report.failed - errors;
    let time: f64 = report
        .items
        .iter()
        .filter_map(|i| i.elapsed_ms)
        .map(|ms| ms as f64 / 1000.0)
        .sum();
    out.push_str(&format!(
        "<testsuites name=\"{}\" tests=\"{}\" failures=\"{failures}\" errors=\"{errors}\" time=\"{time:.3}\">\n",
        esc(report.collection),
        report.total
    ));
    out.push_str(&format!(
        "  <testsuite name=\"{}\" tests=\"{}\" failures=\"{failures}\" errors=\"{errors}\" time=\"{time:.3}\">\n",
        esc(report.collection),
        report.total
    ));
    for i in &report.items {
        let name = if multi {
            format!("{} (ejecución {})", i.request, i.run)
        } else {
            i.request.clone()
        };
        let t = i.elapsed_ms.map_or(0.0, |ms| ms as f64 / 1000.0);
        out.push_str(&format!(
            "    <testcase name=\"{}\" classname=\"{}\" time=\"{t:.3}\"",
            esc(&name),
            esc(report.collection)
        ));
        if let Some(err) = &i.error {
            out.push_str(&format!(
                ">\n      <error message=\"{}\"/>\n    </testcase>\n",
                esc(err)
            ));
        } else if !i.passed {
            let failed: Vec<String> = i
                .assertions
                .iter()
                .filter(|a| !a.passed)
                .map(|a| format!("{}{}", a.name, detail(a.detail.as_deref())))
                .chain(
                    i.tests
                        .iter()
                        .filter(|t| !t.passed)
                        .map(|t| format!("{}{}", t.name, detail(t.detail.as_deref()))),
                )
                .collect();
            out.push_str(&format!(
                ">\n      <failure message=\"{}\">{}</failure>\n    </testcase>\n",
                esc(&format!("{} comprobación(es) fallida(s)", failed.len())),
                esc(&failed.join("\n"))
            ));
        } else {
            out.push_str("/>\n");
        }
    }
    out.push_str("  </testsuite>\n</testsuites>\n");
    out
}

pub fn run_progress(item: &RunItem) {
    println!("{}", item.line);
    for o in &item.assertions {
        if !o.passed {
            println!(
                "    {} {}{}",
                mark(false),
                o.name,
                detail(o.detail.as_deref())
            );
        }
    }
    for t in &item.tests {
        if !t.passed {
            println!(
                "    {} {}{}",
                mark(false),
                t.name,
                detail(t.detail.as_deref())
            );
        }
    }
}

pub fn run_summary(report: &Report<'_>) {
    println!();
    let ok = report.total - report.failed;
    let tests: usize = report
        .items
        .iter()
        .map(|i| i.assertions.len() + i.tests.len())
        .sum();
    let failed_tests: usize = report
        .items
        .iter()
        .map(|i| {
            i.assertions.iter().filter(|o| !o.passed).count()
                + i.tests.iter().filter(|t| !t.passed).count()
        })
        .sum();
    if report.failed == 0 {
        println!(
            "{} {ok}/{} solicitudes correctas · {tests} comprobaciones",
            mark(true),
            report.total
        );
    } else {
        println!(
            "{} {}/{} solicitudes fallidas · {failed_tests}/{tests} comprobaciones fallidas",
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

fn print_checks(outcomes: &[Outcome], tests: &[TestResult]) {
    if outcomes.is_empty() && tests.is_empty() {
        return;
    }
    println!();
    for o in outcomes {
        println!(
            "{} {}{}",
            mark(o.passed),
            o.name,
            detail(o.detail.as_deref())
        );
    }
    for t in tests {
        println!(
            "{} {}{}",
            mark(t.passed),
            t.name,
            detail(t.detail.as_deref())
        );
    }
}

fn print_logs(logs: &[String]) {
    if logs.is_empty() {
        return;
    }
    println!();
    for l in logs {
        println!("  · {l}");
    }
}

fn detail(d: Option<&str>) -> String {
    d.map_or_else(String::new, |d| format!(" — {d}"))
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
