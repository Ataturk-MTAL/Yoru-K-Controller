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

use std::sync::LazyLock;

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

// ── Sabit yerleşim ölçüleri ────────────────────────────
pub const STATUS_BAR_HEIGHT: f32 = 26.0;
pub const SIDEBAR_WIDTH: f32 = 320.0;

/// Sistem başlık çubuğunun içeriğe girdiği bant.
///
/// macOS'ta `fullsize_content_view` içeriği başlık çubuğunun **altına**
/// uzatıyor: pencerenin ilk ~28 px'i hâlâ sistemin başlık bölgesi. Trafik
/// ışıkları orada duruyor ve o bandaki tıklamalar bize gelmiyor — pencereyi
/// sürüklüyor. Yani bu bant çizim için serbest, etkileşim için değil: hiçbir
/// kontrol buraya konulmamalı.
///
/// Diğer platformlarda sistem başlık çubuğu istemci alanının dışında kalıyor,
/// bant sıfır.
#[cfg(target_os = "macos")]
pub const TITLEBAR_INSET: f32 = 28.0;
#[cfg(not(target_os = "macos"))]
pub const TITLEBAR_INSET: f32 = 0.0;

/// Yüzen adanın kendi yüksekliği (kapsül dolgusu dahil).
pub const ISLAND_HEIGHT: f32 = CONTROL_HEIGHT + 2.0 * SPACE_SM;

