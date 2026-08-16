//! Hakkında / Kısayollar katmanı — Slint `AboutWindow` ve `ShortcutsWindow`
//! karşılığı.
//!
//! Slint'te bunlar ayrı `Window` bileşenleriydi. iced'de ayrı pencere açmak
//! `daemon` mimarisi gerektirir; aynı görsel içerik burada `stack!` üzerine
//! bindirilen bir katmanla veriliyor.

use iced::widget::{button, column, container, mouse_area, row, text, Space};
use iced::{Alignment, Background, Element, Fill, Theme};

use crate::message::{Message, Modal};
use crate::styles;
use crate::theme::{
    Tokens, CONTROL_HEIGHT, FONT_LG, FONT_MD, FONT_SM, FONT_XL, SPACE_MD, SPACE_XL,
};
use crate::view::widgets::separator;

/// Katman genişliği (Slint pencere genişlikleri: 380px / 340px).
const PANEL_WIDTH: f32 = 380.0;
/// Kısayol tuş kutusu genişliği.
const KEY_WIDTH: f32 = 96.0;
/// Tuş kutusu yüksekliği.
const KEY_HEIGHT: f32 = 28.0;

/// Klavye kısayolları — Slint listesine WASD satırları da eklendi.
const SHORTCUTS: [(&str, &str); 9] = [
    ("W / ↑", "İleri"),
    ("S / ↓", "Geri"),
    ("A / ←", "Sol dönüş"),
    ("D / →", "Sağ dönüş"),
    ("Space", "Acil durdurma"),
    ("1 / 2 / 3", "Vites seç"),
    ("L", "Işık aç/kapat"),
    ("B", "Fren aç/kapat"),
    ("C / M", "Kamera / Harita sekmesi"),
];

pub fn view(modal: Modal) -> Element<'static, Message> {
    let panel_content = match modal {
        Modal::About => about(),
        Modal::Shortcuts => shortcuts(),
    };

    // Karartma katmanı — boş alana tıklamak kapatır.
    mouse_area(
        container(panel_content)
            .center(Fill)
            .style(|theme: &Theme| container::Style {
                background: Some(Background::Color(Tokens::for_theme(theme).scrim)),
                ..container::Style::default()
            }),
    )
    .on_press(Message::ModalClosed)
    .into()
}

fn about() -> Element<'static, Message> {
    let content = column![
        text("Yörü-K İKA Kontrol Sistemi")
            .size(FONT_XL)
            .style(styles::text_primary),
        text("v0.1.0").size(FONT_SM).style(styles::text_tertiary),
        separator(),
        text("Yörü-K diferansiyel sürüşlü İKA kontrol yazılımı")
            .size(FONT_LG)
            .style(styles::text_secondary),
        text(
            "Rust + iced + ONNX Runtime kullanarak Toroslar Atatürk MTAL \
             Yörü-K Teknoloji Takımı tarafından üretilmiştir."
        )
        .size(FONT_MD)
        .style(styles::text_tertiary),
        Space::new().height(SPACE_MD),
        close_button(),
    ]
    .spacing(SPACE_MD)
    .align_x(Alignment::Center);

    panel(content)
}

fn shortcuts() -> Element<'static, Message> {
    let mut content = column![text("Klavye Kısayolları")
        .size(FONT_LG)
        .style(styles::text_primary)]
    .spacing(SPACE_MD)
    .align_x(Alignment::Center);

    content = content.push(separator());

    for (key, description) in SHORTCUTS {
        content = content.push(
            row![
                container(text(key).size(FONT_SM).center().style(styles::text_accent))
                    .width(KEY_WIDTH)
                    .height(KEY_HEIGHT)
                    .style(styles::sunken),
                text(description)
                    .size(FONT_MD)
                    .style(styles::text_secondary),
            ]
            .spacing(SPACE_MD)
            .align_y(Alignment::Center)
            .width(Fill),
        );
    }

    content = content.push(Space::new().height(SPACE_MD));
    content = content.push(close_button());

    panel(content)
}

/// Ortak kart çerçevesi.
///
/// Panelin kendisine yapılan tıklama, arkadaki karartmanın kapatma olayına
/// dönüşmemeli — bu yüzden ayrı bir `mouse_area` ile olay yutulur.
fn panel<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    mouse_area(
        container(content)
            .width(PANEL_WIDTH)
            .padding(SPACE_XL)
            .style(styles::card),
    )
    .into()
}

fn close_button() -> Element<'static, Message> {
    button(text("Kapat").size(FONT_SM).center())
        .width(120.0)
        .height(CONTROL_HEIGHT)
        .style(styles::secondary)
        .on_press(Message::ModalClosed)
        .into()
}
