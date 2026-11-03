//! Pruebas de integración contra un servidor HTTP mínimo en `localhost`.
//! Sin dependencias: un hilo con `TcpListener` que responde según la ruta.

use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
};

use harpoon_core::{Auth, Body, Collection, Method, Request};
use harpoon_http::Client;

/// Arranca un servidor que devuelve la petición recibida como JSON y
/// obedece unas rutas especiales. Devuelve la URL base.
fn spawn_server() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = stream.unwrap();
            let mut buf = vec![0u8; 65536];
            let n = stream.read(&mut buf).unwrap();
            let raw = String::from_utf8_lossy(&buf[..n]).into_owned();
            let (head, body) = raw.split_once("\r\n\r\n").unwrap_or((&raw, ""));
            let mut lines = head.lines();
            let request_line = lines.next().unwrap_or_default().to_owned();
            let path = request_line
                .split_whitespace()
                .nth(1)
                .unwrap_or("/")
                .to_owned();
            let headers: Vec<(String, String)> = lines
                .filter_map(|l| l.split_once(": "))
                .map(|(k, v)| (k.to_ascii_lowercase(), v.to_owned()))
                .collect();

            let (status, ctype, out) = if path.starts_with("/status/") {
                let code = path.trim_start_matches("/status/").to_owned();
                (code, "text/plain", String::new())
            } else if path == "/slow" {
                thread::sleep(std::time::Duration::from_millis(300));
                ("200".into(), "text/plain", "slow".into())
            } else {
                let hdrs: Vec<String> = headers
                    .iter()
                    .map(|(k, v)| format!("{k:?}:{v:?}"))
                    .collect();
                (
                    "200".into(),
                    "application/json",
                    format!(
                        "{{\"line\":{:?},\"headers\":{{{}}},\"body\":{:?}}}",
                        request_line,
                        hdrs.join(","),
                        body
                    ),
                )
            };
            let reason = if status == "200" { "OK" } else { "X" };
            let response = format!(
                "HTTP/1.1 {status} {reason}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{out}",
                out.len()
            );
            stream.write_all(response.as_bytes()).unwrap();
        }
    });
    format!("http://{addr}")
}

fn collection(base_url: &str) -> Collection {
    let tmp = tempfile::tempdir().unwrap();
    let mut col = Collection::init(tmp.path(), "test").unwrap();
    col.meta.vars.insert("base_url".into(), base_url.into());
    col.meta.auth = Auth::Bearer {
        token: "{{token}}".into(),
    };
    col.environments.clear(); // sin entorno: usamos solo vars de colección
    col.meta.default_environment = None;
    col.meta.vars.insert("token".into(), "T0K3N".into());
    // Evita que el tempdir se borre antes de tiempo.
    std::mem::forget(tmp);
    col
}

#[test]
fn user_agent_por_defecto_se_puede_sobrescribir_o_desactivar() {
    let col = collection("http://localhost");
    let scope = col.scope(None).unwrap();
    let mut req = Request::new("ua", Method::Get, "{{base_url}}/");

    let prepared = harpoon_http::prepare(&col, &req, &scope).unwrap();
    assert_eq!(prepared.headers["User-Agent"], harpoon_http::USER_AGENT);

    req.headers
        .insert("user-agent".into(), "mi-cliente/1".into());
    let prepared = harpoon_http::prepare(&col, &req, &scope).unwrap();
    assert_eq!(prepared.headers["user-agent"], "mi-cliente/1");
    assert!(!prepared.headers.contains_key("User-Agent"));

    req.headers.clear();
    req.disabled_headers.push("User-Agent".into());
    let prepared = harpoon_http::prepare(&col, &req, &scope).unwrap();
    assert!(
        !prepared
            .headers
            .keys()
            .any(|k| k.eq_ignore_ascii_case("user-agent"))
    );
}

