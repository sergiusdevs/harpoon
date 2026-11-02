//! Paletas de Harpoon: minimalistas, con un solo color de acento.

use iced::{Color, Theme, theme::Palette};

const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::from_rgb8(r, g, b)
}

/// Acento: azul petróleo, el color de un arpón bajo el agua.
const ACCENT: Color = rgb(0x1f, 0x6f, 0x8b);

pub fn light() -> Theme {
    Theme::custom(
        "Harpoon Light",
        Palette {
            background: rgb(0xfa, 0xfa, 0xf8),
            text: rgb(0x1c, 0x1e, 0x21),
            primary: ACCENT,
            success: rgb(0x2e, 0x8b, 0x57),
            warning: rgb(0xc7, 0x8a, 0x00),
            danger: rgb(0xc0, 0x39, 0x2b),
        },
    )
}

pub fn dark() -> Theme {
    Theme::custom(
        "Harpoon Dark",
        Palette {
            background: rgb(0x14, 0x16, 0x1a),
            text: rgb(0xe6, 0xe6, 0xe3),
            primary: rgb(0x4f, 0xa3, 0xc4),
            success: rgb(0x5f, 0xb8, 0x7a),
            warning: rgb(0xe0, 0xa8, 0x2e),
            danger: rgb(0xe0, 0x5a, 0x4f),
        },
    )
}
