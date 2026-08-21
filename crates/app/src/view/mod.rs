//! Kök görünüm.
//!
//! Yerleşim: solda kamera/harita alanı, sağda 320 px kontrol paneli, altta
//! 26 px durum çubuğu. Üstte artık bir şerit YOK — eski 48 px'lik araç çubuğu
//! kalktı, içeriği kamera/harita alanının üstünde yüzen bir adaya ve iki menü
//! çipine taşındı (`island`). macOS'ta bunların üstündeki bant doğrudan
//! sistemin başlık çubuğu; `main.rs` içerik alanını oraya kadar uzatıyor.
//!
//! Alt durum çubuğu kaldırılmadı. İstek "tüm toolbarlar" diyordu ama bu bir
//! araç çubuğu değil: bağlantı durumu, motor/vites, ışık/fren, GPS ve hata
//! dizgeleri (`status_text`) yalnızca orada görünüyor. Hareket eden bir aracın
//! kumandasında bu okumaları kamera görüntüsünün üstüne yüzen çiplere
//! taşımak, kontrastı garanti olmayan bir zeminde güvenlik bilgisi göstermek
//! olurdu.

mod camera_view;
mod connection_panel;
mod island;
pub mod joystick;
pub mod map_view;
mod modal;
mod motor_control;
mod speed_display;
mod status_bar;
mod widgets;

use iced::widget::{canvas, column, container, row, scrollable, stack, Container};
use iced::{Element, Fill, Padding};

use crate::message::{Message, Tab};
use crate::state::App;
use crate::styles;
use crate::theme::{SIDEBAR_WIDTH, SPACE_LG, SPACE_MD, SPACE_XL, TITLEBAR_INSET};

use joystick::{JoystickCanvas, JoystickKey};
use widgets::{section_title, separator};

/// Joystick kartının yüksekliği.
///
/// Slint'te 220 px'ti. Halka, tam sapmada taşan bilek yarısı kırpılmasın diye
/// bilek yarıçapı kadar içeri alınıyor; kart bir miktar büyütülerek halka
/// çapı ve gezinme aralığı korunuyor.
const JOYSTICK_HEIGHT: f32 = 252.0;

pub fn view(app: &App) -> Element<'_, Message> {
    let layout = column![
        row![main_pane(app), sidebar(app)].height(Fill),
        separator(),
        status_bar::view(app),
    ];

    let base = container(layout)
        .width(Fill)
        .height(Fill)
        .style(styles::page);

    match app.modal {
        Some(active) => stack![base, modal::view(active)].into(),
        None => base.into(),
    }
}

/// Sol taraf — seçili sekmeye göre kamera ya da harita, üstünde ada.
///
/// Ada ve menü çipleri `stack!` katmanı: yerleşimde yer kaplamadıkları için
/// eski araç çubuğunun 48 px'i doğrudan içeriğe kalıyor. Alttaki görünümlerin
/// kendi üst köşe overlay'leri `theme::TOP_STRIP_HEIGHT` kadar aşağıdan
/// başlıyor, yoksa adanın altında kalırlardı.
fn main_pane(app: &App) -> Element<'_, Message> {
    let content = match app.tab {
        Tab::Camera => camera_view::view(app),
        Tab::Map => map_view::view(app),
    };

    stack![
        container(content)
            .width(Fill)
            .height(Fill)
            .style(styles::page),
        island::view(app),
        island::menu(app),
    ]
    .into()
}

/// Yan panel bölümünün kart çerçevesi.
///
/// Dört bölüm (bağlantı, motor, joystick, hız) tek çerçeve dilini paylaşır:
/// hepsi `styles::card` üstünde aynı `SPACE_LG` iç dolgusuyla çizilir. Bu
/// yardımcı olmadan iki bölüm çıplak, ikisi kart olarak duruyordu ve bunun
/// görünen bedeli hizalamaydı: kartın iç dolgusu başlığı `SPACE_LG` kadar daha
/// içeri kaydırdığı için "BAĞLANTI"/"MOTOR KONTROL" ile
/// "SÜRÜŞ KUMANDA KOLU"/"HIZ GÖSTERGESİ" iki ayrı sol hizada başlıyordu.
///
/// Ortak dil "çıplak + ayırıcı" değil "kart" seçildi çünkü seçim zaten yarı
/// yarıya yapılmıştı: kartlı iki bölüm hem içerik olarak ağır (canvas, çubuk
/// grafiği) hem de kendi zeminine ihtiyaç duyuyor — joystick halkası ve hız
/// track'i `surface_sunken` kullanıyor, girinti tonunun bir yüzeyden çökmesi
/// gerekiyor, yan panel zemini o yüzey değil.
///
/// `Container` döner (Element değil): joystick kartı üstüne bir de sabit
/// yükseklik ekliyor.
fn section_card<'a>(content: impl Into<Element<'a, Message>>) -> Container<'a, Message> {
    container(content)
        .width(Fill)
        .padding(SPACE_LG)
        .style(styles::card)
}

