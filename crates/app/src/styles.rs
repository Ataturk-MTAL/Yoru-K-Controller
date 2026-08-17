//! Widget stilleri — Material 3 rol modeli üzerine kurulu.
//!
//! İki kural taşınıyor:
//!
//! * **Dolgu rengini seçen, üstündeki metni seçmez.** Her dolgu `on_*` eşiyle
//!   birlikte geliyor; kontrast token tarafında garanti altında.
//! * **Durum ayrı renk değil, katman.** Hover/pressed için elle seçilmiş
//!   renkler yerine taban rengin üstüne `on_*` rengi belli bir oranda
//!   karıştırılıyor. Yeni bir varyant eklemek 4 renk değil 2 rol istiyor.

use iced::widget::{button, container, pick_list, text, text_input};
use iced::{Background, Border, Color, Theme};

use crate::theme::{Tokens, RADIUS_LG, RADIUS_MD, RADIUS_SM};

/// Durum katmanı oranları — M3'ün hover %8 / pressed %10 basamakları.
///
/// Bu oranlar keyfi değil, kontrast bütçesi: `state_layer` zemini METİN rengine
/// doğru kaydırıyor, yani katman kalınlaştıkça metin kontrastı düşüyor. Önceki
/// 0.12 / 0.20 değerleri üç butonda WCAG 1.4.3'ün altına iniyordu — açık tema
/// accent hover 3.76:1 / pressed 3.28:1, açık success pressed 3.68:1, koyu
/// success pressed 4.26:1. Yeni oranlarla sırasıyla 4.97 / 4.79, 5.10 ve 5.38.
/// (Açık accent'in kurtulması ayrıca `primary` tonunun bir basamak
/// koyulaşmasını gerektirdi — bkz. `theme::Tokens::light`.)
const HOVER_LAYER: f32 = 0.08;
const PRESSED_LAYER: f32 = 0.10;

/// `on` rengini `base` üzerine verilen oranda karıştırır.
fn state_layer(base: Color, on: Color, amount: f32) -> Color {
    Color {
        r: base.r + (on.r - base.r) * amount,
        g: base.g + (on.g - base.g) * amount,
        b: base.b + (on.b - base.b) * amount,
        a: base.a,
    }
}

// ═══════════════════════════════════════════════════════
//  Container stilleri
// ═══════════════════════════════════════════════════════

/// Pencere zemini.
pub fn page(theme: &Theme) -> container::Style {
    fill(Tokens::for_theme(theme).surface_dim)
}

/// Üst toolbar / alt status bar zemini.
pub fn bar(theme: &Theme) -> container::Style {
    fill(Tokens::for_theme(theme).surface_container)
}

/// Sağ kontrol paneli zemini.
pub fn sidebar(theme: &Theme) -> container::Style {
    fill(Tokens::for_theme(theme).surface_container_low)
}

/// Kart yüzeyi.
pub fn card(theme: &Theme) -> container::Style {
    let t = Tokens::for_theme(theme);
    container::Style {
        background: Some(Background::Color(t.surface_container)),
        border: Border {
            radius: RADIUS_LG.into(),
            ..Border::default()
        },
        ..container::Style::default()
    }
}

/// Girinti yüzeyi — hız çubuğu / joystick halkası zemini.
pub fn sunken(theme: &Theme) -> container::Style {
    let t = Tokens::for_theme(theme);
    container::Style {
        background: Some(Background::Color(t.surface_sunken)),
        border: Border {
            radius: RADIUS_MD.into(),
            ..Border::default()
        },
        ..container::Style::default()
    }
}

/// 1 piksellik ayırıcı çizgi (M3 `outline-variant`).
pub fn hairline(theme: &Theme) -> container::Style {
    fill(Tokens::for_theme(theme).outline_variant)
}

