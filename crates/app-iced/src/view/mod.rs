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

use iced::widget::{canvas, column, container, row, scrollable, stack};
use iced::{Element, Fill};

use crate::message::{Message, Tab};
use crate::state::App;
use crate::styles;
use crate::theme::{SIDEBAR_WIDTH, SPACE_LG, SPACE_MD};

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

/// Sağ kontrol paneli.
///
/// Bağlantı paneli sabit tepede, gerisi kaydırılabilir. Bu ayrım iki işi
/// birden görüyor: bağlantı — her şeyin ön koşulu — daima görünür kalıyor ve
/// açılır listeli tek kontrol `scrollable` dışında kalıyor.
///
/// İkincisi bir çizim kusurunu kapatıyor: `scrollable` içindeki `pick_list`,
/// açılır ok glifini kaydırma katmanının dışına, sol panelin üstüne de
/// çiziyor (iced 0.14.2'de gözlendi).
fn sidebar(app: &App) -> Element<'_, Message> {
    let scrolling = column![
        motor_control::view(app),
        separator(),
        joystick_card(app),
        speed_display::view(app),
    ]
    .spacing(SPACE_LG)
    .padding(SPACE_LG);

    let panel = column![
        container(connection_panel::view(app)).padding(SPACE_LG),
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

    container(content)
        .height(JOYSTICK_HEIGHT)
        .width(Fill)
        .padding(SPACE_LG)
        .style(styles::card)
        .into()
}
