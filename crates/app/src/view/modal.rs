//! Hakkında / Kısayollar katmanı — Slint `AboutWindow` ve `ShortcutsWindow`
//! karşılığı.
//!
//! Slint'te bunlar ayrı `Window` bileşenleriydi. iced'de ayrı pencere açmak
//! `daemon` mimarisi gerektirir; aynı görsel içerik burada `stack!` üzerine
//! bindirilen bir katmanla veriliyor.

use iced::widget::{button, column, container, mouse_area, row, scrollable, text, Space};
use iced::{Alignment, Background, Element, Fill, Theme};

use crate::message::{Message, Modal};
use crate::styles;
use crate::theme::{self, Tokens, CONTROL_HEIGHT, FONT_MD, FONT_SM, FONT_XL, SPACE_MD, SPACE_XL};
use crate::view::widgets::separator;

/// Katman genişliği (Slint pencere genişlikleri: 380px / 340px).
const PANEL_WIDTH: f32 = 380.0;
/// Kısayol tuş kutusu genişliği.
const KEY_WIDTH: f32 = 96.0;
/// Tuş kutusu yüksekliği.
const KEY_HEIGHT: f32 = 28.0;
/// Kapat butonu genişliği — yükseklik `CONTROL_HEIGHT`, dosyadaki diğer
/// ölçüler gibi adlandırılmış sabitten gelsin diye burada.
const CLOSE_BUTTON_WIDTH: f32 = 120.0;

/// Kısayol listesinin üst yükseklik sınırı.
///
/// Liste dokuz satırdan yirmi ikiye çıktı; sınır olmadan panel en küçük
/// pencerede (768 px) ekranın dışına taşıyor ve `Kapat` düğmesi görünmüyordu.
const SHORTCUT_LIST_MAX_HEIGHT: f32 = 420.0;

/// Klavye kısayolları, işlev kümelerine ayrılmış.
///
/// Düz bir liste dokuz satırda okunuyordu; yirmi ikide okunmuyor. Başlıklar
/// aramayı ikiye indiriyor: önce "hangi küme", sonra "hangi tuş".
///
/// Tuş seçiminin tek kuralı sürüş harflerinden (W/A/S/D) uzak durmak — onlar
/// basılı tutulan tuşlar, üzerlerine ikinci bir anlam binemez.
const SHORTCUT_GROUPS: [(&str, &[(&str, &str)]); 5] = [
    (
        "Sürüş",
        &[
            ("W / ↑", "İleri"),
            ("S / ↓", "Geri"),
            ("A / ←", "Sol dönüş"),
            ("D / →", "Sağ dönüş"),
            ("Space", "Acil durdurma"),
        ],
    ),
    (
        "Motor",
        &[
            ("R", "Motor başlat"),
            ("1 / 2 / 3", "Vites seç"),
            ("[ / ]", "Gönderim aralığı − / +"),
            ("L", "Işık aç/kapat"),
            ("B", "Fren aç/kapat"),
            (", / .", "Sol / Sağ ters çevir"),
        ],
    ),
    (
        "Bağlantı",
        &[
            ("Enter", "Bağlan / Bağlantıyı kes"),
            ("X", "Seri Port ↔ TCP-IP"),
            ("F5", "Port ve kamera listelerini yenile"),
        ],
    ),
    (
        "Kamera ve harita",
        &[
            ("V", "Kamera başlat / durdur"),
            ("N", "Nesne tespiti aç/kapat"),
            ("G", "GPS başlat / durdur"),
            ("+ / −", "Zoom (Harita sekmesinde)"),
        ],
    ),
    (
        "Görünüm",
        &[
            ("C / M", "Kamera / Harita sekmesi"),
            ("T", "Tema değiştir"),
            ("?", "Bu pencere"),
            ("Esc", "Pencereyi kapat"),
        ],
    ),
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
        // Modal başlığı — `shortcuts()` ile aynı rolde olduğu için aynı punto.
        // İki katman aynı yerde, aynı çerçevede açılıyor; başlıkları farklı
        // puntoda olunca ikisi ayrı önemdeymiş gibi okunuyordu.
        text("Yörü-K İKA Kontrol Sistemi")
            .size(FONT_XL)
            .style(styles::text_primary),
        text("v0.1.0").size(FONT_SM).style(styles::text_tertiary),
        separator(),
        // Punto sırası bilgi sırasını izler: başlık (20) → gövde (14) →
        // dipnot (12). Eskiden tek satırlık gövde 16 px ile altındaki
        // paragraftan büyüktü; hiyerarşi tersine dönüyor, göz özeti değil
        // künyeyi son okuyordu.
        text("Yörü-K diferansiyel sürüşlü İKA kontrol yazılımı")
            .size(FONT_MD)
            .style(styles::text_secondary),
        text(
            "Rust + iced + ONNX Runtime kullanarak Toroslar Atatürk MTAL \
             Yörü-K Teknoloji Takımı tarafından üretilmiştir."
        )
        .size(FONT_SM)
        .style(styles::text_tertiary),
        Space::new().height(SPACE_MD),
        close_button(),
    ]
    .spacing(SPACE_MD)
    .align_x(Alignment::Center);

    panel(content)
}

