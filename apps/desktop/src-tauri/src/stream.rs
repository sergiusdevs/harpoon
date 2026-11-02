//! Sesiones efímeras WebSocket y SSE de la aplicación de escritorio.
use eventsource_stream::Eventsource;
use futures_util::{SinkExt, StreamExt};
use harpoon_core::{Body, Method, Request};
use harpoon_http::{Client, Prepared};
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
    let collection = harpoon_core::Collection::discover(&root).map_err(|e| e.to_string())?;
    request.method = Method::Get;
    request.body = Body::None;
    let scope = collection
        .scope_for_request(env.as_deref(), &key)
        .map_err(|e| e.to_string())?
        .with_layer(collection.load_runtime_vars());
    let prepared =
        harpoon_http::prepare(&collection, &request, &scope).map_err(|e| e.to_string())?;
    if prepared.pending_auth.is_some() {
        return Err("Las conexiones en tiempo real todavía no admiten OAuth2 ni Digest".into());
    }
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
    if is_ws
        && (collection.meta.settings.proxy.is_some()
            || collection.meta.settings.insecure
            || collection.meta.settings.ca_cert.is_some()
            || collection.meta.settings.client_cert.is_some()
            || collection.meta.settings.client_key.is_some())
    {
        return Err(
            "WebSocket aún no admite proxy ni ajustes TLS personalizados de la colección".into(),
        );
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
        let name =
            tokio_tungstenite::tungstenite::http::header::HeaderName::from_bytes(name.as_bytes())
                .map_err(|e| e.to_string())?;
        let value = tokio_tungstenite::tungstenite::http::header::HeaderValue::from_str(value)
            .map_err(|e| e.to_string())?;
        request.headers_mut().insert(name, value);
    }
    if !request.headers().contains_key("cookie") {
        let mut cookie_url = url.clone();
        let scheme = if url.scheme() == "wss" {
            "https"
        } else {
            "http"
        };
        cookie_url
            .set_scheme(scheme)
            .map_err(|()| "URL de cookies no válida")?;
        if let Some(cookie) = harpoon_http::cookies::PersistentCookies::for_root(&collection.root)
            .and_then(|jar| jar.header_for_url(cookie_url.as_str()))
            .map_err(|e| e.to_string())?
        {
            request
                .headers_mut()
                .insert("cookie", cookie.parse().map_err(|_| "Cookie no válida")?);
        }
    }
    let config = WebSocketConfig::default()
        .max_message_size(Some(MAX_MESSAGE))
        .max_frame_size(Some(MAX_MESSAGE));
    let (mut socket, _) = tokio::select! {
        result = tokio_tungstenite::connect_async_with_config(request, Some(config), false) => result.map_err(|e| e.to_string())?,
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
async fn sse(
    collection: &harpoon_core::Collection,
    prepared: &Prepared,
    channel: &Channel<StreamEvent>,
    mut actions: mpsc::Receiver<Action>,
) -> Result<(), String> {
    let client = Client::for_collection(collection).map_err(|e| e.to_string())?;
    let response = tokio::select! { response = client.open_sse(prepared) => response.map_err(|e| e.to_string())?, _ = actions.recv() => return Ok(()) };
    if !response.status().is_success() {
        return Err(format!("El servidor respondió HTTP {}", response.status()));
    }
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if !content_type
        .to_ascii_lowercase()
        .starts_with("text/event-stream")
    {
        return Err(format!("Content-Type SSE no válido: {content_type}"));
    }
    emit(channel, "open", "Conectado", "", "");
    let mut events = response.bytes_stream().eventsource();
    loop {
        tokio::select! {
            _ = actions.recv() => break,
            event = events.next() => match event { Some(Ok(event)) => { if event.data.len() > MAX_MESSAGE { return Err("Evento demasiado grande (máximo 1 MB)".into()); } emit(channel, "message", event.data, event.event, event.id); }, Some(Err(error)) => return Err(error.to_string()), None => break }
        }
    }
    Ok(())
}
