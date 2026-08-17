//! Kök görünüm — Slint `app.slint` `AppWindow` yerleşiminin karşılığı.
//!
//! Yerleşim aynen korundu: üstte 40 px toolbar, ortada solda kamera/harita
//! alanı ve sağda 320 px kontrol paneli, altta 26 px durum çubuğu.

mod camera_view;
mod connection_panel;
mod joystick;
mod map_view;
mod modal;
mod motor_control;
mod speed_display;
mod status_bar;
mod toolbar;
mod widgets;

use iced::widget::{canvas, column, container, row, scrollable, stack, Container};
use iced::{Element, Fill};

use crate::message::{Message, Tab};
use crate::state::App;
use crate::styles;
use crate::theme::{SIDEBAR_WIDTH, SPACE_LG, SPACE_MD, SPACE_XL};

use joystick::JoystickCanvas;
use widgets::{section_title, separator};

/// Joystick kartının yüksekliği.
///
/// Slint'te 220 px'ti. Halka, tam sapmada taşan bilek yarısı kırpılmasın diye
/// bilek yarıçapı kadar içeri alınıyor; kart bir miktar büyütülerek halka
/// çapı ve gezinme aralığı korunuyor.
const JOYSTICK_HEIGHT: f32 = 252.0;

pub fn view(app: &App) -> Element<'_, Message> {
    let layout = column![
        toolbar::view(app),
        separator(),
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

/// Sol taraf — seçili sekmeye göre kamera ya da harita.
fn main_pane(app: &App) -> Element<'_, Message> {
    let content = match app.tab {
        Tab::Camera => camera_view::view(app),
        Tab::Map => map_view::view(app),
    };

    container(content)
        .width(Fill)
        .height(Fill)
        .style(styles::page)
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
/// Bağlantı paneli sabit tepede, gerisi kaydırılabilir. Bu ayrım iki işi
/// birden görüyor: bağlantı — her şeyin ön koşulu — daima görünür kalıyor ve
/// açılır listeli tek kontrol `scrollable` dışında kalıyor.
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
    let scrolling = column![
        section_card(motor_control::view(app)),
        joystick_card(app),
        // Hız göstergesi kart çerçevesini kendi dosyasında kuruyor (birebir
        // aynı `SPACE_LG` + `styles::card`); burada ikinci kez sarmalamak
        // çerçeveyi ve dolguyu çiftlerdi.
        speed_display::view(app),
    ]
    .spacing(SPACE_LG)
    .padding(SPACE_XL);

    let panel = column![
        container(section_card(connection_panel::view(app)))
            .width(Fill)
            .padding(SPACE_XL),
        separator(),
        scrollable(scrolling).height(Fill),
    ];

    container(panel)
        .width(SIDEBAR_WIDTH)
        .height(Fill)
        .style(styles::sidebar)
        .into()
}

/// Joystick kartı — başlık + canvas.
fn joystick_card(app: &App) -> Element<'_, Message> {
    let program = JoystickCanvas {
        joystick: app.joystick,
        connected: app.connected,
        motor_running: app.motor_running,
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
