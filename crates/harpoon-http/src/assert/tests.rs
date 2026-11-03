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
