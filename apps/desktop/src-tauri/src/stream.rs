//! Sesiones efímeras WebSocket y SSE de la aplicación de escritorio.
use crate::ws_transport;
use futures_util::{SinkExt, StreamExt};
use harpoon_core::{Body, Method, Request, RequestType};
use harpoon_http::{Client, Prepared, auth::Pending};
use serde::Serialize;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tauri::{State, ipc::Channel};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::{
    client::IntoClientRequest,
    http::Request as HandshakeRequest,
    protocol::{Message, WebSocketConfig},
};

const MAX_MESSAGE: usize = 1024 * 1024;
#[derive(Default)]
pub struct Sessions(Arc<Mutex<HashMap<String, mpsc::Sender<Action>>>>);
enum Action {
    Send(String),
    Stop,
}
#[derive(Clone, Serialize)]
pub struct StreamEvent {
    kind: &'static str,
    data: String,
    event: String,
    id: String,
}
fn emit(
    channel: &Channel<StreamEvent>,
    kind: &'static str,
    data: impl Into<String>,
    event: impl Into<String>,
    id: impl Into<String>,
) {
    let _ = channel.send(StreamEvent {
        kind,
        data: data.into(),
        event: event.into(),
        id: id.into(),
    });
}
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn start_stream(
    root: PathBuf,
    key: String,
    env: Option<String>,
    mut request: Request,
    protocol: String,
    id: String,
    channel: Channel<StreamEvent>,
    state: State<'_, Sessions>,
) -> Result<(), String> {
    if id.is_empty() || id.len() > 100 {
        return Err("Identificador de conexión no válido".into());
    }
    let expected = match request.request_type() {
        RequestType::Websocket => "websocket",
        RequestType::Sse => "sse",
        _ => return Err("Esta solicitud no es WebSocket ni SSE".into()),
    };
    if protocol != expected {
        return Err("El protocolo no coincide con el tipo guardado en la solicitud".into());
    }
    let collection = harpoon_core::Collection::discover(&root).map_err(|e| e.to_string())?;
    request.method = Method::Get;
    request.body = Body::None;
    let scope = collection
        .scope_for_request(env.as_deref(), &key)
        .map_err(|e| e.to_string())?
        .with_layer(collection.load_runtime_vars());
    let request = collection.with_folder_inheritance(&key, &request);
    let prepared =
        harpoon_http::prepare(&collection, &request, &scope).map_err(|e| e.to_string())?;
    let is_ws = match protocol.as_str() {
        "websocket" => true,
        "sse" => false,
        _ => return Err("Protocolo no válido".into()),
    };
    let parsed = url::Url::parse(&prepared.url).map_err(|e| e.to_string())?;
    if (is_ws && !matches!(parsed.scheme(), "ws" | "wss"))
        || (!is_ws && !matches!(parsed.scheme(), "http" | "https"))
    {
        return Err("La URL debe usar ws/wss para WebSocket o http/https para SSE".into());
    }
    let (sender, receiver) = mpsc::channel(32);
    {
        let mut sessions = state.0.lock().map_err(|_| "No se pudo abrir la conexión")?;
        if sessions.contains_key(&id) {
            return Err("La conexión ya existe".into());
        }
        sessions.insert(id.clone(), sender);
    }
    let sessions = state.0.clone();
    tauri::async_runtime::spawn(async move {
        let result = if is_ws {
            websocket(&collection, &prepared, &channel, receiver).await
        } else {
            sse(&collection, &prepared, &channel, receiver).await
        };
        if let Err(error) = result {
            emit(&channel, "error", error, "", "");
        }
        emit(&channel, "closed", "", "", "");
        if let Ok(mut sessions) = sessions.lock() {
            sessions.remove(&id);
        }
    });
    Ok(())
}
#[tauri::command]
pub async fn stream_send(
    id: String,
    data: String,
    state: State<'_, Sessions>,
) -> Result<(), String> {
    if data.len() > MAX_MESSAGE {
        return Err("Mensaje demasiado grande (máximo 1 MB)".into());
    }
    let sender = state
        .0
        .lock()
        .map_err(|_| "Conexión no disponible")?
        .get(&id)
        .cloned()
        .ok_or("Conexión cerrada")?;
    sender
        .send(Action::Send(data))
        .await
        .map_err(|_| "Conexión cerrada".into())
}
#[tauri::command]
pub async fn stop_stream(id: String, state: State<'_, Sessions>) -> Result<(), String> {
    let sender = state
        .0
        .lock()
        .map_err(|_| "Conexión no disponible")?
        .get(&id)
        .cloned();
    if let Some(sender) = sender {
        let _ = sender.send(Action::Stop).await;
    }
    Ok(())
}
/// Construye el handshake con las cabeceras, cookies y, si se indica, la autorización.
fn ws_request(
    collection: &harpoon_core::Collection,
    prepared: &Prepared,
    url: &url::Url,
    authorization: Option<&str>,
) -> Result<HandshakeRequest<()>, String> {
    use tokio_tungstenite::tungstenite::http::header::{AUTHORIZATION, HeaderName, HeaderValue};
    let mut request = url
        .as_str()
        .into_client_request()
        .map_err(|e| e.to_string())?;
    for (name, value) in &prepared.headers {
        if [
            "host",
            "connection",
            "upgrade",
            "sec-websocket-key",
            "sec-websocket-version",
            "sec-websocket-extensions",
        ]
        .iter()
        .any(|reserved| name.eq_ignore_ascii_case(reserved))
        {
            return Err(format!("La cabecera {name} la gestiona WebSocket"));
        }
        if authorization.is_some() && name.eq_ignore_ascii_case("authorization") {
            continue;
        }
        let name = HeaderName::from_bytes(name.as_bytes()).map_err(|e| e.to_string())?;
        let value = HeaderValue::from_str(value).map_err(|e| e.to_string())?;
        request.headers_mut().insert(name, value);
    }
    if let Some(authorization) = authorization {
        let value = HeaderValue::from_str(authorization)
            .map_err(|_| "Cabecera de autorización no válida")?;
        request.headers_mut().insert(AUTHORIZATION, value);
    }
    if !request.headers().contains_key("cookie") {
        let cookie_url = cookie_url(url)?;
        if let Some(cookie) = harpoon_http::cookies::PersistentCookies::for_root(&collection.root)
            .and_then(|jar| jar.header_for_url(cookie_url.as_str()))
            .map_err(|e| e.to_string())?
        {
            request
                .headers_mut()
                .insert("cookie", cookie.parse().map_err(|_| "Cookie no válida")?);
        }
    }
    Ok(request)
}
/// URL http/https equivalente, con la que se indexan las cookies de la colección.
fn cookie_url(url: &url::Url) -> Result<url::Url, String> {
    let mut cookie_url = url.clone();
    let scheme = if url.scheme() == "wss" {
        "https"
    } else {
        "http"
    };
    cookie_url
        .set_scheme(scheme)
        .map_err(|()| "URL de cookies no válida")?;
    Ok(cookie_url)
}
/// Guarda en el jar de la colección las `Set-Cookie` de la respuesta al handshake.
fn store_cookies(
    collection: &harpoon_core::Collection,
    url: &url::Url,
    response: &ws_transport::HandshakeResponse,
) {
    let values: Vec<&str> = response
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|value| value.to_str().ok())
        .collect();
    if values.is_empty() {
        return;
    }
    let stored = cookie_url(url)
        .map_err(harpoon_http::Error::Cookies)
        .and_then(|cookie_url| {
            harpoon_http::cookies::PersistentCookies::for_root(&collection.root)
                .and_then(|jar| jar.store_set_cookies(cookie_url.as_str(), values))
        });
    if let Err(error) = stored {
        eprintln!("No se pudieron guardar las cookies: {error}");
    }
}
/// Respuesta HTTP del handshake si el servidor lo rechazó con 401.
fn unauthorized(error: &ws_transport::ConnectError) -> Option<&ws_transport::HandshakeResponse> {
    match error {
        ws_transport::ConnectError::Handshake(tokio_tungstenite::tungstenite::Error::Http(
            response,
        )) if response.status().as_u16() == 401 => Some(response),
        _ => None,
    }
}
/// Abre el WebSocket resolviendo `OAuth2` (con un reintento tras 401) o el reto Digest.
async fn open_websocket(
    collection: &harpoon_core::Collection,
    prepared: &Prepared,
    url: &url::Url,
) -> Result<ws_transport::Socket, String> {
    let config = WebSocketConfig::default()
        .max_message_size(Some(MAX_MESSAGE))
        .max_frame_size(Some(MAX_MESSAGE));
    let attempt = |authorization: Option<String>| async move {
        let request = ws_request(collection, prepared, url, authorization.as_deref())
            .map_err(ws_transport::ConnectError::Transport)?;
        let result = ws_transport::connect(collection, url, request, config).await;
        match &result {
            Ok((_, response)) => store_cookies(collection, url, response),
            Err(ws_transport::ConnectError::Handshake(
                tokio_tungstenite::tungstenite::Error::Http(response),
            )) => store_cookies(collection, url, response),
            Err(_) => {}
        }
        result.map(|(socket, _)| socket)
    };
    match &prepared.pending_auth {
        None => attempt(None).await.map_err(|e| e.to_string()),
        Some(Pending::OAuth2(cfg)) => {
            let client = Client::for_collection(collection).map_err(|e| e.to_string())?;
            let header = client
                .oauth2_header(cfg, false)
                .await
                .map_err(|e| e.to_string())?;
            match attempt(Some(header)).await {
                Err(error) if unauthorized(&error).is_some() => {
                    let header = client
                        .oauth2_header(cfg, true)
                        .await
                        .map_err(|e| e.to_string())?;
                    attempt(Some(header)).await.map_err(|e| e.to_string())
                }
                result => result.map_err(|e| e.to_string()),
            }
        }
        Some(Pending::Digest { username, password }) => {
            let error = match attempt(None).await {
                Ok(socket) => return Ok(socket),
                Err(error) => error,
            };
            let header = unauthorized(&error).and_then(|response| {
                response
                    .headers()
                    .get_all("www-authenticate")
                    .iter()
                    .filter_map(|value| value.to_str().ok())
                    .find_map(|challenge| {
                        harpoon_http::auth::digest_authorization(
                            challenge,
                            "GET",
                            &harpoon_http::auth::request_uri(url.as_str()),
                            username,
                            password,
                        )
                    })
            });
            match header {
                Some(header) => attempt(Some(header)).await.map_err(|e| e.to_string()),
                None => Err(error.to_string()),
            }
        }
    }
}
async fn websocket(
    collection: &harpoon_core::Collection,
    prepared: &Prepared,
    channel: &Channel<StreamEvent>,
    mut actions: mpsc::Receiver<Action>,
) -> Result<(), String> {
    let mut url = url::Url::parse(&prepared.url).map_err(|e| e.to_string())?;
    if !prepared.query.is_empty() {
        url.query_pairs_mut().extend_pairs(prepared.query.iter());
    }
    let mut socket = tokio::select! {
        result = open_websocket(collection, prepared, &url) => result?,
        _ = actions.recv() => return Ok(()),
    };
    emit(channel, "open", "Conectado", "", "");
    loop {
        tokio::select! {
            action = actions.recv() => if let Some(Action::Send(data)) = action { socket.send(Message::Text(data.clone().into())).await.map_err(|e| e.to_string())?; emit(channel, "sent", data, "", ""); } else { let _ = socket.close(None).await; break; },
            next = socket.next() => match next { Some(Ok(Message::Text(text))) => emit(channel, "message", text.to_string(), "texto", ""), Some(Ok(Message::Binary(data))) => emit(channel, "message", format!("Mensaje binario: {} bytes", data.len()), "binario", ""), Some(Ok(Message::Close(_))) | None => break, Some(Err(error)) => return Err(error.to_string()), _ => {} }
        }
    }
    Ok(())
}
#[allow(clippy::if_not_else)]
async fn sse(
    collection: &harpoon_core::Collection,
    prepared: &Prepared,
    channel: &Channel<StreamEvent>,
    mut actions: mpsc::Receiver<Action>,
) -> Result<(), String> {
    use std::time::Duration;
    let client = Client::for_collection(collection).map_err(|e| e.to_string())?;
    let mut last_id: Option<String> = None;
    let mut retry = Duration::from_secs(3);
    loop {
        let response = tokio::select! {
            response = client.open_sse(prepared, last_id.as_deref()) => response,
            _ = actions.recv() => return Ok(()),
        };
        match response {
            Ok(response) => {
                if response.status().as_u16() == 204 {
                    return Ok(());
                }
                if response.status().is_client_error() && response.status().as_u16() != 429 {
                    return Err(format!("El servidor respondió HTTP {}", response.status()));
                }
                if !response.status().is_success() {
                    emit(
                        channel,
                        "reconnecting",
                        format!("HTTP {}. Reintentando…", response.status()),
                        "",
                        "",
                    );
                } else {
                    let content_type = response
                        .headers()
                        .get("content-type")
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("");
                    if !content_type
                        .split(';')
                        .next()
                        .unwrap_or("")
                        .trim()
                        .eq_ignore_ascii_case("text/event-stream")
                    {
                        return Err(format!("Content-Type SSE no válido: {content_type}"));
                    }
                    emit(channel, "open", "Conectado", "", "");
                    let mut decoder = crate::sse::Decoder::new(last_id.take(), retry);
                    let mut stream = response.bytes_stream();
                    loop {
                        tokio::select! {
                            _ = actions.recv() => return Ok(()),
                            chunk = stream.next() => match chunk {
                                Some(Ok(chunk)) => for event in decoder.feed(&chunk)? { emit(channel, "message", event.data, event.kind, event.id); },
                                Some(Err(error)) => { emit(channel, "reconnecting", format!("{error}. Reintentando…"), "", ""); break; },
                                None => { emit(channel, "reconnecting", "Conexión cerrada. Reintentando…", "", ""); break; },
                            }
                        }
                    }
                    last_id = decoder.last_id;
                    retry = decoder.retry;
                }
            }
            Err(harpoon_http::Error::Network(error)) => emit(
                channel,
                "reconnecting",
                format!("{error}. Reintentando…"),
                "",
                "",
            ),
            Err(error) => return Err(error.to_string()),
        }
        tokio::select! { _ = actions.recv() => return Ok(()), () = tokio::time::sleep(retry) => {} }
    }
}
#[cfg(test)]
mod tests;