/// Hata rozeti — `error_container` + `on_error_container` çifti.
pub fn error_badge(theme: &Theme) -> container::Style {
    let t = Tokens::for_theme(theme);
    container::Style {
        text_color: Some(t.on_error_container),
        background: Some(Background::Color(t.error_container)),
        border: Border {
            color: Color { a: 0.4, ..t.error },
            width: 1.0,
            radius: RADIUS_SM.into(),
        },
        ..container::Style::default()
    }
}

/// Tema dışı içerik üstündeki çip — kamera rozetleri, harita koordinat kutusu,
/// OSM atıfı, video kontrol çubuğu.
///
/// Modal karartması `scrim` ile aynı şey değil: burada amaç arkayı geri plana
/// atmak değil, üstündeki metni okutmak. Metin rengi de zeminle birlikte
/// veriliyor (`on_overlay`) — container'ın `text_color`'ı çocuklara devrolduğu
/// için çağrı yerinde ayrıca renk vermek gerekmiyor.
///
/// Yarıçap dışarıdan geliyor: aynı zemin hem küçük rozette (RADIUS_SM) hem
/// daha büyük kutularda (RADIUS_MD) kullanılıyor.
pub fn overlay_chip(radius: f32) -> impl Fn(&Theme) -> container::Style {
    move |theme: &Theme| {
        let t = Tokens::for_theme(theme);
        container::Style {
            text_color: Some(t.on_overlay),
            background: Some(Background::Color(t.overlay)),
            border: Border {
                radius: radius.into(),
                ..Border::default()
            },
            ..container::Style::default()
        }
    }
}

fn fill(color: Color) -> container::Style {
    container::Style {
        background: Some(Background::Color(color)),
        ..container::Style::default()
    }
}

// ═══════════════════════════════════════════════════════
//  Buton stilleri
// ═══════════════════════════════════════════════════════

/// Dolgu çiftini temaya göre seçer.
///
/// Koyu temada M3'ün `*-container` deseni kullanılıyor: koyu dolgu + açık
/// yazı. Açık temada rol renkleri zaten koyu, üstlerine beyaz yazılıyor.
/// Böylece iki temada da buton "koyu zemin, açık yazı" ailesinde kalıyor;
/// koyu temada parlak dolgu üstüne koyu yazı yazan varyant tercih edilmedi.
fn role_fill(
    tokens: &Tokens,
    base: Color,
    on_base: Color,
    container: Color,
    on_container: Color,
) -> (Color, Color) {
    if tokens.is_dark {
        (container, on_container)
    } else {
        (base, on_base)
    }
}

/// Birincil dolgu.
///
/// Koyu temada `primary_container`, nötr `control` tonuna çok yakın; birincil
/// eylemin ikincil butonlardan ayrılması için rol renginde bir kenarlık alıyor.
pub fn accent(theme: &Theme, status: button::Status) -> button::Style {
    let t = Tokens::for_theme(theme);
    let (fill, on_fill) = role_fill(
        &t,
        t.primary,
        t.on_primary,
        t.primary_container,
        t.on_primary_container,
    );
    filled(&t, status, fill, on_fill, t.is_dark.then_some(t.primary))
}

/// Nötr, kenarlıklı buton.
pub fn secondary(theme: &Theme, status: button::Status) -> button::Style {
    let t = Tokens::for_theme(theme);
    filled(&t, status, t.control, t.on_control, Some(t.outline))
}

/// Başarı / çalıştır butonu.
pub fn success(theme: &Theme, status: button::Status) -> button::Style {
    let t = Tokens::for_theme(theme);
    let (fill, on_fill) = role_fill(
        &t,
        t.success,
        t.on_success,
        t.success_container,
        t.on_success_container,
    );
    filled(&t, status, fill, on_fill, t.is_dark.then_some(t.success))
}

/// Tehlike / durdur butonu.
pub fn danger(theme: &Theme, status: button::Status) -> button::Style {
    let t = Tokens::for_theme(theme);
    let (fill, on_fill) = role_fill(
        &t,
        t.error,
        t.on_error,
        t.error_container,
        t.on_error_container,
    );
    filled(&t, status, fill, on_fill, t.is_dark.then_some(t.error))
}

