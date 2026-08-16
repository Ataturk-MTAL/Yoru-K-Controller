//! Alt durum çubuğu — Slint `app.slint` içindeki status bar bloğunun karşılığı.

use iced::widget::{container, row, text};
use iced::{Alignment, Color, Element, Fill, Theme};

use crate::message::Message;
use crate::state::App;
use crate::styles;
use crate::theme::{Tokens, FONT_XS, SPACE_MD, SPACE_SM, SPACE_XL, STATUS_BAR_HEIGHT};
use crate::view::widgets::{dot, numeric, v_separator};

/// Ayırıcı çizgi yüksekliği.
const DIVIDER_HEIGHT: f32 = 14.0;

type TextStyleFn = fn(&Theme) -> iced::widget::text::Style;

pub fn view(app: &App) -> Element<'_, Message> {
    let tokens = if app.is_dark {
        Tokens::dark()
    } else {
        Tokens::light()
    };

    let mut content = row![
        connection_section(app, &tokens),
        v_separator(DIVIDER_HEIGHT),
        motor_section(app, &tokens),
    ]
    .spacing(SPACE_XL)
    .align_y(Alignment::Center);

    if app.light_on {
        content = content.push(indicator(
            tokens.warning,
            "Işık",
            styles::text_warning as TextStyleFn,
        ));
    }
    if app.brake_on {
        content = content.push(indicator(
            tokens.error,
            "Fren",
            styles::text_error as TextStyleFn,
        ));
    }
    if app.gps_active {
        content = content.push(gps_section(app, &tokens));
    }

    // Durum metni kalan alanı doldurup sağa yaslanır. Ayrı bir esnek boşluk
    // kullanılınca satır taşıyor ve metnin sonu pencere kenarında kırpılıyordu.
    content = content.push(
        text(app.status_text.as_str())
            .size(FONT_XS)
            .width(Fill)
            .align_x(Alignment::End)
            .style(styles::text_tertiary),
    );

    container(content)
        .height(STATUS_BAR_HEIGHT)
        .width(Fill)
        .padding([0.0, SPACE_XL])
        .style(styles::bar)
        .into()
}

fn connection_section<'a>(app: &'a App, tokens: &Tokens) -> Element<'a, Message> {
    let color = if app.connected {
        tokens.sensor_online
    } else {
        tokens.sensor_offline
    };

    let label = if app.connected {
        let kind = if app.is_serial { "Serial" } else { "TCP/IP" };
        format!("{kind} — Bağlı")
    } else {
        "Bağlantı yok".to_string()
    };

    row![
        dot(color, 10.0),
        text(label).size(FONT_XS).style(if app.connected {
            styles::text_success as TextStyleFn
        } else {
            styles::text_tertiary as TextStyleFn
        }),
    ]
    .spacing(SPACE_MD)
    .align_y(Alignment::Center)
    .into()
}

fn motor_section<'a>(app: &'a App, tokens: &Tokens) -> Element<'a, Message> {
    let color = if app.motor_running {
        tokens.motor_running
    } else {
        tokens.motor_stopped
    };

    let mut section = row![
        dot(color, 10.0),
        text(if app.motor_running {
            "Motor Çalışıyor"
        } else {
            "Motor Durdu"
        })
        .size(FONT_XS)
        .style(if app.motor_running {
            styles::text_success as TextStyleFn
        } else {
            styles::text_tertiary as TextStyleFn
        }),
    ]
    .spacing(SPACE_SM)
    .align_y(Alignment::Center);

    if app.motor_running {
        section = section.push(
            text!("V{}", app.gear)
                .size(FONT_XS)
                .style(styles::text_warning),
        );
    }

    section.into()
}

/// Işık / fren göstergesi.
fn indicator<'a>(color: Color, label: &'a str, style: TextStyleFn) -> Element<'a, Message> {
    row![dot(color, 8.0), text(label).size(FONT_XS).style(style)]
        .spacing(SPACE_SM)
        .align_y(Alignment::Center)
        .into()
}

/// GPS koordinatı — dört ondalık basamağa yuvarlanır (Slint ile aynı).
fn gps_section<'a>(app: &'a App, tokens: &Tokens) -> Element<'a, Message> {
    let has_fix = app.gps_point_count > 0;

    let color = if has_fix {
        tokens.sensor_online
    } else {
        tokens.sensor_warning
    };

    let label = if has_fix {
        format!("{:.4}°  {:.4}°", app.gps_lat, app.gps_lon)
    } else {
        "GPS bekliyor...".to_string()
    };

    // Koordinat sürekli değişiyor — sabit genişlikli font olmadan rakamlar
    // her güncellemede yatayda oynuyor.
    let readout = if has_fix {
        numeric(label).size(FONT_XS).style(styles::text_success)
    } else {
        text(label).size(FONT_XS).style(styles::text_warning)
    };

    row![dot(color, 8.0), readout]
        .spacing(SPACE_SM)
        .align_y(Alignment::Center)
        .into()
}
