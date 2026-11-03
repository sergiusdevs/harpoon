//! Selección de proxy y mensajes de error del handshake.
use super::{ConnectError, HandshakeResponse, proxy_for};
use harpoon_core::Collection;
use tokio_tungstenite::tungstenite;

fn with_proxy(proxy: &str) -> Collection {
    let tmp = tempfile::tempdir().unwrap();
    let mut col = Collection::init(tmp.path(), "proxy").unwrap();
    col.meta.settings.proxy = Some(proxy.into());
    std::mem::forget(tmp);
    col
}

fn target(url: &str) -> url::Url {
    url::Url::parse(url).unwrap()
}

#[test]
fn el_proxy_de_la_coleccion_respeta_no_proxy() {
    let col = with_proxy("http://proxy.local:3128");
    let no_proxy = "localhost, .interno.example, 10.0.0.0/8";
    for direct in [
        "ws://localhost:8080/ws",
        "wss://api.interno.example/ws",
        "ws://10.1.2.3/ws",
    ] {
        assert!(
            proxy_for(&col, &target(direct), no_proxy)
                .unwrap()
                .is_none(),
            "{direct} debería conectar directamente"
        );
    }
    let proxied = proxy_for(&col, &target("wss://echo.example.com/ws"), no_proxy)
        .unwrap()
        .expect("debería usar el proxy");
    assert_eq!(proxied.uri().host(), Some("proxy.local"));
    assert!(
        proxy_for(&col, &target("ws://localhost/ws"), "")
            .unwrap()
            .is_some(),
        "sin NO_PROXY todo pasa por el proxy"
    );
}

fn rejected(status: u16, body: Option<&str>) -> String {
    let response: HandshakeResponse = tungstenite::http::Response::builder()
        .status(status)
        .body(body.map(|b| b.as_bytes().to_vec()))
        .unwrap();
    ConnectError::Handshake(tungstenite::Error::Http(Box::new(response))).to_string()
}

#[test]
fn describe_el_rechazo_del_handshake() {
    assert_eq!(
        rejected(401, None),
        "El servidor rechazó la conexión WebSocket (HTTP 401 Unauthorized): revisa la autenticación de la solicitud"
    );
    assert_eq!(
        rejected(403, Some("{\n  \"error\": \"forbidden\"\n}")),
        "El servidor rechazó la conexión WebSocket (HTTP 403 Forbidden): las credenciales no tienen acceso a este recurso. Respuesta: { \"error\": \"forbidden\" }"
    );
    assert!(rejected(200, None).contains("no aceptó el cambio de protocolo"));
    let long = rejected(500, Some(&"x".repeat(500)));
    assert!(long.ends_with('…') && long.len() < 400, "{long}");
}