/// Zemini olmayan buton — toolbar menü öğeleri.
pub fn ghost(theme: &Theme, status: button::Status) -> button::Style {
    let t = Tokens::for_theme(theme);
    let background = match status {
        button::Status::Hovered | button::Status::Pressed => {
            Some(Background::Color(t.primary_container))
        }
        _ => None,
    };
    let text_color = match status {
        button::Status::Disabled => t.on_surface_disabled,
        button::Status::Hovered | button::Status::Pressed => t.on_primary_container,
        _ => t.on_surface_variant,
    };

    button::Style {
        background,
        text_color,
        border: Border {
            radius: RADIUS_MD.into(),
            ..Border::default()
        },
        ..button::Style::default()
    }
}

/// Aç/kapa butonu (Işık, Fren, Sol Ters, Sağ Ters) — açıkken dolgulu.
///
/// "Seçili/açık = birincil dolgu, değil = nötr kenarlıklı" kuralının tek
/// gövdesi burada; `segment` de buna delege ediyor.
pub fn toggle(on: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme: &Theme, status: button::Status| {
        if on {
            accent(theme, status)
        } else {
            secondary(theme, status)
        }
    }
}

/// Segment seçici (Serial / TCP-IP) — aktif olan dolgulu.
///
/// Görsel olarak `toggle` ile aynı; ad ayrı duruyor çünkü anlamı ayrı. Segment
/// bir kümede karşılıklı dışlayan seçim, toggle bağımsız bir aç/kapa. İkisinin
/// görünümü ileride ayrışırsa (örneğin segmentin ortak çerçeve içinde birleşik
/// çizilmesi) değişiklik yalnızca bu fonksiyona dokunur.
pub fn segment(active: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    toggle(active)
}

/// Motor başlat/durdur.
///
/// Çalışırken `danger`: buton eylemi ("Durdur") anlatıyor, durumu değil.
/// Eskiden `success` (yeşil) kullanılıyordu ve aynı "durdur" eylemi kamerada
/// "■ Durdur", haritada "■ GPS Durdur" ile kırmızıyken burada yeşildi.
/// Motorun ÇALIŞTIĞI bilgisi zaten üç ayrı yerde var: durum çubuğundaki
/// motor noktası + "Motor Çalışıyor" etiketi (view/status_bar.rs), joystick
/// rengi ve hız çubukları. Geri almak isteyen buradaki `danger`'ı `success`
/// yapar; durumu buton renginden okumaya dönmek demek olduğunu bilerek.
pub fn motor(running: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme: &Theme, status: button::Status| {
        if running {
            danger(theme, status)
        } else {
            accent(theme, status)
        }
    }
}

/// Dolgulu buton gövdesi — durumlar taban renkten türetilir.
fn filled(
    tokens: &Tokens,
    status: button::Status,
    base: Color,
    on_base: Color,
    border_color: Option<Color>,
) -> button::Style {
    let (background, text_color) = match status {
        button::Status::Active => (base, on_base),
        button::Status::Hovered => (state_layer(base, on_base, HOVER_LAYER), on_base),
        button::Status::Pressed => (state_layer(base, on_base, PRESSED_LAYER), on_base),
        button::Status::Disabled => (tokens.control_disabled, tokens.on_control_disabled),
    };

    // Pasif buton rol rengini taşımamalı: canlı bir kenarlık, basılamayan
    // butonu hâlâ birincil eylem gibi gösteriyor.
    let border_color = match status {
        button::Status::Disabled => border_color.map(|_| tokens.outline),
        _ => border_color,
    };

    button::Style {
        background: Some(Background::Color(background)),
        text_color,
        border: Border {
            color: border_color.unwrap_or(Color::TRANSPARENT),
            width: if border_color.is_some() { 1.0 } else { 0.0 },
            radius: RADIUS_MD.into(),
        },
        ..button::Style::default()
    }
}

