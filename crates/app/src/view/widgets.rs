//! Küçük ortak parçalar — Slint `TStatusDot` / `TSeparator` karşılıkları.
//!
//! iced'de yeniden kullanılabilir bileşen = düz fonksiyon. `Component` trait'i
//! 0.13'te kullanımdan kaldırıldı (gizli state tek doğruluk kaynağını bozuyor).

use iced::widget::{button, container, row, text, Space, Text};
use iced::{Alignment, Background, Border, Color, Element, Theme};

use crate::styles;
use crate::theme::{
    self, CONTROL_HEIGHT, FONT_LABEL, FONT_SM, ICON_BUTTON, RADIUS_FULL, SPACE_MD, SPACE_SM,
};

/// Durum noktası çapı.
///
/// Aynı desen (nokta + etiket) yedi yerde tekrarlanıyordu ve çap yere göre
/// 8 ile 10 arasında geziniyordu; aynı satırda iki farklı çap yan yana
/// düşünce nokta boyutu bilgi taşıyormuş gibi görünüyordu. Tek değer burada.
const STATUS_DOT: f32 = SPACE_MD;

/// Renkli durum noktası — bağlantı / motor / sensör göstergeleri.
pub fn dot<'a, Message: 'a>(color: Color, size: f32) -> Element<'a, Message> {
    container(Space::new())
        .width(size)
        .height(size)
        .style(move |_theme: &Theme| container::Style {
            background: Some(Background::Color(color)),
            border: Border {
                radius: RADIUS_FULL.into(),
                ..Border::default()
            },
            ..container::Style::default()
        })
        .into()
}

/// Panel başlığı — "BAĞLANTI", "MOTOR KONTROL" gibi küçük büyük-harf etiketler.
///
/// Slint'teki `letter-spacing: 1.5px` iced 0.14'te karşılıksız. Etiket o ağırlığı
/// kaybedip gri gürültüye dönüşmesin diye bir punto büyük ve birincil metin
/// tonunda yazılır.
pub fn section_title<'a, Message: 'a>(label: &'a str) -> Element<'a, Message> {
    text(label)
        .size(FONT_LABEL)
        .style(styles::text_primary)
        .into()
}

/// Sayısal okuma — sabit genişlikli fontla, değer değişince hizalama kaymaz.
pub fn numeric<'a>(value: impl text::IntoFragment<'a>) -> Text<'a> {
    text(value).font(theme::mono()).size(FONT_SM)
}

/// Tek ikonluk buton — görünen glif küçük olsa da hedef alanı 40×40.
///
/// `on_press` `None` ise buton pasif çizilir (sınıra dayanmış vites adımı gibi).
pub fn icon_button<'a, Message: Clone + 'a>(
    glyph: &'a str,
    on_press: Option<Message>,
) -> Element<'a, Message> {
    button(text(glyph).size(FONT_SM).center())
        .width(ICON_BUTTON)
        .height(CONTROL_HEIGHT)
        .style(styles::secondary)
        .on_press_maybe(on_press)
        .into()
}

/// Yatay ayırıcı çizgi — Slint `TSeparator`.
pub fn separator<'a, Message: 'a>() -> Element<'a, Message> {
    container(Space::new())
        .width(iced::Fill)
        .height(1.0)
        .style(styles::hairline)
        .into()
}

/// Dikey ayırıcı — status bar bölmeleri arasında.
pub fn v_separator<'a, Message: 'a>(height: f32) -> Element<'a, Message> {
    container(Space::new())
        .width(1.0)
        .height(height)
        .style(styles::hairline)
        .into()
}

/// Nokta + etiket satırı — durum çubuğu ve bağlantı panelindeki göstergeler.
///
/// Nokta çapı ve noktayla etiket arasındaki boşluk burada sabit: yedi çağrı
/// yerinde ayrı ayrı yazıldığında ikisi de kaymıştı. Renk ve metin stili
/// dışarıdan geliyor çünkü hangi rolün geçerli olduğunu (online/offline,
/// uyarı, bekliyor) yalnızca çağıran biliyor.
pub fn status_chip<'a, Message: 'a>(
    color: Color,
    label: String,
    size: f32,
    style: fn(&Theme) -> text::Style,
) -> Element<'a, Message> {
    row![dot(color, STATUS_DOT), text(label).size(size).style(style),]
        .spacing(SPACE_SM)
        .align_y(Alignment::Center)
        .into()
}

/// "Yörü-K bağlı değil" rozeti — kamera ve harita görünümlerinin sol üstünde.
///
/// İki görünümde harfi harfine aynı fonksiyon iki kez yazılıydı. Metin rengi
/// bilerek verilmiyor: `styles::error_badge` zaten `text_color` olarak
/// `on_error_container` veriyor ve container'ın metin rengi çocuklara
/// devroluyor. Çağrı yerlerindeki `styles::text_error` o değeri eziyor ve
/// rozetin kendi zeminine göre seçilmiş kontrastını bozuyordu.
pub fn connection_warning<'a, Message: 'a>() -> Element<'a, Message> {
    container(
        container(text("Yörü-K bağlı değil").size(FONT_SM))
            .padding([SPACE_SM, SPACE_MD])
            .style(styles::error_badge),
    )
    .padding(SPACE_MD)
    .into()
}
