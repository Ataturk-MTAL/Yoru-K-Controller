//! Motor kontrol paneli — Slint `components/motor_control.slint` karşılığı.

use iced::widget::{button, column, row, text, Space};
use iced::{Alignment, Element, Fill};

use crate::message::Message;
use crate::state::{App, INTERVALS_MS};
use crate::styles;
use crate::theme::{CONTROL_HEIGHT, FONT_MD, FONT_SM, SPACE_MD, SPACE_SM};
use crate::view::widgets::{icon_button, numeric, section_title};

/// Başlat/Durdur butonu yüksekliği (Slint `TMotorButton`: 44px).
const MOTOR_BUTTON_HEIGHT: f32 = CONTROL_HEIGHT;

pub fn view(app: &App) -> Element<'_, Message> {
    column![
        section_title("MOTOR KONTROL"),
        motor_button(app),
        gear_row(app),
        interval_row(app),
        peripheral_row(app),
        reverse_row(app),
    ]
    .spacing(SPACE_MD)
    .into()
}

/// Slint `TMotorButton` — bağlantı yokken pasif.
fn motor_button(app: &App) -> Element<'_, Message> {
    let label = if app.motor_running {
        "■  Motor DURDUR"
    } else {
        "▶  Motor BAŞLAT"
    };

    let message = app.connected.then_some({
        if app.motor_running {
            Message::StopPressed
        } else {
            Message::StartPressed
        }
    });

    let control = button(text(label).size(FONT_MD).center())
        .width(Fill)
        .height(MOTOR_BUTTON_HEIGHT)
        .style(styles::motor(app.motor_running))
        .on_press_maybe(message);

    // Uygulamanın birincil eylemi bu; pasifken *neden* pasif olduğu yazılmazsa
    // kullanıcı soluk butona bakıp takılıyor.
    if app.connected {
        control.into()
    } else {
        column![
            control,
            text("Motoru çalıştırmak için önce robota bağlan")
                .size(FONT_SM)
                .style(styles::text_tertiary),
        ]
        .spacing(SPACE_SM)
        .into()
    }
}

/// Vites seçici — Slint'teki `SpinBox { minimum: 1; maximum: 3 }` karşılığı.
fn gear_row(app: &App) -> Element<'_, Message> {
    row![
        text("Vites:").size(FONT_SM).style(styles::text_secondary),
        Space::new().width(Fill),
        icon_button(
            "−",
            (app.gear > 1).then(|| Message::GearSelected(app.gear - 1))
        ),
        numeric(format!("V{}", app.gear))
            .size(FONT_MD)
            .width(36.0)
            .center()
            .style(styles::text_warning),
        icon_button(
            "+",
            (app.gear < 3).then(|| Message::GearSelected(app.gear + 1))
        ),
    ]
    .spacing(SPACE_SM)
    .align_y(Alignment::Center)
    .into()
}

/// Gönderim aralığı — motor çalışırken değiştirilemez (Slint ile aynı kural).
///
/// Açılır liste yerine adım düğmeleri: hem vites satırıyla aynı dili konuşuyor
/// hem de `scrollable` içinde açılır liste bırakmıyor (bkz. `view::sidebar`).
fn interval_row(app: &App) -> Element<'_, Message> {
    let index = INTERVALS_MS
        .iter()
        .position(|value| *value == app.send_interval_ms)
        .unwrap_or(0);

    let editable = !app.motor_running;
    let previous =
        (editable && index > 0).then(|| Message::IntervalSelected(INTERVALS_MS[index - 1]));
    let next = (editable && index + 1 < INTERVALS_MS.len())
        .then(|| Message::IntervalSelected(INTERVALS_MS[index + 1]));

    row![
        text("Gönd. Aralığı:")
            .size(FONT_SM)
            .style(styles::text_secondary),
        Space::new().width(Fill),
        icon_button("−", previous),
        numeric(format!("{} ms", app.send_interval_ms))
            .width(56.0)
            .center()
            .style(if editable {
                styles::text_primary
            } else {
                styles::text_tertiary
            }),
        icon_button("+", next),
    ]
    .spacing(SPACE_SM)
    .align_y(Alignment::Center)
    .into()
}

/// Işık ve fren.
fn peripheral_row(app: &App) -> Element<'_, Message> {
    row![
        button(
            text(if app.light_on {
                "💡 IŞIK ✓"
            } else {
                "💡 IŞIK"
            })
            .size(FONT_SM)
            .center()
        )
        .width(Fill)
        .style(styles::toggle(app.light_on))
        .on_press(Message::LightToggled),
        button(
            text(if app.brake_on {
                "🛑 FREN ✓"
            } else {
                "🛑 FREN"
            })
            .size(FONT_SM)
            .center()
        )
        .width(Fill)
        .style(styles::toggle(app.brake_on))
        .on_press(Message::BrakeToggled),
    ]
    .spacing(SPACE_SM)
    .into()
}

/// Motor ters bağlantı (sırt sırta montaj) — iki bayrak karşılıklı dışlar.
fn reverse_row(app: &App) -> Element<'_, Message> {
    row![
        button(
            text(if app.reverse_left {
                "↺ Sol Ters ✓"
            } else {
                "↺ Sol Ters"
            })
            .size(FONT_SM)
            .center()
        )
        .width(Fill)
        .style(styles::toggle(app.reverse_left))
        .on_press(Message::ReverseLeftToggled),
        button(
            text(if app.reverse_right {
                "↻ Sağ Ters ✓"
            } else {
                "↻ Sağ Ters"
            })
            .size(FONT_SM)
            .center()
        )
        .width(Fill)
        .style(styles::toggle(app.reverse_right))
        .on_press(Message::ReverseRightToggled),
    ]
    .spacing(SPACE_SM)
    .into()
}