fn shortcuts() -> Element<'static, Message> {
    let mut list = column![].spacing(SPACE_MD).width(Fill);

    for (index, (group, entries)) in SHORTCUT_GROUPS.iter().enumerate() {
        // İlk başlıktan önce ayırıcı yok: onun üstünde zaten modal başlığının
        // ayırıcısı duruyor, iki çizgi üst üste geliyordu.
        if index > 0 {
            list = list.push(separator());
        }
        list = list.push(
            text(*group)
                .size(FONT_SM)
                .style(styles::text_tertiary)
                .width(Fill),
        );
        for (key, description) in entries.iter() {
            list = list.push(shortcut_row(key, description));
        }
    }

    // Başlık puntosu `about()` ile aynı: ikisi de modal başlığı rolünde.
    let content = column![
        text("Klavye Kısayolları")
            .size(FONT_XL)
            .style(styles::text_primary),
        separator(),
        // Liste kaydırılabilir, `Kapat` düğmesi kaydırmanın dışında: düğme
        // listenin sonunda olsaydı, kapatmak için sonuna kadar kaydırmak
        // gerekiyordu.
        container(scrollable(list)).max_height(SHORTCUT_LIST_MAX_HEIGHT),
        Space::new().height(SPACE_MD),
        close_button(),
    ]
    .spacing(SPACE_MD)
    .align_x(Alignment::Center);

    panel(content)
}

/// Tek kısayol satırı — tuş kutusu + açıklama.
fn shortcut_row(key: &'static str, description: &'static str) -> Element<'static, Message> {
    row![
        // Tuşun kendisi listenin tarama hedefi: kullanıcı açıklamayı değil
        // "hangi tuş" sorusunu arıyor. Bu yüzden en az açıklama kadar büyük
        // (FONT_MD) ve sabit genişlikli — tuş adları kutu içinde aynı ızgaraya
        // oturmalı, "W / ↑" ile "1 / 2 / 3" arası oransal fontta her satırda
        // başka yerde bitiyordu.
        container(
            text(key)
                .font(theme::mono())
                .size(FONT_MD)
                .center()
                .style(styles::text_accent)
        )
        .width(KEY_WIDTH)
        .height(KEY_HEIGHT)
        .style(styles::sunken),
        text(description)
            .size(FONT_MD)
            .style(styles::text_secondary),
    ]
    .spacing(SPACE_MD)
    .align_y(Alignment::Center)
    .width(Fill)
    .into()
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
        .width(CLOSE_BUTTON_WIDTH)
        .height(CONTROL_HEIGHT)
        .style(styles::secondary)
        .on_press(Message::ModalClosed)
        .into()
}
