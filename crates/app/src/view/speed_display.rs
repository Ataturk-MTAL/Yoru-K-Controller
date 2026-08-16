//! Hız göstergesi — Slint `components/speed_display.slint` karşılığı.
//!
//! Her motor için çift yönlü çubuk: merkezden sola geri (kırmızı), sağa ileri
//! (yeşil). Slint'te dolgu genişliği `parent.width * (speed / 100)` ifadesiyle
//! hesaplanıyordu; iced'de aynı oran `FillPortion` ile kurulur.

use iced::widget::{column, container, row, text, Space};
use iced::{Alignment, Background, Border, Element, Fill, FillPortion, Theme};

use crate::message::Message;
use crate::state::App;
use crate::styles;
use crate::theme::{Tokens, FONT_LG, FONT_SM, RADIUS_SM, SPACE_LG, SPACE_MD, SPACE_SM};
use crate::view::widgets::{dot, numeric, section_title};

/// Çubuk yüksekliği (Slint: 12px).
const BAR_HEIGHT: f32 = 12.0;
/// Oranların çözünürlüğü — `FillPortion` tamsayı ister.
const PORTION_SCALE: f32 = 100.0;

pub fn view(app: &App) -> Element<'_, Message> {
    let content = column![
        section_title("HIZ GÖSTERGESİ"),
        speed_bar("Sol", app.left_speed),
        speed_bar("Sağ", app.right_speed),
        gear_row(app),
    ]
    .spacing(SPACE_MD);

    container(content)
        .padding(SPACE_LG)
        .width(Fill)
        .style(styles::card)
        .into()
}

/// Tek motorun çubuğu: etiket · geri · merkez · ileri · sayı.
fn speed_bar<'a>(label: &'a str, speed: i8) -> Element<'a, Message> {
    let magnitude = (speed.unsigned_abs() as f32 / PORTION_SCALE).clamp(0.0, 1.0);

    row![
        text(label)
            .size(FONT_SM)
            .width(30.0)
            .style(styles::text_secondary),
        track(magnitude, speed < 0, true),
        center_mark(),
        track(magnitude, speed > 0, false),
        numeric(format!("{}{}", if speed >= 0 { "+" } else { "" }, speed))
            .width(32.0)
            .align_x(Alignment::End)
            .style(move |theme: &Theme| {
                let t = Tokens::for_theme(theme);
                iced::widget::text::Style {
                    color: Some(match speed.cmp(&0) {
                        std::cmp::Ordering::Greater => t.success,
                        std::cmp::Ordering::Less => t.error,
                        std::cmp::Ordering::Equal => t.on_surface_muted,
                    }),
                }
            }),
    ]
    .spacing(SPACE_SM)
    .align_y(Alignment::Center)
    .into()
}

/// Bir yön çubuğu — `active` ise dolgu çizilir, `from_right` geri yönü demektir.
fn track<'a>(magnitude: f32, active: bool, from_right: bool) -> Element<'a, Message> {
    let filled = if active {
        (magnitude * PORTION_SCALE) as u16
    } else {
        0
    };
    let empty = PORTION_SCALE as u16 - filled;

    let fill_style = move |theme: &Theme| {
        let t = Tokens::for_theme(theme);
        container::Style {
            background: Some(Background::Color(if from_right {
                t.error
            } else {
                t.success
            })),
            border: Border {
                radius: RADIUS_SM.into(),
                ..Border::default()
            },
            ..container::Style::default()
        }
    };

    let mut bar = row![].width(Fill).height(BAR_HEIGHT);

    // Geri çubuğu sağdan sola dolar — boşluk önce gelir.
    if from_right {
        if empty > 0 {
            bar = bar.push(Space::new().width(FillPortion(empty)));
        }
        if filled > 0 {
            bar = bar.push(
                container(Space::new())
                    .width(FillPortion(filled))
                    .height(Fill)
                    .style(fill_style),
            );
        }
    } else {
        if filled > 0 {
            bar = bar.push(
                container(Space::new())
                    .width(FillPortion(filled))
                    .height(Fill)
                    .style(fill_style),
            );
        }
        if empty > 0 {
            bar = bar.push(Space::new().width(FillPortion(empty)));
        }
    }

    container(bar)
        .width(Fill)
        .height(BAR_HEIGHT)
        // Track kart yüzeyinden ayırt edilebilsin: `bg_tertiary` kart rengine
        // fazla yakındı, sıfır hızda gösterge tamamen kayboluyordu.
        .style(|theme: &Theme| {
            let t = Tokens::for_theme(theme);
            container::Style {
                background: Some(Background::Color(t.surface_sunken)),
                border: Border {
                    color: t.outline_variant,
                    width: 1.0,
                    radius: RADIUS_SM.into(),
                },
                ..container::Style::default()
            }
        })
        .into()
}

/// Ortadaki 2×16 px işaret.
fn center_mark<'a>() -> Element<'a, Message> {
    container(Space::new())
        .width(2.0)
        .height(16.0)
        .style(|theme: &Theme| {
            let t = Tokens::for_theme(theme);
            container::Style {
                background: Some(Background::Color(t.outline)),
                ..container::Style::default()
            }
        })
        .into()
}

/// "Vites: V2   ● Çalışıyor" satırı.
fn gear_row(app: &App) -> Element<'_, Message> {
    let tokens = if app.is_dark {
        Tokens::dark()
    } else {
        Tokens::light()
    };

    let status_color = if app.motor_running {
        tokens.sensor_online
    } else {
        tokens.motor_stopped
    };

    row![
        text("Vites:").size(FONT_SM).style(styles::text_secondary),
        text!("V{}", app.gear)
            .size(FONT_LG)
            .style(styles::text_warning),
        Space::new().width(Fill),
        dot(status_color, 10.0),
        text(if app.motor_running {
            "Çalışıyor"
        } else {
            "Durdu"
        })
        .size(FONT_SM)
        .style(if app.motor_running {
            styles::text_success
        } else {
            styles::text_tertiary
        }),
    ]
    .spacing(SPACE_MD)
    .align_y(Alignment::Center)
    .into()
}
