//! Yüzen ada + menü çipleri — eski 48 px'lik üst araç çubuğunun yerine.
//!
//! Eskiden pencerenin üstünde iki yatay şerit vardı: işletim sisteminin başlık
//! çubuğu ve hemen altında uygulamanın kendi `toolbar`'ı. Şerit kaldırıldı;
//! içeriği iki yüzen katmana dağıldı ve ikisi de `stack!` ile içeriğin ÜSTÜNE
//! biniyor, yerleşimde yer kaplamıyor.
//!
//! * **Ada** — üstte ortada bir kapsül: marka · Kamera/Harita · tema düğmesi.
//! * **Menü çipleri** — üstte sağda: Kısayollar · Hakkında.
//!
//! `Bağlantıyı Kes` buraya taşınmadı, silindi: aynı komut yan paneldeki
//! bağlantı kartında tam genişlikte bir düğme olarak duruyor ve o kart
//! `scrollable` dışında sabitlenmiş, yani her zaman görünür
//! (`view/mod.rs::sidebar`). Güvenlikle ilgili bir komutun iki farklı yerde
//! iki farklı boyda durması, tek büyük düğmeden daha iyi değil.
//!
//! Dikey konum `TITLEBAR_INSET`'ten başlıyor: macOS'ta pencerenin ilk 28 px'i
//! sistemin başlık bandı ve oradaki tıklamalar bize gelmiyor. Ada o bandın
//! altında duruyor — üstündeki boşluk pencereyi sürüklemeye kalıyor, yani
//! ayrıca bir sürükleme bölgesi yazmamız gerekmiyor ve harita canvas'ının
//! fare olaylarını çalan bir katman doğmuyor.

use iced::widget::{button, container, row, text};
use iced::{Alignment, Border, Element, Fill, Font, Padding, Theme};

use crate::message::{Message, Modal, Tab};
use crate::state::App;
use crate::styles;
use crate::theme::{
    CONTROL_HEIGHT, FONT_LG, FONT_SM, ISLAND_HEIGHT, RADIUS_FULL, SPACE_MD, SPACE_SM, SPACE_XS,
    TITLEBAR_INSET,
};

/// Sekme butonu genişliği (Slint: 96px).
const TAB_WIDTH: f32 = 96.0;
/// Tema düğmesi genişliği (Slint `ThemeToggle`: 52×28 hap).
const TOGGLE_WIDTH: f32 = 52.0;

fn brand_font() -> Font {
    Font {
        weight: iced::font::Weight::Bold,
        ..Font::with_name("Saira")
    }
}

/// Üst-orta ada.
pub fn view(app: &App) -> Element<'_, Message> {
    let content = row![
        // Marka ada içinde: dışarıda kalsaydı macOS'ta tam trafik ışıklarının
        // altına düşerdi (`fullsize_content_view` sol üst köşeyi onlara
        // bırakıyor). Bir kontrol değil, yalnızca etiket — sekmelerin
        // "birbirini dışlayan seçim" okumasını bozmuyor.
        text("YÖRÜ-K").size(FONT_LG).font(brand_font()),
        tab_button("📷 Kamera", Tab::Camera, app.tab),
        tab_button("🛰 Harita", Tab::Map, app.tab),
        theme_toggle(app),
    ]
    .spacing(SPACE_SM)
    .align_y(Alignment::Center);

    container(
        container(content)
            .padding(Padding::new(SPACE_SM).left(SPACE_MD))
            .style(styles::island),
    )
    .center_x(Fill)
    .align_top(Fill)
    .padding(Padding::ZERO.top(TITLEBAR_INSET))
    .into()
}

/// Üst-sağ menü çipleri.
///
/// İkisi de az kullanılan bilgi penceresi; adaya girselerdi ada genişleyip
/// sekme geçişi olmaktan çıkardı. Tek bir `⋯` düğmesine toplamak ise iced
/// 0.14'te hazır menü olmadığı için elle açılır katman yazmak demekti — iki
/// öğe için değmez.
pub fn menu(_app: &App) -> Element<'_, Message> {
    let content = row![
        menu_chip("Kısayollar", Some(Message::ModalOpened(Modal::Shortcuts))),
        menu_chip("Hakkında", Some(Message::ModalOpened(Modal::About))),
    ]
    .spacing(SPACE_XS)
    .align_y(Alignment::Center);

    container(
        container(content)
            .padding(SPACE_XS)
            .height(ISLAND_HEIGHT)
            .style(styles::island),
    )
    .align_right(Fill)
    .align_top(Fill)
    .padding(Padding::new(SPACE_MD).top(TITLEBAR_INSET))
    .into()
}

/// Görünüm sekmesi — aktifken accent zeminli.
///
/// Kamera/Harita karşılıklı dışlayan bir seçim, yani yan paneldeki
/// Serial/TCP-IP seçicisiyle aynı şey; stil de oradan, `styles::segment`'ten
/// geliyor.
fn tab_button<'a>(label: &'a str, tab: Tab, active_tab: Tab) -> Element<'a, Message> {
    button(text(label).size(FONT_SM).center())
        .width(TAB_WIDTH)
        .height(CONTROL_HEIGHT)
        .style(styles::segment(tab == active_tab))
        .on_press(Message::TabSelected(tab))
        .into()
}

/// Zeminsiz menü öğesi.
///
/// Yüksekliği adanın iç dolgusu kadar kısaltılmış: çip kapsülü
/// `ISLAND_HEIGHT` ve içindeki düğme tam `CONTROL_HEIGHT` olsaydı kapsülün
/// kenarına yapışırdı.
fn menu_chip<'a>(label: &'a str, message: Option<Message>) -> Element<'a, Message> {
    button(text(label).size(FONT_SM).center())
        .height(CONTROL_HEIGHT - 2.0 * SPACE_XS)
        .style(styles::ghost)
        .on_press_maybe(message)
        .into()
}

/// Koyu/açık tema geçişi.
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
    .height(CONTROL_HEIGHT)
    // Gövde `secondary`'den geliyor, burada yalnızca hap silueti için yarıçap
    // eziliyor.
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
