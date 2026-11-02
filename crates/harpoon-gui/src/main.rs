//! Harpoon — interfaz de escritorio.
//!
//! Tres columnas: colección · petición · respuesta. Todo el trabajo real lo
//! hacen `harpoon-core` y `harpoon-http`; aquí solo hay estado de pantalla.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod theme;
mod view;

use std::path::PathBuf;

use harpoon_core::{Body, Collection, Method, Request};
use harpoon_http::{Client, Outcome, Prepared, Response};
use iced::{Task, widget::text_editor};
use indexmap::IndexMap;

fn main() -> iced::Result {
    iced::application(App::boot, App::update, App::view)
        .title(App::title)
        .theme(App::theme)
        .window_size((1280.0, 800.0))
        .run()
}

/// Resultado de un envío, ya convertido a tipos `Send + Clone`.
#[derive(Debug, Clone)]
pub struct Exchange {
    pub prepared: Prepared,
    pub response: Response,
    pub outcomes: Vec<Outcome>,
}

#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum Message {
    Select(String),
    EnvPicked(String),
    MethodPicked(Method),
    UrlChanged(String),
    HeadersEdited(text_editor::Action),
    BodyEdited(text_editor::Action),
    Send,
    Sent(Result<Exchange, String>),
    Save,
    ToggleTheme,
}

pub struct App {
    collection: Option<Collection>,
    selected: Option<String>,
    env: Option<String>,
    method: Method,
    url: String,
    headers: text_editor::Content,
    body: text_editor::Content,
    exchange: Option<Result<Exchange, String>>,
    sending: bool,
    status: String,
    dark: bool,
}

impl App {
    fn boot() -> (Self, Task<Message>) {
        let start = std::env::args().nth(1).map_or_else(
            || std::env::current_dir().unwrap_or_default(),
            PathBuf::from,
        );
        let (collection, status) = match Collection::discover(&start) {
            Ok(c) => {
                let s = format!("{} · {}", c.meta.name, c.root.display());
                (Some(c), s)
            }
            Err(e) => (
                None,
                format!("{e}  —  abre Harpoon desde una carpeta con harpoon.toml"),
            ),
        };
        let env = collection
            .as_ref()
            .and_then(|c| c.meta.default_environment.clone());
        let mut app = Self {
            collection,
            selected: None,
            env,
            method: Method::Get,
            url: String::new(),
            headers: text_editor::Content::new(),
            body: text_editor::Content::new(),
            exchange: None,
            sending: false,
            status,
            dark: false,
        };
        if let Some(first) = app
            .collection
            .as_ref()
            .and_then(|c| c.requests.keys().next().cloned())
        {
            app.load(&first);
        }
        (app, Task::none())
    }

    fn title(&self) -> String {
        match (&self.collection, &self.selected) {
            (Some(c), Some(k)) => format!("{k} — {} — Harpoon", c.meta.name),
            (Some(c), None) => format!("{} — Harpoon", c.meta.name),
            _ => "Harpoon".into(),
        }
    }

    fn theme(&self) -> iced::Theme {
        if self.dark {
            theme::dark()
        } else {
            theme::light()
        }
    }

    /// Carga una solicitud en el editor.
    fn load(&mut self, key: &str) {
        let Some(req) = self.collection.as_ref().and_then(|c| c.requests.get(key)) else {
            return;
        };
        self.method = req.method;
        self.url = req.url.clone();
        let mut lines: Vec<String> = req
            .headers
            .iter()
            .map(|(k, v)| format!("{k}: {v}"))
            .collect();
        lines.extend(req.query.iter().map(|(k, v)| format!("?{k}={v}")));
        self.headers = text_editor::Content::with_text(&lines.join("\n"));
        self.body = text_editor::Content::with_text(match &req.body {
            Body::Json { json } => json,
            Body::Text { text } => text,
            Body::None | Body::Form { .. } => "",
        });
        self.selected = Some(key.to_owned());
        self.exchange = None;
    }

    /// Reconstruye la solicitud a partir del editor (conserva auth, expect, scripts y docs).
    fn current_request(&self) -> Option<Request> {
        let key = self.selected.as_ref()?;
        let base = self.collection.as_ref()?.requests.get(key)?;
        let mut req = base.clone();
        req.method = self.method;
        self.url.trim().clone_into(&mut req.url);
        req.headers = IndexMap::new();
        req.query = IndexMap::new();
        for line in self.headers.text().lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some(rest) = line.strip_prefix('?') {
                if let Some((k, v)) = rest.split_once('=') {
                    req.query.insert(k.trim().into(), v.trim().into());
                }
            } else if let Some((k, v)) = line.split_once(':') {
                req.headers.insert(k.trim().into(), v.trim().into());
            }
        }
        let body = self.body.text();
        let body = body.trim_end();
        req.body = if body.is_empty() {
            Body::None
        } else if serde_json::from_str::<serde_json::Value>(body).is_ok()
            || body.starts_with('{')
            || body.starts_with('[')
        {
            Body::Json {
                json: body.to_owned(),
            }
        } else {
            Body::Text {
                text: body.to_owned(),
            }
        };
        Some(req)
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Select(key) => self.load(&key),
            Message::EnvPicked(env) => self.env = Some(env),
            Message::MethodPicked(m) => self.method = m,
            Message::UrlChanged(u) => self.url = u,
            Message::HeadersEdited(a) => self.headers.perform(a),
            Message::BodyEdited(a) => self.body.perform(a),
            Message::ToggleTheme => self.dark = !self.dark,
            Message::Save => {
                if let (Some(req), Some(key)) = (self.current_request(), self.selected.clone()) {
                    if let Some(col) = self.collection.as_mut() {
                        self.status = match col.save_request(&key, req) {
                            Ok(()) => format!("Guardado requests/{key}.toml"),
                            Err(e) => format!("No se pudo guardar: {e}"),
                        };
                    }
                }
            }
            Message::Send => {
                if self.sending {
                    return Task::none();
                }
                let (Some(col), Some(req)) = (self.collection.clone(), self.current_request())
                else {
                    return Task::none();
                };
                let env = self.env.clone();
                self.sending = true;
                self.status = format!("Enviando {} {}…", req.method, req.url);
                return Task::perform(send(col, req, env), Message::Sent);
            }
            Message::Sent(result) => {
                self.sending = false;
                self.status = match &result {
                    Ok(x) => format!(
                        "{} · {} ms · {} B · {}",
                        x.response.status,
                        x.response.elapsed_ms,
                        x.response.size,
                        if x.outcomes.iter().all(|o| o.passed) {
                            "aserciones OK"
                        } else {
                            "aserciones con fallos"
                        }
                    ),
                    Err(e) => e.clone(),
                };
                self.exchange = Some(result);
            }
        }
        Task::none()
    }
}

async fn send(col: Collection, req: Request, env: Option<String>) -> Result<Exchange, String> {
    let scope = col.scope(env.as_deref()).map_err(|e| e.to_string())?;
    let client = Client::new(&col.meta.settings).map_err(|e| e.to_string())?;
    let (prepared, response, outcomes) = harpoon_http::execute(&client, &col, &req, &scope)
        .await
        .map_err(|e| e.to_string())?;
    Ok(Exchange {
        prepared,
        response,
        outcomes,
    })
}
