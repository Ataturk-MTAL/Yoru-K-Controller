//! Renk ve ölçü token'ları — Material 3 rol modeline hizalanmış hâli.
//!
//! Kaynak palet Slint `crates/app/ui/theme/theme.slint` dosyasından geldi;
//! yapı ise M3'ün rol sisteminden:
//!
//! * **Her dolgu rolünün bir `on_*` eşi var.** Renkli bir zemin kullanan her
//!   yerde üstüne gelecek metin rengi tahmine bırakılmıyor.
//! * **`*_container` aileleri** yumuşak tint yüzeyler için (rozet, uyarı şeridi).
//! * **Yüzey merdiveni** M3 sözlüğünde: `surface_dim` → `surface_container_*`.
//!   Derinlik gölgeyle değil tonla kuruluyor.
//!
//! Açık temada rol renkleri bilerek koyu tonlardan seçildi (M3'ün açık şemada
//! koyu dolgu tercihi): aynı değer hem yüzey üstünde okunur metin, hem de
//! beyaz yazılı dolgu olarak kontrastı karşılıyor.

use iced::theme::Palette;
use iced::{Color, Font, Theme};

/// Sayısal okumalar için sabit genişlikli font.
///
/// FPS, hız, koordinat gibi sürekli değişen değerlerde rakam genişliği sabit
/// kalmazsa metin her karede yatayda zıplar — CSS'teki `tabular-nums`'ın
/// karşılığı burada Space Mono'ya geçmek.
pub fn mono() -> Font {
    Font::with_name("Space Mono")
}

// ── Dokunma hedefleri ───────────────────────────────────
/// İkon butonu kenarı — 40×40 alt sınır.
///
/// M3 mobilde 48 dp ister; bu masaüstü konsolunda işaretçi hassasiyeti daha
/// yüksek ve 320 px'lik yan panel 48'lik hedeflerle şişerdi.
pub const ICON_BUTTON: f32 = 40.0;

/// **Standart kontrol yüksekliği.** Buton, metin kutusu ve seçim kutusu —
/// hepsi bu yüksekliği kullanır. Aksi açıkça istenmedikçe sapma yok.
pub const CONTROL_HEIGHT: f32 = 36.0;
/// `text_input` ve `pick_list` iced 0.14'te `height()` sunmuyor; standart
/// yüksekliğe dikey dolguyla ulaşılıyor (12 px metin + 2×10 ≈ 36 px).
pub const FIELD_PADDING_Y: f32 = 10.0;
pub const FIELD_PADDING_X: f32 = 12.0;

// ── Boşluk (spacing) ────────────────────────────────────
/// Slint ölçeğinin tamamı korunuyor; bu adım şu an hiçbir yerde kullanılmıyor.
#[allow(dead_code)]
pub const SPACE_XS: f32 = 2.0;
pub const SPACE_SM: f32 = 4.0;
pub const SPACE_MD: f32 = 8.0;
pub const SPACE_LG: f32 = 12.0;
pub const SPACE_XL: f32 = 16.0;

// ── Yuvarlatma (radius) ─────────────────────────────────
pub const RADIUS_SM: f32 = 4.0;
pub const RADIUS_MD: f32 = 6.0;
pub const RADIUS_LG: f32 = 8.0;
pub const RADIUS_FULL: f32 = 999.0;

// ── Tipografi ───────────────────────────────────────────
pub const FONT_XS: f32 = 10.0;
/// Bölüm etiketleri — Slint'teki `letter-spacing: 1.5px` iced'de yok;
/// kaybolan görsel ağırlık bir punto büyüterek telafi ediliyor.
pub const FONT_LABEL: f32 = 11.0;
pub const FONT_SM: f32 = 12.0;
pub const FONT_MD: f32 = 14.0;
pub const FONT_LG: f32 = 16.0;
pub const FONT_XL: f32 = 20.0;

// ── Sabit yerleşim ölçüleri (Slint app.slint ile aynı) ──
/// Toolbar — standart yükseklikteki kontroller sığsın diye Slint'teki 40 px'ten
/// büyütüldü (36 px kontrol + üstte/altta 6 px pay).
pub const TOOLBAR_HEIGHT: f32 = 48.0;
pub const STATUS_BAR_HEIGHT: f32 = 26.0;
pub const SIDEBAR_WIDTH: f32 = 320.0;

/// Tema adları.
pub const DARK_NAME: &str = "Yörü-K Dark";
pub const LIGHT_NAME: &str = "Yörü-K Light";

fn hex(value: u32) -> Color {
    Color::from_rgb8(
        ((value >> 16) & 0xFF) as u8,
        ((value >> 8) & 0xFF) as u8,
        (value & 0xFF) as u8,
    )
}

fn hexa(value: u32, alpha: f32) -> Color {
    Color {
        a: alpha,
        ..hex(value)
    }
}