/// Sağ kontrol paneli.
///
/// Panel üç bölgeli: tepede sabit bağlantı kartı, ortada kaydırılabilir
/// ayarlar, altta sabit hız göstergesi. İki uç da bilinçli olarak kaydırmanın
/// dışında.
///
/// Bağlantı — her şeyin ön koşulu — daima görünür kalıyor ve açılır listeli
/// tek kontrol `scrollable` dışında kalıyor.
///
/// Hız göstergesi de sabit, çünkü o bir **canlı okuma**: sürüş sırasında
/// bakılan sayı. Kaydırmanın içindeyken 1280×948 pencerede katlamanın altında
/// kalıyordu — "Sol" satırı yarım, "Sağ" satırı hiç görünmüyordu. Üst araç
/// çubuğunun kalkması (bkz. modül başlığı) 48 px kazandırdı ama sorunu
/// yapısal olarak çözmedi: en küçük pencerede (1024×768) yine katlanırdı.
/// Kaydırma artık yalnızca ayarlar için; sürüş sırasında okunan hiçbir şey
/// için değil.
///
/// İkincisi bir çizim kusurunu kapatıyor: `scrollable` içindeki `pick_list`,
/// açılır ok glifini kaydırma katmanının dışına, sol panelin üstüne de
/// çiziyor (iced 0.14.2'de gözlendi).
///
/// Panelin kenar dolgusu `SPACE_XL`: toolbar ve durum çubuğu da yatayda bu
/// değeri kullanıyor, panel ise `SPACE_LG` kullanıyordu — pencerenin sağ
/// kenarında üç yüzey arasında 12/16 px'lik bir basamak görünüyordu.
///
/// Panelde tek ayırıcı çizgi kaldı ve o da sabit/kaydırılan sınırını
/// işaretliyor, yani panelin tam genişliğini kaplaması doğru. Bölüm araları
/// ayırıcı yerine kart kenarlarıyla okunuyor; ayırıcı orada kalsaydı biri
/// panel genişliğinde (320 px), diğeri dolgu içinde (296 px) olan iki farklı
/// uzunlukta çizgi yan yana düşüyordu.
fn sidebar(app: &App) -> Element<'_, Message> {
    let scrolling = column![section_card(motor_control::view(app)), joystick_card(app),]
        .spacing(SPACE_LG)
        .padding(SPACE_XL);

    let panel = column![
        container(section_card(connection_panel::view(app)))
            .width(Fill)
            // Üstte `TITLEBAR_INSET` kadar fazladan pay: macOS'ta içerik
            // alanı sistemin başlık bandının altına uzuyor ve o banttaki
            // tıklamalar pencereyi sürüklüyor. Pay olmadan bağlantı kartının
            // üst kenarı basılamaz bir şeride denk gelirdi.
            .padding(Padding::new(SPACE_XL).top(SPACE_XL + TITLEBAR_INSET)),
        separator(),
        scrollable(scrolling).height(Fill),
        separator(),
        // Hız göstergesi kart çerçevesini kendi dosyasında kuruyor (birebir
        // aynı `SPACE_LG` + `styles::card`); burada ikinci kez sarmalamak
        // çerçeveyi ve dolguyu çiftlerdi.
        container(speed_display::view(app))
            .width(Fill)
            .padding(SPACE_XL),
    ];

    container(panel)
        .width(SIDEBAR_WIDTH)
        .height(Fill)
        .style(styles::sidebar)
        .into()
}

/// Joystick kartı — başlık + canvas.
fn joystick_card(app: &App) -> Element<'_, Message> {
    // Anahtar önce: `Keyed::get` durum değiştiyse önbelleği düşürür.
    let key = JoystickKey::new(app.joystick, app.connected, app.motor_running, app.is_dark);

    let program = JoystickCanvas {
        joystick: app.joystick,
        connected: app.connected,
        motor_running: app.motor_running,
        cache: app.joystick_cache.get(key),
    };

    let content = column![
        section_title("SÜRÜŞ KUMANDA KOLU"),
        canvas(program).width(Fill).height(Fill),
    ]
    .spacing(SPACE_MD);

    // Tek kartın sabit yüksekliği var: canvas kendi başına büyümek istemez,
    // halkanın çapını kartın yüksekliği belirliyor.
    section_card(content).height(JOYSTICK_HEIGHT).into()
}
