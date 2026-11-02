//! Handshake WebSocket con `OAuth2` y Digest contra un servidor local.
use super::open_websocket;
use harpoon_core::{Auth, Collection, Method, Request};
use std::{
    io::{Read, Write},
    sync::{Arc, Mutex},
};
use tokio_tungstenite::tungstenite::{
    handshake::server::{ErrorResponse, Request as WsRequest, Response as WsResponse},
    http::StatusCode,
};

/// Servidor WebSocket que acepta el handshake solo si `check` aprueba la cabecera
/// `Authorization`; si no, responde 401 con `challenge` (si lo hay).
async fn spawn_ws(
    check: impl Fn(&str) -> bool + Send + Sync + 'static,
    challenge: Option<&'static str>,
) -> (String, Arc<Mutex<Vec<String>>>) {
    spawn_ws_with_cookie(check, challenge, None).await
}

/// Como `spawn_ws`, pero añadiendo `Set-Cookie` tanto al 101 como al 401.
async fn spawn_ws_with_cookie(
    check: impl Fn(&str) -> bool + Send + Sync + 'static,
    challenge: Option<&'static str>,
    cookie: Option<&'static str>,
) -> (String, Arc<Mutex<Vec<String>>>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let log = seen.clone();
    let check = Arc::new(check);
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            let check = check.clone();
            let log = log.clone();
            tokio::spawn(async move {
                #[allow(clippy::result_large_err)]
                let callback = |request: &WsRequest, mut response: WsResponse| {
                    let auth = request
                        .headers()
                        .get("authorization")
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("")
                        .to_owned();
                    log.lock().unwrap().push(auth.clone());
                    if check(&auth) {
                        if let Some(cookie) = cookie {
                            response
                                .headers_mut()
                                .append("set-cookie", "abierta=1; Path=/".parse().unwrap());
                            response
                                .headers_mut()
                                .append("set-cookie", cookie.parse().unwrap());
                        }
                        return Ok(response);
                    }
                    let mut error = ErrorResponse::new(None);
                    *error.status_mut() = StatusCode::UNAUTHORIZED;
                    if cookie.is_some() {
                        error
                            .headers_mut()
                            .append("set-cookie", "reto=1; Path=/".parse().unwrap());
                    }
                    if let Some(challenge) = challenge {
                        error
                            .headers_mut()
                            .append("www-authenticate", "Basic realm=\"x\"".parse().unwrap());
                        error
                            .headers_mut()
                            .append("www-authenticate", challenge.parse().unwrap());
                    }
                    Err(error)
                };
                let _ = tokio_tungstenite::accept_hdr_async(stream, callback).await;
            });
        }
    });
    (format!("ws://{addr}/socket"), seen)
}

/// Endpoint de tokens (client credentials): emite `tok1`, `tok2`…
fn spawn_token_server(issued: Arc<Mutex<u32>>) -> String {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = stream.unwrap();
            let mut buf = vec![0u8; 16384];
            let _ = stream.read(&mut buf).unwrap();
            let body = {
                let mut n = issued.lock().unwrap();
                *n += 1;
                format!(r#"{{"access_token":"tok{n}","expires_in":3600,"token_type":"Bearer"}}"#)
            };
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(response.as_bytes()).unwrap();
        }
    });
    format!("http://{addr}/token")
}

fn collection() -> Collection {
    let tmp = tempfile::tempdir().unwrap();
    let mut col = Collection::init(tmp.path(), "ws").unwrap();
    col.environments.clear();
    col.meta.default_environment = None;
    std::mem::forget(tmp);
    col
}

async fn open(col: &Collection, url: &str, auth: Auth) -> Result<(), String> {
    let mut request = Request::new("ws", Method::Get, url);
    request.auth = auth;
    let scope = col.scope(None).unwrap();
    let prepared = harpoon_http::prepare(col, &request, &scope).unwrap();
    let parsed = url::Url::parse(&prepared.url).unwrap();
    open_websocket(col, &prepared, &parsed).await.map(|_| ())
}

