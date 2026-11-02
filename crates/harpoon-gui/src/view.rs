//! Composición de la ventana: barra superior + tres columnas.

use harpoon_core::Method;
use iced::{
    Border, Element, Font, Length, Theme,
    widget::{
        button, column, container, pick_list, row, scrollable, text, text_editor, text_input,
    },
};

use crate::{App, Exchange, Message};

const METHODS: [Method; 7] = [
    Method::Get,
    Method::Post,
    Method::Put,
    Method::Patch,
    Method::Delete,
    Method::Head,
    Method::Options,
];

const MONO: Font = Font::MONOSPACE;
const SIDEBAR_WIDTH: f32 = 240.0;
const PAD: u16 = 14;

impl App {
    pub fn view(&self) -> Element<'_, Message> {
        let body = row![self.sidebar(), self.editor(), self.response()]
            .spacing(0)
            .height(Length::Fill);
        column![self.topbar(), body, self.statusbar()].into()
    }

    fn topbar(&self) -> Element<'_, Message> {
        let name = self
            .collection
            .as_ref()
            .map_or("Sin colección", |c| c.meta.name.as_str());
        let envs: Vec<String> = self
            .collection
            .as_ref()
            .map(|c| c.environments.keys().cloned().collect())
            .unwrap_or_default();
        let env_picker: Element<'_, Message> = if envs.is_empty() {
            text("sin entornos").size(13).style(text::secondary).into()
        } else {
            pick_list(envs, self.env.clone(), Message::EnvPicked)
                .text_size(13)
                .padding([6, 10])
                .into()
        };
        let bar = row![
            text("Harpoon").size(18).font(Font {
                weight: iced::font::Weight::Semibold,
                ..MONO
            }),
            text(name)
                .size(14)
                .style(text::secondary)
                .width(Length::Fill),
            text("Entorno").size(13).style(text::secondary),
            env_picker,
            button(text(if self.dark { "☀" } else { "☾" }).size(14))
                .on_press(Message::ToggleTheme)
                .style(button::text)
                .padding([4, 8]),
        ]
        .spacing(12)
        .align_y(iced::Alignment::Center)
        .padding([10, PAD]);
        container(bar)
            .style(panel_strong)
            .width(Length::Fill)
            .into()
    }

    fn sidebar(&self) -> Element<'_, Message> {
        let mut list = column![].spacing(2);
        if let Some(col) = &self.collection {
            let mut current_folder = String::new();
            for (key, req) in &col.requests {
                let (folder, name) = key.rsplit_once('/').unwrap_or(("", key));
                if folder != current_folder {
                    folder.clone_into(&mut current_folder);
                    list = list.push(
                        text(folder.to_uppercase())
                            .size(11)
                            .style(text::secondary)
                            .font(Font {
                                weight: iced::font::Weight::Semibold,
                                ..Font::DEFAULT
                            }),
                    );
                }
                let selected = self.selected.as_deref() == Some(key);
                let label = row![
                    text(req.method.to_string())
                        .size(11)
                        .font(MONO)
                        .width(Length::Fixed(52.0))
                        .style(move |t: &Theme| text::Style {
                            color: Some(method_color(req.method, t))
                        }),
                    text(name).size(13),
                ]
                .spacing(6)
                .align_y(iced::Alignment::Center);
                list = list.push(
                    button(label)
                        .on_press(Message::Select(key.clone()))
                        .width(Length::Fill)
                        .padding([6, 8])
                        .style(move |t, s| sidebar_item(t, s, selected)),
                );
            }
        } else {
            list = list.push(text("Abre Harpoon desde una carpeta con harpoon.toml").size(13));
        }
        container(scrollable(list.padding([PAD, 10])))
            .width(Length::Fixed(SIDEBAR_WIDTH))
            .height(Length::Fill)
            .style(panel_soft)
            .into()
    }

    fn editor(&self) -> Element<'_, Message> {
        let send_label = if self.sending {
            "Enviando…"
        } else {
            "Enviar"
        };
        let mut send = button(text(send_label).size(14))
            .padding([8, 18])
            .style(button::primary);
        if !self.sending && self.selected.is_some() {
            send = send.on_press(Message::Send);
        }
        let url_row = row![
            pick_list(METHODS, Some(self.method), Message::MethodPicked)
                .font(MONO)
                .text_size(13)
                .padding([8, 10]),
            text_input("{{base_url}}/ruta", &self.url)
                .on_input(Message::UrlChanged)
                .on_submit(Message::Send)
                .font(MONO)
                .size(14)
                .padding(8),
            send,
            button(text("Guardar").size(14))
                .padding([8, 14])
                .style(button::secondary)
                .on_press(Message::Save),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center);

        let section = |title: &'static str, hint: &'static str| {
            row![
                text(title).size(12).style(text::secondary),
                text(hint)
                    .size(11)
                    .style(text::secondary)
                    .width(Length::Fill),
            ]
            .spacing(8)
        };

        let content = column![
            url_row,
            section(
                "CABECERAS Y QUERY",
                "una por línea: Clave: valor · ?param=valor"
            ),
            text_editor(&self.headers)
                .on_action(Message::HeadersEdited)
                .font(MONO)
                .size(13)
                .padding(8)
                .height(Length::FillPortion(2)),
            section("CUERPO", "JSON o texto; vacío = sin cuerpo"),
            text_editor(&self.body)
                .on_action(Message::BodyEdited)
                .font(MONO)
                .size(13)
                .padding(8)
                .height(Length::FillPortion(3)),
        ]
        .spacing(10)
        .padding(PAD);
        container(content)
            .width(Length::FillPortion(1))
            .height(Length::Fill)
            .into()
    }

    fn response(&self) -> Element<'_, Message> {
        let inner: Element<'_, Message> = match &self.exchange {
            None => container(
                text("Envía una petición para ver la respuesta")
                    .size(13)
                    .style(text::secondary),
            )
            .center(Length::Fill)
            .into(),
            Some(Err(e)) => container(text(e).size(13).style(|t: &Theme| text::Style {
                color: Some(t.palette().danger),
            }))
            .padding(PAD)
            .into(),
            Some(Ok(x)) => Self::exchange_view(x),
        };
        container(inner)
            .width(Length::FillPortion(1))
            .height(Length::Fill)
            .style(panel_soft)
            .into()
    }

    fn exchange_view(x: &Exchange) -> Element<'_, Message> {
        let res = &x.response;
        let status_color = move |t: &Theme| text::Style {
            color: Some(status_color(res.status, t)),
        };
        let header = row![
            text(res.status.to_string())
                .size(20)
                .font(MONO)
                .style(status_color),
            text(format!("{} ms", res.elapsed_ms))
                .size(13)
                .style(text::secondary),
            text(format!("{} B", res.size))
                .size(13)
                .style(text::secondary),
        ]
        .spacing(14)
        .align_y(iced::Alignment::Center);

        let mut asserts = column![].spacing(4);
        for o in &x.outcomes {
            let passed = o.passed;
            let line = match &o.detail {
                Some(d) => format!("{} {} — {d}", if passed { "✓" } else { "✗" }, o.name),
                None => format!("{} {}", if passed { "✓" } else { "✗" }, o.name),
            };
            asserts =
                asserts.push(
                    text(line)
                        .size(12)
                        .font(MONO)
                        .style(move |t: &Theme| text::Style {
                            color: Some(if passed {
                                t.palette().success
                            } else {
                                t.palette().danger
                            }),
                        }),
                );
        }

        let pretty = res.json().filter(|_| res.is_json()).map_or_else(
            || res.text(),
            |v| serde_json::to_string_pretty(&v).unwrap_or_default(),
        );
        let mut headers = column![].spacing(2);
        for (k, v) in &res.headers {
            headers = headers.push(
                row![
                    text(k).size(12).font(MONO).style(text::secondary),
                    text(v).size(12).font(MONO),
                ]
                .spacing(8),
            );
        }

        let content = column![
            header,
            asserts,
            text("CUERPO").size(12).style(text::secondary),
            text(pretty).size(13).font(MONO),
            text("CABECERAS").size(12).style(text::secondary),
            headers,
        ]
        .spacing(12)
        .padding(PAD);
        scrollable(content).into()
    }

    fn statusbar(&self) -> Element<'_, Message> {
        container(text(&self.status).size(12).style(text::secondary))
            .padding([6, PAD])
            .width(Length::Fill)
            .style(panel_strong)
            .into()
    }
}

