//! Üst toolbar — Slint `app.slint` toolbar'ı + MenuBar öğeleri.
//!
//! iced 0.14'te yerleşik menü çubuğu yok; Slint'teki üç menünün (Bağlantı /
//! Görünüm / Yardım) öğeleri toolbar'ın sağ tarafında düz butonlara taşındı.
//! Görünüm menüsündeki Kamera/Harita zaten ortadaki sekmelerde.

use iced::widget::{button, container, row, text, Space};
use iced::{Alignment, Background, Border, Element, Fill, Font, Theme};

use crate::message::{Message, Modal, Tab};
use crate::state::App;
use crate::styles;
use crate::theme::{
    Tokens, CONTROL_HEIGHT, FONT_LG, FONT_SM, RADIUS_FULL, RADIUS_MD, SPACE_SM, SPACE_XL,
    TOOLBAR_HEIGHT,
};

/// Sekme butonu genişliği (Slint: 96px).
const TAB_WIDTH: f32 = 96.0;
/// Sekme yüksekliği — 40 px'lik toolbar içinde üstte/altta 6 px boşluk kalır.
const TAB_HEIGHT: f32 = CONTROL_HEIGHT;
/// Tema düğmesi ölçüsü (Slint `ThemeToggle`: 52×28 hap).
const TOGGLE_WIDTH: f32 = 52.0;
const TOGGLE_HEIGHT: f32 = CONTROL_HEIGHT;

fn brand_font() -> Font {
    Font {
        weight: iced::font::Weight::Bold,
        ..Font::with_name("Saira")
    }
}

pub fn view(app: &App) -> Element<'_, Message> {
    let content = row![
        text("YÖRÜ-K").size(FONT_LG).font(brand_font()),
        Space::new().width(Fill),
        tab_button("📷 Kamera", Tab::Camera, app.tab),
        tab_button("🛰 Harita", Tab::Map, app.tab),
        Space::new().width(Fill),
        // Emoji karışımı (⌨ ℹ ⏻) satırda farklı ağırlıkta duruyordu; sağ küme
        // düz metne indirildi, tek görsel vurgu sekmelerde kaldı.
        menu_button("Kısayollar", Some(Message::ModalOpened(Modal::Shortcuts))),
        menu_button("Hakkında", Some(Message::ModalOpened(Modal::About))),
        menu_button(
            "Bağlantıyı Kes",
            app.connected.then_some(Message::DisconnectPressed),
        ),
        theme_toggle(app),
    ]
    .spacing(SPACE_SM)
    .align_y(Alignment::Center);

    container(content)
        .height(TOOLBAR_HEIGHT)
        .width(Fill)
        .padding([0.0, SPACE_XL])
        .style(styles::bar)
        .into()
}

/// Ortadaki görünüm sekmesi — aktifken accent zeminli.
fn tab_button<'a>(label: &'a str, tab: Tab, active_tab: Tab) -> Element<'a, Message> {
    let active = tab == active_tab;

    button(text(label).size(FONT_SM).center())
        .width(TAB_WIDTH)
        // Açık yükseklik: 40 px'lik bar içinde sekme dikeyde bara yapışıyordu.
        .height(TAB_HEIGHT)
        .style(move |theme: &Theme, _status| {
            let t = Tokens::for_theme(theme);
            button::Style {
                background: active.then_some(Background::Color(t.primary_container)),
                text_color: if active {
                    t.primary
                } else {
                    t.on_surface_variant
                },
                border: Border {
                    radius: RADIUS_MD.into(),
                    ..Border::default()
                },
                ..button::Style::default()
            }
        })
        .on_press(Message::TabSelected(tab))
        .into()
}

/// Eski menü öğesi — zeminsiz buton.
fn menu_button<'a>(label: &'a str, message: Option<Message>) -> Element<'a, Message> {
    button(text(label).size(FONT_SM).center())
        .height(CONTROL_HEIGHT)
        .style(styles::ghost)
        .on_press_maybe(message)
        .into()
}

/// Slint `ThemeToggle` — koyu/açık geçişi.
///
/// Slint'teki kayan knob'lu hap düğmenin yerine, aynı hap siluetini koruyan
/// ama tek parça olan bir buton: animasyon iced 0.14'te widget başına
/// `Animation` state'i gerektiriyor, o ayrı bir iş.
fn theme_toggle(app: &App) -> Element<'_, Message> {
    button(
        text(if app.is_dark { "🌙" } else { "☀️" })
            .size(FONT_SM)
            .center(),
    )
    .width(TOGGLE_WIDTH)
    .height(TOGGLE_HEIGHT)
    .style(|theme: &Theme, status| {
        let t = Tokens::for_theme(theme);
        let background = match status {
            button::Status::Hovered | button::Status::Pressed => t.control,
            _ => t.control,
        };
        button::Style {
            background: Some(Background::Color(background)),
            text_color: t.on_surface,
            border: Border {
                color: t.outline,
                width: 1.0,
                radius: RADIUS_FULL.into(),
            },
            ..button::Style::default()
        }
    })
    .on_press(Message::ThemeToggled)
    .into()
}