/// İçerik alanının üstünde adaya ve menü çiplerine ayrılan bant.
///
/// Ada ve çipler `stack!` ile içeriğin ÜSTÜNE biniyor, yani yerleşimde yer
/// kaplamıyorlar (kazanç #1'e gidiyor). Ama altlarındaki katmanların kendi
/// üst köşe overlay'leri var — kamera rozetleri, harita koordinat kutusu,
/// "bağlı değil" uyarısı. Bu sabit onların ne kadar aşağıdan başlayacağını
/// söylüyor; tek bir yerde durması, ada büyüdüğünde dördünün birlikte
/// kaymasını sağlıyor.
pub const TOP_STRIP_HEIGHT: f32 = TITLEBAR_INSET + ISLAND_HEIGHT + SPACE_MD;

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
    /// Bileşen sınırı — buton, `pick_list`, `text_input` kenarlığı.
    ///
    /// WCAG 2.2 SC 1.4.11 bir bileşenin sınırının komşu renge karşı en az
    /// 3:1 tutmasını istiyor. Bu ton o eşiğe göre seçildi; sakin bir gri-mavi
    /// olan eski değerler (koyu `#2e3348`, açık `#d8dae0`) yan panelde 1.41 /
    /// 1.24, kartta 1.33 / 1.40'ta kalıyordu, yani buton üstünde durduğu
    /// yüzeyden gözle ayırt edilmiyordu.
    ///
    /// Muafiyet yolu kasıtlı olarak seçilmedi. SC 1.4.11 metin etiketiyle
    /// tanınabilen bir bileşende kenarlığa kontrast şartı koşmuyor; ama bu
    /// tonu taşıyan kontroller arasında Işık, Fren ve Sol/Sağ Ters var —
    /// kapalı durumdayken hepsi `secondary`. Donanım hareket ettiren bir
    /// anahtarın nerede başlayıp bittiği görünmediğinde, uyum sağlanmış olsa
    /// bile sorun duruyor. `on_control_disabled` için verilen karar da aynı
    /// gerekçeliydi (bkz. aşağısı).
    ///
    /// `accent` / `success` / `danger` bu tondan geçmiyor: dolguları ya da
    /// kendi rol renkli kenarlıkları zaten 3:1'in üstünde. `ghost`'un hiç
    /// kenarlığı yok ve olmamalı — onu metni tanıtıyor, sınırı değil.
    pub outline: Color,
    /// Ayraç çizgileri (M3 `outline-variant`).
    ///
    /// SC 1.4.11 kapsamı dışında: ayraç bir bileşen değil, taşıdığı gruplama
    /// bilgisi bölüm başlıkları ve boşluklarla zaten veriliyor.
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
            // Üçüncü kademe eskiden #636778'di ve 12 px metinde WCAG 1.4.3'ü
            // geçmiyordu: kart üstünde 2.95:1, yan panelde 3.12:1, sayfada
            // 3.36:1 (gereken 4.5). Yedi çağrı yerinin hepsi taşıyıcı metin —
            // "Motoru çalıştırmak için önce robota bağlan", "Bağlantı yok",
            // "Kamera aktif değil". Yeni ton: 5.15 / 5.46 / 5.87. Merdiven
            // monoton kalıyor (`on_surface_variant` kart üstünde 6.36).
            on_surface_muted: hex(0x8b8fa0),
            // `on_control_disabled` ile aynı gerekçe, ama ayrı yüzeyler: bu ton
            // dolgusuz zeminlerde kullanılıyor — toolbar'ın pasif `ghost`
            // düğmesi (`styles.rs:217` → `view/toolbar.rs:93`) ve haritada
            // indirilemeyen tile'ın çaprazı (`view/map_view.rs:248`). Eski
            // #464a5c toolbar üstünde 1.89:1'de kalıyordu; bağlantı yokken
            // "Bağlantıyı Kes" etiketi gözle neredeyse görünmüyordu. Yeni ton:
            // toolbar 3.09:1, tile iskeleti 3.28:1.
            on_surface_disabled: hex(0x656a80),

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
            // Pasif metin eskiden #464a5c ile 1.96:1'de kalıyordu. WCAG pasif
            // bileşenleri muaf tutuyor, ama burada pasif olan çoğu zaman
            // BİRİNCİL eylem: "Motor BAŞLAT" ve "BAĞLAN" bağlantı kurulana
            // kadar bu tonda duruyor. 3:1 alt sınırına çekildi (3.21:1).
            on_control_disabled: hex(0x656a80),

            // Yan panele karşı 3.24:1, karta karşı 3.06:1 (eski #2e3348:
            // 1.41 / 1.33). Ölçüldü, tahmin değil.
            outline: hex(0x606890),
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
            // Koyu temadakiyle aynı gerekçe, ters yönde: eski #8b8fa0 kart
            // üstünde 3.21:1, yan panelde 2.85:1 veriyordu. Yeni ton 5.48 /
            // 4.86. `on_surface_variant` (kart üstünde 6.25) hâlâ bir kademe
            // önde, yani merdiven kademelerini kaybetmedi.
            on_surface_muted: hex(0x656975),
            // Koyu temadakiyle aynı gerekçe: eski #b8bac4 toolbar üstünde
            // 1.93:1, tile iskeleti üstünde 1.71:1. Yeni ton 3.44:1 / 3.05:1 —
            // iki zeminde de 3.0 eşiğini geçen en hafif dokunuş.
            on_surface_disabled: hex(0x868a98),

            // #3b6ee6 tabanda 4.61:1 ile sınırı ancak geçiyordu; durum katmanı
            // eklendiğinde hover 4.00:1 / pressed 3.90:1'e düşüyordu — yani
            // birincil butonun üstüne fare gelince metin standardın altına
            // iniyordu. Bir ton koyu: taban 5.86, hover 4.97, pressed 4.79.
            // `text_accent` de kazanıyor (kart üstünde 4.61 → 5.86).
            primary: hex(0x2f5ecb),
            on_primary: on_filled,
            primary_container: hex(0xe8effc),
            on_primary_container: hex(0x123a80),

            // Aynı sebep: #15794a pressed durumunda 4.4952:1'de kalıyordu
            // (yuvarlamasız karışımla 4.4719:1) — sınırın altı. Yeni ton
            // pressed'de 5.10:1. `motor_running` ve `sensor_online` da aynı
            // değeri taşıyor; üçü tek yeşil olmalı, yoksa aynı "çalışıyor"
            // bilgisi noktada ve butonda iki farklı tonda görünür.
            success: hex(0x136e43),
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
            // Koyu temadakiyle aynı gerekçe: 1.85:1 → 3.12:1.
            on_control_disabled: hex(0x84889a),

            // Yan panele karşı 3.12:1, karta karşı 3.52:1 (eski #d8dae0:
            // 1.24 / 1.40).
            outline: hex(0x84889a),
            outline_variant: hex(0xe8eaee),

            motor_running: hex(0x136e43),
            motor_stopped: hex(0x8b8fa0),
            sensor_online: hex(0x136e43),
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

/// Tema nesneleri bir kez kurulur.
///
/// `App::theme()` her arayüz yenilemesinde çağrılıyor
/// (`iced_winit/src/window/state.rs:219` → `build_user_interfaces`), yani kamera
/// 30 fps çalışırken saniyede ~30 kez. `Theme::custom` her çağrıda bir `String`
/// ve bir `Arc` ayırıyor, üstüne `palette::Extended::generate` beş rol için
/// `mix`/`deviate` hesaplıyor — `deviate` OKLch'e gidip geri dönüyor. `Theme`
/// içi `Arc<Custom>` olduğu için önbellekten `clone()` yalnızca sayaç artırımı.
static DARK: LazyLock<Theme> = LazyLock::new(|| build(DARK_NAME, Tokens::dark()));
static LIGHT: LazyLock<Theme> = LazyLock::new(|| build(LIGHT_NAME, Tokens::light()));

fn build(name: &'static str, t: Tokens) -> Theme {
    Theme::custom(
        name.to_string(),
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

/// Koyu tema — `Palette`, iced'in yerleşik widget'larını (pick_list, slider…) besler.
pub fn dark() -> Theme {
    DARK.clone()
}

/// Açık tema.
pub fn light() -> Theme {
    LIGHT.clone()
}
