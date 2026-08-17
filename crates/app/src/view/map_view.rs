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
use iced::{Alignment, Color, Element, Fill, Point, Rectangle, Renderer, Theme};

use crate::map::{MapState, MAX_ZOOM, MIN_ZOOM};
use crate::message::Message;
use crate::state::App;
use crate::styles;
use crate::theme::{
    Tokens, CONTROL_HEIGHT, FONT_SM, FONT_XS, RADIUS_MD, RADIUS_SM, SPACE_MD, SPACE_SM, SPACE_XS,
};
use crate::view::widgets::{connection_warning, icon_button, numeric};

/// Alt zoom çubuğu yüksekliği — kabuk, içindeki denetimden bir tık yüksek olsun
/// diye CONTROL_HEIGHT + dikey nefes payı. Buton yüksekliği DEĞİL: düğmeler
/// CONTROL_HEIGHT'tan gelir, bu sayı yalnızca çubuğun kendi kabuğunu ölçer.
const ZOOM_BAR_HEIGHT: f32 = 44.0;
/// GPS butonu genişliği.
const GPS_BUTTON_WIDTH: f32 = 200.0;
/// Robot işaretçisi yarıçapı.
const MARKER_RADIUS: f32 = 8.0;
/// İşaretçi çevresindeki halka yarıçapı.
const MARKER_RING: f32 = 14.0;
/// Canvas'taki ince çizgi kalınlığı (iskelet çerçevesi ve çaprazı).
const SKELETON_LINE: f32 = 1.0;
/// Başarısız tile'ın çapraz işaretinin kenardan içe oranı.
const SKELETON_CROSS_INSET: f32 = 0.35;

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
    if app.map.has_failures() {
        layers = layers.push(offline_badge());
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

    // Yükseklik CONTROL_HEIGHT: eskiden ZOOM_BAR_HEIGHT verilmişti, yani alt
    // çubuğun kabuk ölçüsü buton ölçüsü sanılıyordu ve bu düğme uygulamadaki
    // diğer bütün butonlardan 8 px yüksek çiziliyordu.
    container(
        button(text(label).size(FONT_SM).center())
            .width(GPS_BUTTON_WIDTH)
            .height(CONTROL_HEIGHT)
            .style(style)
            .on_press_maybe(app.connected.then_some(Message::GpsToggled)),
    )
    .align_bottom(Fill)
    .padding(SPACE_MD)
    .into()
}

/// Sağ üst koordinat kutusu ya da "GPS kapalı" rozeti.
///
/// Metinlerin hiçbiri kendi rengini vermiyor: zemin `overlay_chip` ve o çipin
/// `text_color`'ı `on_overlay`. Buradaki eski `text_success` / `text_primary` /
/// `text_tertiary` rolleri tema paletinden geliyordu, yani açık temada KOYU
/// tonlardı ve 0.72 alfa siyah çipin üstünde okunmuyordu. Durum ayrımı artık
/// metnin kendisinde ("● GPS … pkt" / "GPS bekliyor…" / "GPS kapalı").
///
/// Enlem/boylam `numeric` ile mono yazılır — durum çubuğu aynı veriyi aynı
/// fontla gösteriyor; ondalık haneler basamak basamak hizalı kalsın.
fn coordinate_overlay(app: &App) -> Element<'_, Message> {
    let content: Element<'_, Message> = if app.gps_active && app.gps_point_count > 0 {
        column![
            text!("● GPS  {} pkt", app.gps_point_count).size(FONT_SM),
            numeric(format!(
                "{:.4}° {}",
                app.gps_lat,
                if app.gps_lat >= 0.0 { "N" } else { "S" }
            )),
            numeric(format!(
                "{:.4}° {}",
                app.gps_lon,
                if app.gps_lon >= 0.0 { "E" } else { "W" }
            )),
        ]
        .spacing(SPACE_XS)
        .into()
    } else if app.gps_active {
        text("GPS bekliyor...").size(FONT_SM).into()
    } else {
        text("GPS kapalı").size(FONT_SM).into()
    };

    container(
        container(content)
            .padding(SPACE_SM)
            .style(styles::overlay_chip(RADIUS_MD)),
    )
    .align_right(Fill)
    .align_top(Fill)
    .padding(SPACE_MD)
    .into()
}

