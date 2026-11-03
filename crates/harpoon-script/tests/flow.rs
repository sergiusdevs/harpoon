//! Flujo típico: `login` guarda el token con un script `post`, y la
//! siguiente solicitud lo usa vía `{{token}}` sin configuración extra.

use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
};

use harpoon_core::{Auth, Body, Collection, Method, Request};
use harpoon_http::Client;

fn spawn_server() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = stream.unwrap();
            let mut buf = vec![0u8; 65536];
            let n = stream.read(&mut buf).unwrap();
            let raw = String::from_utf8_lossy(&buf[..n]).into_owned();
            let head = raw.split("\r\n\r\n").next().unwrap_or("");
            let path = head
                .lines()
                .next()
                .and_then(|l| l.split_whitespace().nth(1))
                .unwrap_or("/")
                .to_owned();
            let auth = head
                .lines()
                .find_map(|l| l.strip_prefix("authorization: "))
                .unwrap_or("")
                .to_owned();
            let (status, body) = if path == "/login" {
                ("200", r#"{"token":"tok-123","user":{"id":7}}"#.to_owned())
            } else if auth == "Bearer tok-123" {
                ("200", r#"{"ok":true}"#.to_owned())
            } else {
                ("401", r#"{"error":"unauthorized"}"#.to_owned())
            };
            let res = format!(
                "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(res.as_bytes()).unwrap();
        }
    });
    format!("http://{addr}")
}

#[tokio::test]
async fn login_guarda_token_y_la_siguiente_lo_usa() {
    let base = spawn_server();
    let tmp = tempfile::tempdir().unwrap();
    let mut col = Collection::init(tmp.path(), "flow").unwrap();
    col.meta.vars.insert("base_url".into(), base);
    col.meta.default_environment = None;
    col.meta.auth = Auth::Bearer {
        token: "{{token}}".into(),
    };
    col.meta.scripts.test = "test('siempre json', () => expect(harpoon.res.header('content-type')).to.include('json'));".into();

    let mut login = Request::new("login", Method::Post, "{{base_url}}/login");
    login.auth = Auth::None;
    login.body = Body::Json {
        json: r#"{"user":"ada"}"#.into(),
    };
    login.scripts.post = r#"
        const b = harpoon.res.json();
        harpoon.vars.set("token", b.token);
        harpoon.vars.set("user_id", b.user.id);
        console.log("token guardado");
    "#
    .into();
    login.scripts.test = "test('200', () => expect(harpoon.res.status).to.equal(200));".into();

    let mut me = Request::new("me", Method::Get, "{{base_url}}/me/{{user_id}}");
    me.scripts.pre = r#"harpoon.req.headers["X-From-Pre"] = "1";"#.into();
    me.expect.status = Some(200);
    me.scripts.test = r#"
        test('ok', () => expect(harpoon.res.json().ok).to.be.true);
        test('pre aplicado', () => expect(harpoon.req.headers["X-From-Pre"]).to.equal("1"));
    "#
    .into();

    let client = Client::for_collection(&col).unwrap();

    // Sin token aún: la interpolación de {{token}} falla antes de enviar.
    let err = harpoon_script::runner::execute_persistent(&client, &col, &me, None)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("user_id"), "{err}");

    let exec = harpoon_script::runner::execute_persistent(&client, &col, &login, None)
        .await
        .unwrap();
    assert!(exec.passed, "{:?}", exec.tests);
    assert_eq!(exec.vars["token"], "tok-123");
    assert_eq!(exec.vars["user_id"], "7");
    assert_eq!(exec.logs, vec!["token guardado"]);
    assert_eq!(exec.tests.len(), 2, "test de colección + test de solicitud");
    assert!(tmp.path().join(".harpoon/vars.toml").is_file());

    let exec = harpoon_script::runner::execute_persistent(&client, &col, &me, None)
        .await
        .unwrap();
    assert_eq!(exec.response.status, 200);
    assert_eq!(
        exec.prepared.url,
        format!("{}/me/7", col.meta.vars["base_url"])
    );
    assert_eq!(exec.prepared.headers["Authorization"], "Bearer tok-123");
    assert!(exec.passed, "{:?} {:?}", exec.assertions, exec.tests);
    assert_eq!(exec.tests.len(), 3);
}

#[tokio::test]
async fn script_post_roto_no_impide_la_respuesta() {
    let base = spawn_server();
    let tmp = tempfile::tempdir().unwrap();
    let mut col = Collection::init(tmp.path(), "flow").unwrap();
    col.meta.vars.insert("base_url".into(), base);
    col.meta.default_environment = None;
    let mut req = Request::new("login", Method::Post, "{{base_url}}/login");
    req.scripts.post = "harpoon.res.json().nope.deeper".into();
    let client = Client::for_collection(&col).unwrap();
    let exec = harpoon_script::execute(&client, &col, &req, None, indexmap::IndexMap::default())
        .await
        .unwrap();
    assert_eq!(exec.response.status, 200);
    assert!(!exec.passed);
    assert_eq!(exec.tests.len(), 1);
    assert!(exec.tests[0].name.contains("script post"));
}

#[tokio::test]
async fn la_carpeta_aporta_auth_cabeceras_y_scripts_en_orden() {
    let base = spawn_server();
    let tmp = tempfile::tempdir().unwrap();
    let mut col = Collection::init(tmp.path(), "flow").unwrap();
    col.meta.vars.insert("base_url".into(), base);
    col.meta.default_environment = None;
    col.meta.scripts.pre = "console.log('colección');".into();
    col.create_folder("api").unwrap();
    col.save_folder_settings(
        "api",
        [("X-Team".to_owned(), "core".to_owned())].into(),
        Auth::Bearer {
            token: "tok-123".into(),
        },
        harpoon_core::Scripts {
            pre: "console.log('carpeta');".into(),
            test: "test('de carpeta', () => expect(harpoon.res.status).to.equal(200));".into(),
            ..Default::default()
        },
    )
    .unwrap();
    let mut me = Request::new("me", Method::Get, "{{base_url}}/me");
    me.scripts.pre = "console.log('solicitud');".into();

    let client = Client::for_collection(&col).unwrap();
    let exec = harpoon_script::runner::execute_for_key(
        &client,
        &col,
        &me,
        None,
        indexmap::IndexMap::new(),
        "api/me",
    )
    .await
    .unwrap();
    assert_eq!(exec.response.status, 200, "la auth de la carpeta se envía");
    assert_eq!(exec.prepared.headers["Authorization"], "Bearer tok-123");
    assert_eq!(exec.prepared.headers["X-Team"], "core");
    assert_eq!(exec.logs, vec!["colección", "carpeta", "solicitud"]);
    assert!(exec.passed, "{:?}", exec.tests);
    assert_eq!(exec.tests.len(), 1);
}

#[test]
fn exportar_no_falla_si_un_script_pre_usa_apis_del_navegador() {
    let tmp = tempfile::tempdir().unwrap();
    let mut col = Collection::init(tmp.path(), "export").unwrap();
    col.meta.default_environment = None;
    let mut req = Request::new("x", Method::Get, "https://api.test/x");
    req.scripts.pre = "document.title = 'x';".into();
    req.headers
        .insert("Accept".into(), "application/json".into());

    let (prepared, warnings) =
        harpoon_script::runner::prepare_for_export(&col, &req, None, "x").unwrap();
    assert_eq!(prepared.url, "https://api.test/x");
    assert_eq!(prepared.headers["Accept"], "application/json");
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].contains("document"), "{warnings:?}");
}