fn method_color(m: Method, t: &Theme) -> iced::Color {
    let p = t.palette();
    match m {
        Method::Get => p.primary,
        Method::Post => p.success,
        Method::Put | Method::Patch => p.warning,
        Method::Delete => p.danger,
        Method::Head | Method::Options => t.extended_palette().background.base.text,
    }
}

fn status_color(status: u16, t: &Theme) -> iced::Color {
    let p = t.palette();
    match status {
        200..=299 => p.success,
        300..=399 => p.primary,
        400..=499 => p.warning,
        _ => p.danger,
    }
}

fn panel_soft(t: &Theme) -> container::Style {
    let e = t.extended_palette();
    container::Style {
        background: Some(e.background.weak.color.into()),
        ..container::Style::default()
    }
}

fn panel_strong(t: &Theme) -> container::Style {
    let e = t.extended_palette();
    container::Style {
        background: Some(e.background.weak.color.into()),
        border: Border {
            color: e.background.strong.color,
            width: 1.0,
            ..Border::default()
        },
        ..container::Style::default()
    }
}

fn sidebar_item(t: &Theme, status: button::Status, selected: bool) -> button::Style {
    let e = t.extended_palette();
    let bg = if selected {
        Some(e.primary.weak.color.into())
    } else if matches!(status, button::Status::Hovered | button::Status::Pressed) {
        Some(e.background.strong.color.into())
    } else {
        None
    };
    button::Style {
        background: bg,
        text_color: e.background.base.text,
        border: Border {
            radius: 6.0.into(),
            ..Border::default()
        },
        ..button::Style::default()
    }
}