#[tokio::test]
async fn digest_responde_al_reto_del_handshake() {
    let (url, seen) = spawn_ws(
        |auth| {
            auth.starts_with("Digest ")
                && auth.contains("username=\"Mufasa\"")
                && auth.contains("uri=\"/socket\"")
                && auth.contains("qop=auth")
        },
        Some(
            "Digest realm=\"testrealm@host.com\", qop=\"auth\", nonce=\"dcd98b7102dd2f0e8b11d0f600bfb0c093\", opaque=\"5ccc069c403ebaf9f0171e9517f40e41\"",
        ),
    )
    .await;
    let col = collection();
    let auth = Auth::Digest {
        username: "Mufasa".into(),
        password: "Circle Of Life".into(),
    };
    open(&col, &url, auth).await.unwrap();
    let seen = seen.lock().unwrap();
    assert_eq!(
        seen.len(),
        2,
        "un intento sin credenciales y otro con Digest"
    );
    assert!(seen[0].is_empty());
}

#[tokio::test]
async fn digest_sin_reto_devuelve_el_error() {
    let (url, seen) = spawn_ws(|_| false, None).await;
    let col = collection();
    let auth = Auth::Digest {
        username: "u".into(),
        password: "p".into(),
    };
    let error = open(&col, &url, auth).await.unwrap_err();
    assert!(
        error.contains("HTTP 401 Unauthorized") && error.contains("autenticación"),
        "{error}"
    );
    assert_eq!(seen.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn oauth2_envia_el_token_y_renueva_tras_401() {
    let issued = Arc::new(Mutex::new(0u32));
    let token_url = spawn_token_server(issued.clone());
    let current = issued.clone();
    let (url, seen) = spawn_ws(
        move |auth| auth == format!("Bearer tok{}", *current.lock().unwrap()),
        None,
    )
    .await;
    let col = collection();
    let auth = Auth::OAuth2 {
        grant: harpoon_core::model::OAuth2Grant::ClientCredentials,
        token_url,
        client_id: "cid".into(),
        client_secret: "csecret".into(),
        scope: String::new(),
        username: String::new(),
        password: String::new(),
        refresh_token: String::new(),
        client_auth: harpoon_core::model::OAuth2ClientAuth::Basic,
        extra: indexmap::IndexMap::default(),
        header_prefix: "Bearer".into(),
    };
    open(&col, &url, auth.clone()).await.unwrap();
    // El servidor deja de aceptar tok1: la siguiente conexión usa la caché,
    // recibe 401, pide un token nuevo y reintenta una sola vez.
    *issued.lock().unwrap() += 1;
    open(&col, &url, auth).await.unwrap();
    assert_eq!(
        *seen.lock().unwrap(),
        ["Bearer tok1", "Bearer tok1", "Bearer tok3"]
    );
}

#[tokio::test]
async fn guarda_las_cookies_del_handshake() {
    let (url, _) = spawn_ws_with_cookie(
        |auth| !auth.is_empty(),
        Some("Digest realm=\"r\", nonce=\"n\""),
        Some("sesion=abc; Path=/; HttpOnly"),
    )
    .await;
    let col = collection();
    let auth = Auth::Digest {
        username: "u".into(),
        password: "p".into(),
    };
    open(&col, &url, auth).await.unwrap();
    let http_url = url.replacen("ws://", "http://", 1);
    let header = harpoon_http::cookies::PersistentCookies::for_root(&col.root)
        .unwrap()
        .header_for_url(&http_url)
        .unwrap()
        .unwrap_or_default();
    for cookie in ["reto=1", "abierta=1", "sesion=abc"] {
        assert!(header.contains(cookie), "falta {cookie} en {header}");
    }
    assert!(col.root.join(".harpoon/cookies.json").is_file());
}
