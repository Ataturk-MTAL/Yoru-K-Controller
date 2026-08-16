//! Harita görünümü — Slint `components/map_view.slint` karşılığı.
//!
//! Slint'te tile'lar `Flickable` içindeki `Image` widget'larıydı; burada tek
//! bir `Canvas` hepsini çizer. Kaydırma ve tekerlek zoom'u canvas olaylarından
//! mesaja dönüşür, viewport boyutu ise `sensor` widget'ıyla bildirilir.

use iced::mouse;
use iced::widget::canvas::{self, Frame, Geometry, Path, Stroke};
use iced::widget::{
    button, canvas as canvas_widget, column, container, row, sensor, slider, stack, text,
};
use iced::{
    Alignment, Background, Border, Color, Element, Fill, Point, Rectangle, Renderer, Theme,
};

use crate::map::{MapState, MAX_ZOOM, MIN_ZOOM};
use crate::message::Message;
use crate::state::App;
use crate::styles;
use crate::theme::{
    Tokens, CONTROL_HEIGHT, FONT_LG, FONT_SM, FONT_XS, ICON_BUTTON, RADIUS_MD, RADIUS_SM, SPACE_MD,
    SPACE_SM,
};
use crate::view::widgets::numeric;

/// Alt zoom çubuğu yüksekliği — 40×40 zoom düğmeleri sığsın diye Slint'teki
/// 32 px'ten büyütüldü.
const ZOOM_BAR_HEIGHT: f32 = 44.0;
/// GPS butonu genişliği.
const GPS_BUTTON_WIDTH: f32 = 200.0;
/// Robot işaretçisi yarıçapı.
const MARKER_RADIUS: f32 = 8.0;
/// İşaretçi çevresindeki halka yarıçapı.
const MARKER_RING: f32 = 14.0;
/// Kenar boşluğu.
const EDGE: f32 = 8.0;

type ButtonStyleFn = fn(&Theme, button::Status) -> button::Style;

pub fn view(app: &App) -> Element<'_, Message> {
    let program = MapCanvas {
        map: &app.map,
        gps: (app.gps_active && app.gps_point_count > 0)
            .then_some((app.gps_lat as f64, app.gps_lon as f64)),
    };

    // sensor: canvas boyutu değiştiğinde görünür tile kümesi yeniden hesaplanır.
    let surface = sensor(canvas_widget(program).width(Fill).height(Fill))
        .on_show(Message::MapResized)
        .on_resize(Message::MapResized);

    let mut layers = stack![surface];
    layers = layers.push(gps_button(app));
    layers = layers.push(coordinate_overlay(app));
    layers = layers.push(attribution());

    if !app.connected {
        layers = layers.push(connection_warning());
    }

    column![layers.height(Fill), zoom_bar(app)].into()
}

/// Sol alt GPS başlat/durdur düğmesi.
fn gps_button(app: &App) -> Element<'_, Message> {
    let label = if app.gps_active {
        "■ GPS Durdur"
    } else {
        "▶ GPS Başlat"
    };

    let style: ButtonStyleFn = if app.gps_active {
        styles::danger
    } else {
        styles::success
    };

    container(
        button(text(label).size(FONT_SM).center())
            .width(GPS_BUTTON_WIDTH)
            .height(ZOOM_BAR_HEIGHT)
            .style(style)
            .on_press_maybe(app.connected.then_some(Message::GpsToggled)),
    )
    .align_bottom(Fill)
    .padding(EDGE)
    .into()
}

/// Sağ üst koordinat kutusu ya da "GPS kapalı" rozeti.
fn coordinate_overlay(app: &App) -> Element<'_, Message> {
    let content: Element<'_, Message> = if app.gps_active && app.gps_point_count > 0 {
        column![
            text!("● GPS  {} pkt", app.gps_point_count)
                .size(FONT_SM)
                .style(styles::text_success),
            text!(
                "{:.4}° {}",
                app.gps_lat,
                if app.gps_lat >= 0.0 { "N" } else { "S" }
            )
            .size(FONT_SM)
            .style(styles::text_primary),
            text!(
                "{:.4}° {}",
                app.gps_lon,
                if app.gps_lon >= 0.0 { "E" } else { "W" }
            )
            .size(FONT_SM)
            .style(styles::text_primary),
        ]
        .spacing(2.0)
        .into()
    } else if app.gps_active {
        text("GPS bekliyor...")
            .size(FONT_SM)
            .style(styles::text_warning)
            .into()
    } else {
        text("GPS kapalı")
            .size(FONT_SM)
            .style(styles::text_tertiary)
            .into()
    };

    container(
        container(content)
            .padding(SPACE_SM)
            .style(|theme: &Theme| container::Style {
                background: Some(Background::Color(Tokens::for_theme(theme).scrim)),
                border: Border {
                    radius: RADIUS_MD.into(),
                    ..Border::default()
                },
                ..container::Style::default()
            }),
    )
    .align_right(Fill)
    .align_top(Fill)
    .padding(EDGE)
    .into()
}