/// Üst ortada "harita çevrimdışı" rozeti.
///
/// Sol üst köşe `connection_warning`'e, sağ üst koordinat kutusuna, alt köşeler
/// GPS düğmesi ile OSM atıfına ait — bu rozet dördünün hiçbiriyle çakışmayan
/// tek serbest bölgede duruyor.
fn offline_badge<'a>() -> Element<'a, Message> {
    container(
        container(text("Harita çevrimdışı — tile'lar indirilemedi").size(FONT_SM))
            .padding([SPACE_XS, SPACE_SM])
            .style(styles::overlay_chip(RADIUS_SM)),
    )
    .center_x(Fill)
    .align_top(Fill)
    .padding(SPACE_MD)
    .into()
}

/// Sağ altta OSM atıfı.
///
/// Lisans gereği okunur kalmalı; 9 px soluk gri harita üzerinde kayboluyordu,
/// diğer overlay'lerle aynı koyu çipe alındı. Dolgu ölçek dışı 6 px değil,
/// SPACE_* adımlarından: rozet küçük olduğu için en dar iki adım.
fn attribution<'a>() -> Element<'a, Message> {
    container(
        container(text("© OpenStreetMap").size(FONT_XS))
            .padding([SPACE_XS, SPACE_SM])
            .style(styles::overlay_chip(RADIUS_SM)),
    )
    .align_right(Fill)
    .align_bottom(Fill)
    .padding(SPACE_MD)
    .into()
}

/// Alt zoom çubuğu.
///
/// `−` ve `+` daha önce düz metindi: buton gibi görünüp hiçbir şey yapmıyordu.
/// Artık ortak `icon_button` — motor panelindeki aynı −/+ glifiyle tek gövde,
/// tek punto. Yerel kopyası glifi FONT_LG çizdiği için iki panelde aynı işaret
/// iki boyda görünüyordu.
fn zoom_bar(app: &App) -> Element<'_, Message> {
    let zoom = app.map.zoom.clamp(MIN_ZOOM, MAX_ZOOM);

    container(
        row![
            icon_button(
                "−",
                (zoom > MIN_ZOOM).then_some(Message::MapZoomSelected(zoom.saturating_sub(1)))
            ),
            slider(MIN_ZOOM as u8..=MAX_ZOOM as u8, zoom as u8, |value| {
                Message::MapZoomSelected(u32::from(value))
            })
            .width(Fill),
            icon_button(
                "+",
                (zoom < MAX_ZOOM).then_some(Message::MapZoomSelected(zoom + 1))
            ),
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

// ═══════════════════════════════════════════════════════
//  Canvas
// ═══════════════════════════════════════════════════════

/// Eksik tile'ın yerine çizilen iskelet.
///
/// İki durumu ayırt eder: `failed` ise çapraz işaretli (bir daha gelmeyecek,
/// kullanıcı ağı kontrol etsin), değilse düz plaka (yükleniyor). Aradaki fark
/// olmadan yavaş ağ ile çevrimdışı ağ aynı görünüyordu.
fn draw_tile_skeleton(frame: &mut Frame, rect: Rectangle, failed: bool, t: &Tokens) {
    frame.fill_rectangle(rect.position(), rect.size(), t.surface_container_low);
    frame.stroke(
        &Path::rectangle(rect.position(), rect.size()),
        Stroke::default()
            .with_width(SKELETON_LINE)
            .with_color(t.outline_variant),
    );

    if !failed {
        return;
    }

    // Plakanın ortasında ölçekle küçülen bir çapraz — tile 256 px olduğu için
    // işaret sabit değil, kenarın oranı kadar içeriden başlıyor.
    let inset = rect.width * SKELETON_CROSS_INSET;
    let (left, right) = (rect.x + inset, rect.x + rect.width - inset);
    let (top, bottom) = (rect.y + inset, rect.y + rect.height - inset);
    let cross = Path::new(|builder| {
        builder.move_to(Point::new(left, top));
        builder.line_to(Point::new(right, bottom));
        builder.move_to(Point::new(right, top));
        builder.line_to(Point::new(left, bottom));
    });
    frame.stroke(
        &cross,
        Stroke::default()
            .with_width(SKELETON_LINE)
            .with_color(t.on_surface_disabled),
    );
}

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

        // Görünür alanın tamamı üzerinden geçiyoruz, elde olanlar üzerinden
        // değil: eksik tile'ın yerinde eskiden çıplak zemin kalıyordu ve
        // "yükleniyor" ile "indirilemedi" ayırt edilemiyordu.
        for coord in self.map.visible_tiles() {
            let rect = self.map.tile_rect(coord);
            match self.map.tiles.get(&coord) {
                Some(handle) => frame.draw_image(rect, handle),
                None => draw_tile_skeleton(&mut frame, rect, self.map.is_failed(&coord), &t),
            }
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
