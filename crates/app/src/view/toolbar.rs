//! Üst toolbar — Slint `app.slint` toolbar'ı + MenuBar öğeleri.
//!
//! iced 0.14'te yerleşik menü çubuğu yok; Slint'teki üç menünün (Bağlantı /
//! Görünüm / Yardım) öğeleri toolbar'ın sağ tarafında düz butonlara taşındı.
//! Görünüm menüsündeki Kamera/Harita zaten ortadaki sekmelerde.

use iced::widget::{button, container, row, text, Space};
use iced::{Alignment, Border, Element, Fill, Font, Theme};

use crate::message::{Message, Modal, Tab};
use crate::state::App;
use crate::styles;
use crate::theme::{
    CONTROL_HEIGHT, FONT_LG, FONT_SM, RADIUS_FULL, SPACE_SM, SPACE_XL, TOOLBAR_HEIGHT,
};

/// Sekme butonu genişliği (Slint: 96px).
const TAB_WIDTH: f32 = 96.0;
/// Sekme yüksekliği — standart kontrol ölçüsü.
///
/// Slint'teki bar 40 px'ti ve sekme dikeyde bara yapışıyordu; bar
/// `TOOLBAR_HEIGHT` (48 px) olduğundan beri aynı sekme üstte/altta 6 px pay
/// bırakıyor.
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
///
/// Kamera/Harita karşılıklı dışlayan bir seçim, yani yan paneldeki
/// Serial/TCP-IP seçicisiyle aynı şey; stil de oradan, `styles::segment`'ten
/// geliyor. Kazancı üç yerde:
///
/// * Hover/pressed artık var — buradaki eski closure `button::Status`'u hiç
///   okumuyordu, sekmeler uygulamadaki geri bildirimsiz tek kontroldü.
/// * Aktif etiket dolgunun `on_*` eşini alıyor. Eskiden `primary_container`
///   zeminin üstüne `primary` yazılıyordu: koyu 3.32:1, açık 3.99:1 — ikisi de
///   AA (4.5:1) altında.
/// * Pasif sekme nötr `control` dolgusuna düşüyor. `ghost` verilemezdi: onun
///   hover zemini de `primary_container`, yani üzerine gelinen pasif sekme
///   aktif sekmeden ayırt edilemez olurdu.
fn tab_button<'a>(label: &'a str, tab: Tab, active_tab: Tab) -> Element<'a, Message> {
    button(text(label).size(FONT_SM).center())
        .width(TAB_WIDTH)
        // Açık yükseklik: sekme yalnız metin kadar kalırsa dikeyde bara
        // yapışıyor; `TAB_HEIGHT` bar içinde payı garanti ediyor.
        .height(TAB_HEIGHT)
        .style(styles::segment(tab == active_tab))
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
    // Gövde `secondary`'den geliyor, burada yalnızca hap silueti için yarıçap
    // eziliyor. Eskiden bu buton kendi `match`'ini yazıyordu ve Hovered/Pressed
    // kolu `_` kolu ile aynı rengi döndürdüğü için uygulamada geri bildirimi
    // olmayan tek butondu; metin de `control` dolgusunun üstünde `on_surface`
    // ile yazılıyordu, dolgunun `on_*` eşi (`on_control`) ile değil.
    .style(|theme: &Theme, status| {
        let base = styles::secondary(theme, status);
        button::Style {
            border: Border {
                radius: RADIUS_FULL.into(),
                ..base.border
            },
            ..base
        }
    })
    .on_press(Message::ThemeToggled)
    .into()
}
