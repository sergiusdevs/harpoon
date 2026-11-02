//! Evaluación de las aserciones declarativas `[expect]`.

use harpoon_core::model::Expect;
use serde::Serialize;

use crate::Response;

/// Una aserción concreta, legible por humanos.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Assertion {
    pub name: String,
}

/// Resultado de evaluar una aserción.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Outcome {
    pub name: String,
    pub passed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl Outcome {
    fn ok(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            passed: true,
            detail: None,
        }
    }
    fn fail(name: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            passed: false,
            detail: Some(detail.into()),
        }
    }
}

#[must_use]
pub fn list(expect: &Expect) -> Vec<Assertion> {
    let mut out = Vec::new();
    if let Some(s) = expect.status {
        out.push(Assertion {
            name: format!("status == {s}"),
        });
    }
    for (k, v) in &expect.headers {
        out.push(Assertion {
            name: format!("header {k} == {v:?}"),
        });
    }
    for frag in &expect.body_contains {
        out.push(Assertion {
            name: format!("body contains {frag:?}"),
        });
    }
    if let Some(ms) = expect.max_time_ms {
        out.push(Assertion {
            name: format!("time <= {ms}ms"),
        });
    }
    out
}

#[must_use]
pub fn evaluate(expect: &Expect, res: &Response) -> Vec<Outcome> {
    let mut out = Vec::new();
    if let Some(want) = expect.status {
        let name = format!("status == {want}");
        out.push(if res.status == want {
            Outcome::ok(name)
        } else {
            Outcome::fail(name, format!("recibido {}", res.status))
        });
    }
    for (k, want) in &expect.headers {
        let name = format!("header {k} == {want:?}");
        out.push(match res.header(k) {
            Some(got) if got == want => Outcome::ok(name),
            Some(got) => Outcome::fail(name, format!("recibido {got:?}")),
            None => Outcome::fail(name, "cabecera ausente"),
        });
    }
    if !expect.body_contains.is_empty() {
        let text = res.text();
        for frag in &expect.body_contains {
            let name = format!("body contains {frag:?}");
            out.push(if text.contains(frag.as_str()) {
                Outcome::ok(name)
            } else {
                Outcome::fail(name, "no aparece en el cuerpo")
            });
        }
    }
    if let Some(max) = expect.max_time_ms {
        let name = format!("time <= {max}ms");
        out.push(if res.elapsed_ms <= u128::from(max) {
            Outcome::ok(name)
        } else {
            Outcome::fail(name, format!("tardó {}ms", res.elapsed_ms))
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::IndexMap;

    fn res(status: u16, body: &str) -> Response {
        let mut headers = IndexMap::new();
        headers.insert("content-type".into(), "application/json".into());
        Response {
            status,
            headers,
            body: body.as_bytes().to_vec(),
            elapsed_ms: 12,
            size: body.len(),
        }
    }

    #[test]
    fn evalua_todas_las_aserciones() {
        let mut expect = Expect {
            status: Some(200),
            ..Default::default()
        };
        expect
            .headers
            .insert("Content-Type".into(), "application/json".into());
        expect.body_contains.push("\"ok\"".into());
        expect.max_time_ms = Some(10);

        let outcomes = evaluate(&expect, &res(200, r#"{"ok":true}"#));
        let passed: Vec<bool> = outcomes.iter().map(|o| o.passed).collect();
        assert_eq!(passed, vec![true, true, true, false]);
        assert_eq!(outcomes[3].detail.as_deref(), Some("tardó 12ms"));
    }

    #[test]
    fn status_incorrecto() {
        let expect = Expect {
            status: Some(201),
            ..Default::default()
        };
        let o = evaluate(&expect, &res(500, ""));
        assert!(!o[0].passed);
        assert_eq!(o[0].detail.as_deref(), Some("recibido 500"));
    }
}