#[tokio::test]
async fn envia_con_auth_heredada_query_y_cuerpo_json() {
    let base = spawn_server();
    let col = collection(&base);
    let scope = col.scope(None).unwrap();
    let client = Client::new(&col.meta.settings).unwrap();

    let mut req = Request::new("echo", Method::Post, "{{base_url}}/echo");
    req.query.insert("q".into(), "a b".into());
    req.body = Body::Json {
        json: r#"{"hello":"{{token}}"}"#.into(),
    };
    req.expect.status = Some(200);
    req.expect.body_contains.push("T0K3N".into());

    let (prepared, res, outcomes) = harpoon_http::execute(&client, &col, &req, &scope)
        .await
        .unwrap();
    assert_eq!(prepared.headers["Authorization"], "Bearer T0K3N");
    assert_eq!(res.status, 200);
    assert!(res.is_json());

    let json = res.json().unwrap();
    assert_eq!(json["line"], "POST /echo?q=a+b HTTP/1.1");
    assert_eq!(json["headers"]["authorization"], "Bearer T0K3N");
    assert_eq!(json["headers"]["content-type"], "application/json");
    assert_eq!(json["body"], r#"{"hello":"T0K3N"}"#);
    assert!(outcomes.iter().all(|o| o.passed), "{outcomes:?}");
}

#[tokio::test]
async fn auth_basic_y_api_key_en_query() {
    let base = spawn_server();
    let col = collection(&base);
    let scope = col.scope(None).unwrap();
    let client = Client::new(&col.meta.settings).unwrap();

    let mut req = Request::new("basic", Method::Get, "{{base_url}}/x");
    req.auth = Auth::Basic {
        username: "user".into(),
        password: "pass".into(),
    };
    let (_, res, _) = harpoon_http::execute(&client, &col, &req, &scope)
        .await
        .unwrap();
    assert_eq!(
        res.json().unwrap()["headers"]["authorization"],
        "Basic dXNlcjpwYXNz"
    );

    let mut req = Request::new("key", Method::Get, "{{base_url}}/x");
    req.auth = Auth::ApiKey {
        key: "api_key".into(),
        value: "k".into(),
        r#in: harpoon_core::model::ApiKeyLocation::Query,
    };
    let (_, res, _) = harpoon_http::execute(&client, &col, &req, &scope)
        .await
        .unwrap();
    assert_eq!(res.json().unwrap()["line"], "GET /x?api_key=k HTTP/1.1");
}

#[tokio::test]
async fn aserciones_fallidas_y_timeout() {
    let base = spawn_server();
    let mut col = collection(&base);
    let scope = col.scope(None).unwrap();

    let mut req = Request::new("fail", Method::Get, "{{base_url}}/status/503");
    req.expect.status = Some(200);
    let client = Client::new(&col.meta.settings).unwrap();
    let (_, res, outcomes) = harpoon_http::execute(&client, &col, &req, &scope)
        .await
        .unwrap();
    assert_eq!(res.status, 503);
    assert_eq!(outcomes.len(), 1);
    assert!(!outcomes[0].passed);

    col.meta.settings.timeout_ms = 50;
    let client = Client::new(&col.meta.settings).unwrap();
    let req = Request::new("slow", Method::Get, "{{base_url}}/slow");
    let err = harpoon_http::execute(&client, &col, &req, &scope)
        .await
        .unwrap_err();
    assert!(matches!(err, harpoon_http::Error::Timeout(_)), "{err}");
}

#[tokio::test]
async fn variable_sin_definir_no_envia_nada() {
    let col = collection("http://127.0.0.1:1");
    let scope = col.scope(None).unwrap();
    let client = Client::new(&col.meta.settings).unwrap();
    let req = Request::new("bad", Method::Get, "{{base_url}}/{{missing}}");
    let err = harpoon_http::execute(&client, &col, &req, &scope)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("missing"));
}

// ------------------------------------------------------------ OAuth2 / Digest

