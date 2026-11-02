use super::*;

fn req() -> ScriptRequest {
    ScriptRequest {
        method: Method::Get,
        url: "{{base_url}}/x".into(),
        headers: IndexMap::new(),
        query: IndexMap::new(),
        body: None,
    }
}

#[test]
fn pre_modifica_la_solicitud_y_las_variables() {
    let e = Engine::new().unwrap();
    let out = e
        .run(
            "pre",
            r#"
                harpoon.req.headers["X-Trace"] = "abc";
                harpoon.req.url = harpoon.req.url + "?ts=1";
                harpoon.vars.set("nonce", 42);
                console.log("hola", {a: 1});
                "#,
            Some(&req()),
            None,
            &IndexMap::new(),
            &IndexMap::new(),
        )
        .unwrap();
    let r = out.req.unwrap();
    assert_eq!(r.headers["X-Trace"], "abc");
    assert_eq!(r.url, "{{base_url}}/x?ts=1");
    assert_eq!(out.vars["nonce"], "42");
    assert_eq!(out.logs, vec![r#"hola {"a":1}"#]);
}

#[test]
fn post_lee_json_y_tests_estilo_chai() {
    let e = Engine::new().unwrap();
    let headers: IndexMap<String, String> =
        [("content-type".to_owned(), "application/json".to_owned())]
            .into_iter()
            .collect();
    let res = ScriptResponse {
        status: 201,
        headers: &headers,
        body: r#"{"id": 42, "tags": ["a","b"], "user": {"name": "Ada"}}"#.into(),
        elapsed_ms: 12,
        size: 50,
    };
    let out = e
            .run(
                "test",
                r#"
                harpoon.vars.set("id", harpoon.res.json().id);
                test("status 201", () => expect(harpoon.res.status).to.equal(201));
                test("tiene id", () => expect(harpoon.res.json()).to.have.property("id", 42));
                test("tags", () => expect(harpoon.res.json().tags).to.include("b").and.have.length(2));
                test("deep", () => expect(harpoon.res.json().user).to.deep.equal({name: "Ada"}));
                test("header", () => expect(harpoon.res.header("Content-Type")).to.contain("json"));
                test("falla", () => expect(harpoon.res.status).to.be.below(200));
                test("not", () => expect(harpoon.res.status).to.not.equal(500));
                test("tiempo", () => expect(harpoon.res.elapsed_ms).to.be.below(100));
                test("tipo", () => expect(harpoon.res.json().id).to.be.a("number"));
                test("array", () => expect(harpoon.res.json().tags).to.be.an("array").that.is.not.empty);
                test("oneOf", () => expect(harpoon.res.status).to.be.oneOf([200, 201]));
                "#,
                Some(&req()),
                Some(&res),
                &IndexMap::new(),
                &IndexMap::new(),
            )
            .unwrap();
    assert_eq!(out.vars["id"], "42");
    let passed: Vec<bool> = out.tests.iter().map(|t| t.passed).collect();
    assert_eq!(
        passed,
        vec![
            true, true, true, true, true, false, true, true, true, true, true
        ]
    );
    assert_eq!(
        out.tests[5].detail.as_deref(),
        Some("esperaba < 200, recibido 201")
    );
}

#[test]
fn error_de_sintaxis_y_timeout() {
    let e = Engine::new().unwrap();
    let err = e
        .run(
            "pre",
            "let x = ;",
            Some(&req()),
            None,
            &IndexMap::new(),
            &IndexMap::new(),
        )
        .unwrap_err();
    assert!(matches!(err, Error::Script { phase: "pre", .. }), "{err}");

    let err = e
        .run(
            "pre",
            "while(true){}",
            Some(&req()),
            None,
            &IndexMap::new(),
            &IndexMap::new(),
        )
        .unwrap_err();
    assert!(matches!(err, Error::Timeout(..)), "{err}");
}

#[test]
fn sin_acceso_a_io() {
    let e = Engine::new().unwrap();
    for src in [
        "fetch('http://x')",
        "require('fs')",
        "std.open('/etc/passwd')",
    ] {
        let err = e
            .run(
                "pre",
                src,
                Some(&req()),
                None,
                &IndexMap::new(),
                &IndexMap::new(),
            )
            .unwrap_err();
        assert!(matches!(err, Error::Script { .. }), "{src}: {err}");
    }
}
