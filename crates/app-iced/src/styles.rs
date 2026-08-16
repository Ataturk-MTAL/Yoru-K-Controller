//! Widget stilleri — Material 3 rol modeli üzerine kurulu.
//!
//! İki kural taşınıyor:
//!
//! * **Dolgu rengini seçen, üstündeki metni seçmez.** Her dolgu `on_*` eşiyle
//!   birlikte geliyor; kontrast token tarafında garanti altında.
//! * **Durum ayrı renk değil, katman.** Hover/pressed için elle seçilmiş
//!   renkler yerine taban rengin üstüne `on_*` rengi belli bir oranda
//!   karıştırılıyor. Yeni bir varyant eklemek 4 renk değil 2 rol istiyor.

use iced::widget::{button, container, text};
use iced::{Background, Border, Color, Theme};

use crate::theme::{Tokens, RADIUS_LG, RADIUS_MD, RADIUS_SM};

/// Durum katmanı oranları.
///
/// M3 durumları bir opaklık katmanıyla anlatıyor; spec'teki tam yüzdeler
/// doğrulanamadı (m3.material.io sayfaları JS ile çiziliyor), bu yüzden
/// aşağıdaki değerler bu uygulamaya ait.
const HOVER_LAYER: f32 = 0.12;
const PRESSED_LAYER: f32 = 0.20;

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

/// Segment seçici (Serial / TCP-IP) — aktif olan dolgulu.
pub fn segment(active: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme: &Theme, status: button::Status| {
        if active {
            accent(theme, status)
        } else {
            secondary(theme, status)
        }
    }
}

/// Aç/kapa butonu (Işık, Fren, Sol Ters, Sağ Ters).
pub fn toggle(on: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme: &Theme, status: button::Status| {
        if on {
            accent(theme, status)
        } else {
            secondary(theme, status)
        }
    }
}

/// Motor başlat/durdur — çalışırken yeşil, dururken birincil renk.
pub fn motor(running: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme: &Theme, status: button::Status| {
        if running {
            success(theme, status)
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