/// Uygulamanın renk rolleri.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct Tokens {
    pub is_dark: bool,

    // ── Yüzey merdiveni (M3 surface tiers) ──────────────
    /// Pencere zemini.
    pub surface_dim: Color,
    /// Yan panel gibi ikincil bölgeler.
    pub surface_container_low: Color,
    /// Kart, toolbar, durum çubuğu.
    ///
    /// Açık temada bu ton beyaz, yani `surface_container_low` yan panelinden
    /// daha AÇIK: merdivenin bu basamağındaki ters yön bilinçli. M3 açık şemada
    /// kartı "kağıt" gibi düşünür — kart panelden yükselir, panele gömülmez.
    pub surface_container: Color,
    /// Bir basamak yukarısı.
    pub surface_container_high: Color,
    /// Üste binen yüzeyler.
    pub surface_container_highest: Color,
    /// Girinti — M3'te doğrudan karşılığı yok; joystick halkası, hız track'i.
    pub surface_sunken: Color,

    pub on_surface: Color,
    pub on_surface_variant: Color,
    pub on_surface_muted: Color,
    pub on_surface_disabled: Color,

    // ── Roller ──────────────────────────────────────────
    pub primary: Color,
    pub on_primary: Color,
    pub primary_container: Color,
    pub on_primary_container: Color,

    pub success: Color,
    pub on_success: Color,
    pub success_container: Color,
    pub on_success_container: Color,

    pub warning: Color,
    pub on_warning: Color,
    pub warning_container: Color,
    pub on_warning_container: Color,

    pub error: Color,
    pub on_error: Color,
    pub error_container: Color,
    pub on_error_container: Color,

    pub info: Color,

    // ── Nötr kontroller ─────────────────────────────────
    pub control: Color,
    pub on_control: Color,
    pub control_disabled: Color,
    pub on_control_disabled: Color,

    // ── Kenarlıklar ─────────────────────────────────────
    pub outline: Color,
    pub outline_strong: Color,
    /// Ayraç çizgileri (M3 `outline-variant`).
    pub outline_variant: Color,

    // ── Endüstriyel / IoT ───────────────────────────────
    pub motor_running: Color,
    pub motor_stopped: Color,
    pub sensor_online: Color,
    pub sensor_offline: Color,
    pub sensor_warning: Color,

    /// Modal karartması — arkadaki arayüzü pasifleştirmek için.
    ///
    /// Yalnızca modalın kendi karartma katmanı bunu kullanır; tema dışı içerik
    /// (kamera karesi, harita tile'ı) üstündeki çipler `overlay` kullanır.
    pub scrim: Color,

    /// Tema dışı içerik üstündeki çip zemini (kamera karesi, harita tile'ı).
    ///
    /// `scrim`'den ayrı bir rol: karartmanın işi arkayı geri plana atmak,
    /// bunun işi ÜSTÜNDEKİ METNİ okutmak. Açık temada `scrim` 0.33 alfa —
    /// parlak bir kamera karesinin üstünde çipin var olma sebebini
    /// karşılamıyor. Bu yüzden `overlay` iki temada da koyu ve daha opak:
    /// altındaki görüntü tema seçiminden bağımsız, dolayısıyla rol de öyle.
    pub overlay: Color,
    /// `overlay` üstündeki metin — iki temada da açık ton.
    pub on_overlay: Color,
}

impl Tokens {
    pub fn dark() -> Self {
        // Koyu temada dolgular açık tonlu, üzerlerindeki içerik koyu — M3'ün
        // koyu şemada açık dolgu / koyu içerik yaklaşımı. Eskiden bu dolguların
        // üstüne beyaz yazılıyordu ve kontrast zayıf kalıyordu.
        let on_filled = hex(0x0f1117);

        Self {
            is_dark: true,

            surface_dim: hex(0x0f1117),
            surface_container_low: hex(0x161923),
            surface_container: hex(0x1a1e2e),
            surface_container_high: hex(0x1c2030),
            surface_container_highest: hex(0x2a2f44),
            surface_sunken: hex(0x0b0d14),

            on_surface: hex(0xe8eaf0),
            on_surface_variant: hex(0x9ca0b0),
            on_surface_muted: hex(0x636778),
            on_surface_disabled: hex(0x464a5c),

            // Container tonları bilerek doygun: bunlar dolgulu butonların
            // zemini ve `surface_container` (#1a1e2e) ile `control` (#222638)
            // arasında kaybolmamaları gerekiyor. İlk seçilen soluk tonlar
            // (#1e2a4a / #132e22 / #301818) panelden ayırt edilemiyordu.
            primary: hex(0x5b8af5),
            on_primary: on_filled,
            primary_container: hex(0x1f3a7a),
            on_primary_container: hex(0xcfdcff),

            success: hex(0x3dd68c),
            on_success: on_filled,
            success_container: hex(0x14523a),
            on_success_container: hex(0xa6ecc8),

            warning: hex(0xf5b731),
            on_warning: on_filled,
            warning_container: hex(0x5a4110),
            on_warning_container: hex(0xffd98a),

            error: hex(0xf06565),
            on_error: on_filled,
            error_container: hex(0x6b1f22),
            on_error_container: hex(0xffc9c9),

            info: hex(0x40b8e0),

            control: hex(0x222638),
            on_control: hex(0xd0d3de),
            control_disabled: hex(0x181b26),
            on_control_disabled: hex(0x464a5c),

            outline: hex(0x2e3348),
            outline_strong: hex(0x3e4460),
            outline_variant: hex(0x222638),

            motor_running: hex(0x3dd68c),
            motor_stopped: hex(0x636778),
            sensor_online: hex(0x3dd68c),
            sensor_offline: hex(0xf06565),
            sensor_warning: hex(0xf5b731),

            scrim: hexa(0x000000, 0.67),

            overlay: hexa(0x000000, 0.72),
            on_overlay: hex(0xf2f4f8),
        }
    }