/// Servidor con `/token` (client credentials), `/digest` (reto MD5) y `/api`
/// (exige `Bearer` válido; el token cambia en cada emisión).
fn spawn_auth_server() -> String {
    use std::sync::{Arc, Mutex};
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let issued = Arc::new(Mutex::new(0u32));
    thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = stream.unwrap();
            let mut buf = vec![0u8; 65536];
            let n = stream.read(&mut buf).unwrap();
            let raw = String::from_utf8_lossy(&buf[..n]).into_owned();
            let (head, body) = raw.split_once("\r\n\r\n").unwrap_or((&raw, ""));
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
            let (status, extra, out) = if path == "/token" {
                // client_id:client_secret en Basic = "cid:csecret"
                if auth != "Basic Y2lkOmNzZWNyZXQ="
                    || !body.contains("grant_type=client_credentials")
                {
                    (
                        "401",
                        String::new(),
                        r#"{"error":"invalid_client"}"#.to_owned(),
                    )
                } else {
                    let mut n = issued.lock().unwrap();
                    *n += 1;
                    (
                        "200",
                        String::new(),
                        format!(
                            r#"{{"access_token":"tok{}","expires_in":3600,"token_type":"Bearer"}}"#,
                            *n
                        ),
                    )
                }
            } else if path == "/api" {
                let n = *issued.lock().unwrap();
                if auth == format!("Bearer tok{n}") {
                    (
                        "200",
                        String::new(),
                        format!(r#"{{"ok":true,"token":"tok{n}"}}"#),
                    )
                } else {
                    ("401", String::new(), r#"{"error":"bad token"}"#.to_owned())
                }
            } else if path == "/digest" {
                if auth.starts_with("Digest ")
                    && auth.contains("username=\"Mufasa\"")
                    && auth.contains("response=\"")
                {
                    ("200", String::new(), r#"{"digest":"ok"}"#.to_owned())
                } else {
                    (
                        "401",
                        "WWW-Authenticate: Digest realm=\"testrealm@host.com\", qop=\"auth\", nonce=\"dcd98b7102dd2f0e8b11d0f600bfb0c093\", opaque=\"5ccc069c403ebaf9f0171e9517f40e41\"\r\n".to_owned(),
                        String::new(),
                    )
                }
            } else {
                ("404", String::new(), String::new())
            };
            let response = format!(
                "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\n{extra}Content-Length: {}\r\nConnection: close\r\n\r\n{out}",
                out.len()
            );
            stream.write_all(response.as_bytes()).unwrap();
        }
    });
    format!("http://{addr}")
}

#[tokio::test]
async fn oauth2_client_credentials_con_cache_y_reintento() {
    let base = spawn_auth_server();
    let tmp = tempfile::tempdir().unwrap();
    let mut col = Collection::init(tmp.path(), "oauth").unwrap();
    col.meta.default_environment = None;
    col.meta.vars.insert("base_url".into(), base.clone());
    col.meta.auth = Auth::OAuth2 {
        grant: harpoon_core::model::OAuth2Grant::ClientCredentials,
        token_url: "{{base_url}}/token".into(),
        client_id: "cid".into(),
        client_secret: "csecret".into(),
        scope: "read".into(),
        username: String::new(),
        password: String::new(),
        refresh_token: String::new(),
        client_auth: harpoon_core::model::OAuth2ClientAuth::Basic,
        extra: indexmap::IndexMap::default(),
        header_prefix: "Bearer".into(),
    };
    let scope = col.scope(None).unwrap();
    let req = Request::new("api", Method::Get, "{{base_url}}/api");

    let client = Client::for_collection(&col).unwrap();
    let (_, res, _) = harpoon_http::execute(&client, &col, &req, &scope)
        .await
        .unwrap();
    assert_eq!(res.status, 200);
    assert_eq!(res.json().unwrap()["token"], "tok1");
    assert!(
        tmp.path().join(".harpoon/tokens.toml").is_file(),
        "token cacheado en disco"
    );

    // Segunda llamada: reutiliza tok1 (no emite otro).
    let (_, res, _) = harpoon_http::execute(&client, &col, &req, &scope)
        .await
        .unwrap();
    assert_eq!(res.json().unwrap()["token"], "tok1");

    // Un cliente nuevo lee la caché del disco; forzamos que el servidor
    // invalide tok1 pidiendo un token "fuera" (tok2) y comprobamos que, tras
    // el 401, Harpoon renueva y reintenta solo una vez.
    let http = reqwest::Client::new();
    http.post(format!("{base}/token"))
        .header("Authorization", "Basic Y2lkOmNzZWNyZXQ=")
        .body("grant_type=client_credentials")
        .send()
        .await
        .unwrap(); // ahora el servidor solo acepta tok2
    let client2 = Client::for_collection(&col).unwrap();
    let (_, res, _) = harpoon_http::execute(&client2, &col, &req, &scope)
        .await
        .unwrap();
    assert_eq!(res.status, 200);
    assert_eq!(res.json().unwrap()["token"], "tok3");
}

#[tokio::test]
async fn digest_responde_al_reto() {
    let base = spawn_auth_server();
    let col = collection(&base);
    let scope = col.scope(None).unwrap();
    let client = Client::for_collection(&col).unwrap();
    let mut req = Request::new("d", Method::Get, "{{base_url}}/digest");
    req.auth = Auth::Digest {
        username: "Mufasa".into(),
        password: "Circle Of Life".into(),
    };
    let (_, res, _) = harpoon_http::execute(&client, &col, &req, &scope)
        .await
        .unwrap();
    assert_eq!(res.status, 200, "{}", res.text());
    assert_eq!(res.json().unwrap()["digest"], "ok");
}