// ═══════════════════════════════════════════════════════
//  Alan stilleri (pick_list / text_input)
// ═══════════════════════════════════════════════════════
//
// Bu iki widget stil verilmediğinde iced'in `Palette`'ten türettiği kendi
// renklerini ve 2.0'lık bir köşe yarıçapını kullanıyor — ikisi de bizim
// ölçeğimizde yok. Sonuç: aynı satırdaki buton ile alan farklı bir tasarım
// dilinden geliyormuş gibi duruyordu. İkisi de aynı üçlüyü kullanır:
// zemin `control`, kenarlık `outline`, metin `on_control` — yani nötr
// kenarlıklı butonla (`secondary`) birebir aynı aile.

/// Seçim kutusu (pick_list) — port ve kamera seçicileri.
///
/// Açık durumda kenarlık `primary`'e geçiyor: liste ekranın başka bir yerinde
/// açılıyor, hangi alandan çıktığı yalnızca bu kenarlıktan anlaşılıyor.
pub fn field(theme: &Theme, status: pick_list::Status) -> pick_list::Style {
    let t = Tokens::for_theme(theme);

    let (background, border_color) = match status {
        pick_list::Status::Active => (t.control, t.outline),
        pick_list::Status::Hovered => {
            (state_layer(t.control, t.on_control, HOVER_LAYER), t.outline)
        }
        pick_list::Status::Opened { .. } => (t.control, t.primary),
    };

    pick_list::Style {
        text_color: t.on_control,
        placeholder_color: t.on_surface_muted,
        // Oku metinden bir ton geri çekiyoruz: değer okunur, gösterge süs.
        handle_color: t.on_surface_variant,
        background: Background::Color(background),
        border: Border {
            color: border_color,
            width: 1.0,
            radius: RADIUS_MD.into(),
        },
    }
}

/// Metin kutusu — host / port alanları.
///
/// Odaklıyken kenarlık `primary`; klavye odağının nerede olduğu tema renginden
/// okunuyor, kalınlık değişmiyor (kalınlaşan kenarlık alanı 1 px oynatır).
pub fn input(theme: &Theme, status: text_input::Status) -> text_input::Style {
    let t = Tokens::for_theme(theme);

    let (background, border_color, value) = match status {
        text_input::Status::Active => (t.control, t.outline, t.on_control),
        text_input::Status::Hovered => (
            state_layer(t.control, t.on_control, HOVER_LAYER),
            t.outline,
            t.on_control,
        ),
        text_input::Status::Focused { .. } => (t.control, t.primary, t.on_control),
        text_input::Status::Disabled => (t.control_disabled, t.outline, t.on_control_disabled),
    };

    text_input::Style {
        background: Background::Color(background),
        border: Border {
            color: border_color,
            width: 1.0,
            radius: RADIUS_MD.into(),
        },
        icon: t.on_surface_variant,
        placeholder: t.on_surface_muted,
        value,
        // Seçim `primary` değil `primary_container`: dolgu rol renginde olursa
        // seçili metin kendi zemininin altında kalıyor.
        selection: t.primary_container,
    }
}

// ═══════════════════════════════════════════════════════
//  Metin stilleri
// ═══════════════════════════════════════════════════════

pub fn text_primary(theme: &Theme) -> text::Style {
    colored(Tokens::for_theme(theme).on_surface)
}

pub fn text_secondary(theme: &Theme) -> text::Style {
    colored(Tokens::for_theme(theme).on_surface_variant)
}

pub fn text_tertiary(theme: &Theme) -> text::Style {
    colored(Tokens::for_theme(theme).on_surface_muted)
}

pub fn text_success(theme: &Theme) -> text::Style {
    colored(Tokens::for_theme(theme).success)
}

pub fn text_warning(theme: &Theme) -> text::Style {
    colored(Tokens::for_theme(theme).warning)
}

pub fn text_error(theme: &Theme) -> text::Style {
    colored(Tokens::for_theme(theme).error)
}

pub fn text_accent(theme: &Theme) -> text::Style {
    colored(Tokens::for_theme(theme).primary)
}

fn colored(color: Color) -> text::Style {
    text::Style { color: Some(color) }
}