    pub fn light() -> Self {
        // Açık temada rol renkleri koyu seçiliyor: aynı değer hem yüzey
        // üstünde okunur metin hem de beyaz yazılı dolgu olarak çalışıyor.
        let on_filled = Color::WHITE;

        Self {
            is_dark: false,

            surface_dim: hex(0xf8f9fc),
            surface_container_low: hex(0xf0f1f5),
            surface_container: Color::WHITE,
            surface_container_high: hex(0xe6e8ee),
            // Merdiven yukarı doğru koyulaşmalı: eski değer (#f4f5f7) high'tan
            // daha açıktı, yani "highest" adı yön değiştiriyordu. İki token da
            // henüz hiçbir yerde kullanılmıyor — bu düzeltme görünen hiçbir
            // yüzeyi değiştirmiyor, yalnızca merdivenin tanımını doğruluyor.
            surface_container_highest: hex(0xd9dce4),
            surface_sunken: hex(0xedeef2),

            on_surface: hex(0x1a1c24),
            on_surface_variant: hex(0x5c6070),
            on_surface_muted: hex(0x8b8fa0),
            on_surface_disabled: hex(0xb8bac4),

            primary: hex(0x3b6ee6),
            on_primary: on_filled,
            primary_container: hex(0xe8effc),
            on_primary_container: hex(0x123a80),

            success: hex(0x15794a),
            on_success: on_filled,
            success_container: hex(0xe6f7ef),
            on_success_container: hex(0x0a4b2c),

            warning: hex(0x8a6200),
            on_warning: on_filled,
            warning_container: hex(0xfdf4e0),
            on_warning_container: hex(0x4a3400),

            error: hex(0xa71d2a),
            on_error: on_filled,
            error_container: hex(0xfce8ea),
            on_error_container: hex(0x6b1119),

            info: hex(0x106d94),

            // Nötr dolgu beyaz olamaz: `surface_container` da beyaz olduğu için
            // toolbar'daki tema düğmesi ve modalın kapat butonu 1.00:1 ile
            // zeminde kayboluyordu. Bu gri hem beyazdan (1.29:1) hem de yan
            // panel tonu #f0f1f5'ten (1.14:1) ayrılıyor; `on_control` ile
            // kontrastı 10.4:1, yani metin tarafında hiçbir şey feda edilmiyor.
            control: hex(0xdfe3ea),
            on_control: hex(0x2c2f3a),
            control_disabled: hex(0xf0f1f5),
            on_control_disabled: hex(0xb0b3be),

            outline: hex(0xd8dae0),
            outline_strong: hex(0xc0c3cc),
            outline_variant: hex(0xe8eaee),

            motor_running: hex(0x15794a),
            motor_stopped: hex(0x8b8fa0),
            sensor_online: hex(0x15794a),
            sensor_offline: hex(0xa71d2a),
            sensor_warning: hex(0x8a6200),

            scrim: hexa(0x000000, 0.33),

            // Açık temada da koyu: altındaki kamera karesi / harita tile'ı
            // temadan bağımsız, bu yüzden çip zemini tema ile açılmıyor.
            overlay: hexa(0x000000, 0.72),
            on_overlay: hex(0xf2f4f8),
        }
    }

    /// Aktif temadan token setini türetir.
    ///
    /// Stil closure'ları yalnızca `&Theme` alır; hangi paletin geçerli olduğunu
    /// arka plan renginin parlaklığından anlıyoruz.
    pub fn for_theme(theme: &Theme) -> Self {
        if theme.palette().background.r < 0.5 {
            Self::dark()
        } else {
            Self::light()
        }
    }
}

/// Koyu tema — `Palette`, iced'in yerleşik widget'larını (pick_list, slider…) besler.
pub fn dark() -> Theme {
    let t = Tokens::dark();
    Theme::custom(
        DARK_NAME.to_string(),
        Palette {
            background: t.surface_dim,
            text: t.on_surface,
            primary: t.primary,
            success: t.success,
            warning: t.warning,
            danger: t.error,
        },
    )
}

/// Açık tema.
pub fn light() -> Theme {
    let t = Tokens::light();
    Theme::custom(
        LIGHT_NAME.to_string(),
        Palette {
            background: t.surface_dim,
            text: t.on_surface,
            primary: t.primary,
            success: t.success,
            warning: t.warning,
            danger: t.error,
        },
    )
}
