//! Bağlantı paneli — Slint `components/connection_panel.slint` karşılığı.

use iced::widget::{button, column, container, pick_list, row, text, text_input};
use iced::{Alignment, Border, Element, Fill, Font, Theme};

use crate::message::Message;
use crate::state::App;
use crate::styles;
use crate::theme::{
    Tokens, CONTROL_HEIGHT, FIELD_PADDING_X, FIELD_PADDING_Y, FONT_MD, FONT_SM, RADIUS_MD,
    SPACE_MD, SPACE_SM,
};
use crate::view::widgets::{dot, icon_button, section_title};

/// Seri port satırındaki "Port:" etiketinin genişliği (Slint: 34px).
const LABEL_WIDTH: f32 = 34.0;
/// Mod seçici satır yüksekliği — 30 px'lik hedef alanı küçüktü.
const ROW_HEIGHT: f32 = CONTROL_HEIGHT;

type ButtonStyleFn = fn(&Theme, button::Status) -> button::Style;

pub fn view(app: &App) -> Element<'_, Message> {
    column![
        section_title("BAĞLANTI"),
        mode_selector(app),
        endpoint_fields(app),
        connect_button(app),
        status_line(app),
    ]
    .spacing(SPACE_MD)
    .into()
}

/// Serial / TCP-IP segment seçici.
fn mode_selector(app: &App) -> Element<'_, Message> {
    row![
        button(text("Serial").size(FONT_SM).center())
            .width(Fill)
            .height(ROW_HEIGHT)
            .style(styles::segment(app.is_serial))
            .on_press(Message::SerialModeSelected(true)),
        button(text("TCP/IP").size(FONT_SM).center())
            .width(Fill)
            .height(ROW_HEIGHT)
            .style(styles::segment(!app.is_serial))
            .on_press(Message::SerialModeSelected(false)),
    ]
    .spacing(SPACE_SM)
    .into()
}

/// Seçili moda göre port seçici ya da host/port alanları.
fn endpoint_fields(app: &App) -> Element<'_, Message> {
    if app.is_serial {
        row![
            text("Port:")
                .size(FONT_SM)
                .width(LABEL_WIDTH)
                .style(styles::text_secondary),
            pick_list(
                app.available_ports.as_slice(),
                app.serial_port.as_ref(),
                Message::PortSelected,
            )
            .placeholder("Port seçin")
            .text_size(FONT_SM)
            .padding([FIELD_PADDING_Y, FIELD_PADDING_X])
            .width(Fill),
            icon_button("↻", Some(Message::PortsRefreshRequested)),
        ]
        .spacing(SPACE_SM)
        .align_y(Alignment::Center)
        .into()
    } else {
        let host_valid = app.tcp_host_valid();

        row![
            container(
                text_input("192.168.4.1", &app.tcp_host)
                    .on_input(Message::HostChanged)
                    .font(Font::with_name("Space Mono"))
                    .padding([FIELD_PADDING_Y, FIELD_PADDING_X])
                    .size(FONT_SM)
                    .width(Fill)
            )
            .width(Fill)
            .style(move |theme: &Theme| {
                let t = Tokens::for_theme(theme);
                container::Style {
                    border: Border {
                        color: if host_valid { t.success } else { t.error },
                        width: 1.5,
                        radius: RADIUS_MD.into(),
                    },
                    ..container::Style::default()
                }
            }),
            text(":").size(FONT_MD).style(styles::text_secondary),
            text_input("80", &app.tcp_port)
                .on_input(Message::TcpPortChanged)
                .font(Font::with_name("Space Mono"))
                .padding([FIELD_PADDING_Y, FIELD_PADDING_X])
                .size(FONT_SM)
                .width(64.0),
        ]
        .spacing(SPACE_SM)
        .align_y(Alignment::Center)
        .into()
    }
}

/// BAĞLAN / BAĞLANTIYI KES.
fn connect_button(app: &App) -> Element<'_, Message> {
    let (label, message, style): (&str, Option<Message>, ButtonStyleFn) = if app.connected {
        (
            "BAĞLANTIYI KES",
            Some(Message::DisconnectPressed),
            styles::secondary,
        )
    } else {
        (
            "BAĞLAN",
            app.can_connect().then_some(Message::ConnectPressed),
            styles::accent,
        )
    };

    button(text(label).size(FONT_SM).center())
        .width(Fill)
        .height(CONTROL_HEIGHT)
        .style(style)
        .on_press_maybe(message)
        .into()
}

/// Nokta + "Seri Port — Bağlı" satırı.
fn status_line(app: &App) -> Element<'_, Message> {
    let tokens = if app.is_dark {
        Tokens::dark()
    } else {
        Tokens::light()
    };

    let (color, label) = if app.connected {
        let kind = if app.is_serial { "Seri Port" } else { "TCP/IP" };
        (tokens.sensor_online, format!("{kind} — Bağlı"))
    } else {
        (tokens.motor_stopped, "Bağlantı yok".to_string())
    };

    row![
        dot(color, 10.0),
        text(label).size(FONT_SM).style(if app.connected {
            styles::text_success
        } else {
            styles::text_tertiary
        }),
    ]
    .spacing(SPACE_MD)
    .align_y(Alignment::Center)
    .into()
}