/// Sağ altta OSM atıfı.
///
/// Lisans gereği okunur kalmalı; 9 px soluk gri harita üzerinde kayboluyordu,
/// diğer overlay'lerle aynı koyu çipe alındı.
fn attribution<'a>() -> Element<'a, Message> {
    container(
        container(
            text("© OpenStreetMap")
                .size(FONT_XS)
                .style(styles::text_secondary),
        )
        .padding([2.0, 6.0])
        .style(|theme: &Theme| container::Style {
            background: Some(Background::Color(Tokens::for_theme(theme).scrim)),
            border: Border {
                radius: RADIUS_SM.into(),
                ..Border::default()
            },
            ..container::Style::default()
        }),
    )
    .align_right(Fill)
    .align_bottom(Fill)
    .padding(EDGE)
    .into()
}

fn connection_warning<'a>() -> Element<'a, Message> {
    container(
        container(
            text("Yörü-K bağlı değil")
                .size(FONT_SM)
                .style(styles::text_error),
        )
        .padding([4.0, 8.0])
        .style(styles::error_badge),
    )
    .padding(EDGE)
    .into()
}

/// Alt zoom çubuğu.
///
/// `−` ve `+` daha önce düz metindi: buton gibi görünüp hiçbir şey yapmıyordu.
/// Artık gerçek butonlar ve 40×40 hedef alanına sahipler.
fn zoom_bar(app: &App) -> Element<'_, Message> {
    let zoom = app.map.zoom.clamp(MIN_ZOOM, MAX_ZOOM);

    container(
        row![
            zoom_step("−", zoom.saturating_sub(1), zoom > MIN_ZOOM),
            slider(MIN_ZOOM as u8..=MAX_ZOOM as u8, zoom as u8, |value| {
                Message::MapZoomSelected(u32::from(value))
            })
            .width(Fill),
            zoom_step("+", zoom + 1, zoom < MAX_ZOOM),
            numeric(format!("z{zoom}"))
                .width(32.0)
                .align_x(Alignment::End)
                .style(styles::text_secondary),
        ]
        .spacing(SPACE_MD)
        .align_y(Alignment::Center),
    )
    .height(ZOOM_BAR_HEIGHT)
    .padding([0.0, SPACE_MD])
    .style(styles::page)
    .into()
}

/// Tek adım zoom düğmesi — sınıra gelince pasifleşir.
fn zoom_step<'a>(glyph: &'a str, target: u32, enabled: bool) -> Element<'a, Message> {
    button(text(glyph).size(FONT_LG).center())
        .width(ICON_BUTTON)
        .height(CONTROL_HEIGHT)
        .style(styles::secondary)
        .on_press_maybe(enabled.then_some(Message::MapZoomSelected(target)))
        .into()
}

// ═══════════════════════════════════════════════════════
//  Canvas
// ═══════════════════════════════════════════════════════

struct MapCanvas<'a> {
    map: &'a MapState,
    gps: Option<(f64, f64)>,
}

/// Sürükleme takibi — yalnızca etkileşim anına ait, uygulama verisi değil.
#[derive(Default)]
struct Drag {
    last: Option<Point>,
}

impl canvas::Program<Message> for MapCanvas<'_> {
    type State = Drag;

    fn update(
        &self,
        state: &mut Self::State,
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        match event {
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                state.last = cursor.position_in(bounds);
                state.last.map(|_| canvas::Action::capture())
            }

            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                state.last = None;
                None
            }

            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let last = state.last?;
                let position = cursor.position_in(bounds)?;
                state.last = Some(position);
                Some(canvas::Action::publish(Message::MapPanned {
                    dx: position.x - last.x,
                    dy: position.y - last.y,
                }))
            }

            canvas::Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                let position = cursor.position_in(bounds)?;
                let scrolled_up = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => *y > 0.0,
                    mouse::ScrollDelta::Pixels { y, .. } => *y > 0.0,
                };
                let zoom = if scrolled_up {
                    self.map.zoom + 1
                } else {
                    self.map.zoom.saturating_sub(1)
                };
                Some(
                    canvas::Action::publish(Message::MapZoomedAt {
                        zoom: zoom.clamp(MIN_ZOOM, MAX_ZOOM),
                        pivot: position,
                    })
                    .and_capture(),
                )
            }

            _ => None,
        }
    }

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let t = Tokens::for_theme(theme);
        let mut frame = Frame::new(renderer, bounds.size());

        frame.fill_rectangle(Point::ORIGIN, bounds.size(), t.surface_sunken);

        for (coord, handle) in &self.map.tiles {
            frame.draw_image(self.map.tile_rect(*coord), handle);
        }

        if let Some((lat, lon)) = self.gps {
            let position = self.map.screen_position(lat, lon);
            frame.fill(&Path::circle(position, MARKER_RADIUS), t.success);
            frame.stroke(
                &Path::circle(position, MARKER_RADIUS),
                Stroke::default().with_width(2.0).with_color(Color::WHITE),
            );
            frame.stroke(
                &Path::circle(position, MARKER_RING),
                Stroke::default().with_width(2.0).with_color(Color {
                    a: 0.4,
                    ..t.success
                }),
            );
        }

        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if state.last.is_some() {
            mouse::Interaction::Grabbing
        } else if cursor.is_over(bounds) {
            mouse::Interaction::Grab
        } else {
            mouse::Interaction::default()
        }
    }
}
