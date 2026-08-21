//! Sürüş kumanda kolu — Slint `joystick_panel.slint` karşılığı, `Canvas` ile.
//!
//! Slint tarafında halka/kılavuz/bilek ayrı `Rectangle`'lardı ve konum hesabı
//! `.slint` ifadelerindeydi. iced'de tek bir `canvas::Program` hem çizer hem
//! fare olaylarını mesaja çevirir.

use iced::mouse;
use iced::widget::canvas::{self, Geometry, Path, Stroke};
use iced::{Color, Point, Rectangle, Renderer, Size, Theme};

use crate::message::Message;
use crate::state::Joystick;
use crate::theme::{Tokens, SPACE_LG, SPACE_MD};

/// Halka ile bileşen kenarı arasındaki boşluk (Slint: `min(...) - 20px`).
///
/// Slint'ten geldiği gibi **çap** ölçeğinde tanımlı; `ring()` yarıçap hesabı
/// yaptığı için orada yarısı düşülür, yani kenarda görünen boşluk 10 px.
const RING_MARGIN: f32 = 20.0;
/// Bilek çapı.
const KNOB_SIZE: f32 = 56.0;
/// Merkez noktası çapı — `widgets::STATUS_DOT` ile aynı ölçek.
const CENTER_DOT: f32 = SPACE_MD;
/// Bileğin üstündeki ışık noktasının çapı.
///
/// Merkez noktasından bir basamak büyük: ikisi tam sapmasız durumda üst üste
/// gelir ve eşit çapta olsalar bilek işaretiyle halka merkezi tek nokta gibi
/// okunuyordu.
const KNOB_LIGHT: f32 = SPACE_LG;
/// Tolerans bandı genişliği — halka çapının oranı (Slint: %16).
const SNAP_BAND: f32 = 0.16;

/// Canvas çizgi kalınlığı — halka, kılavuz ve bilek kenarlığı aynı ağırlıkta.
///
/// Üç çizim de aynı değeri kullanıyor: biri değişirse üçü değişmeli, yoksa
/// hangi çizginin yapısal (halka) hangisinin yardımcı (kılavuz) olduğu
/// kalınlıktan okunuyormuş gibi bir izlenim doğuyor — oysa o ayrımı alfa
/// taşıyor.
const LINE_WIDTH: f32 = 2.0;
/// Bağlıyken halka kenarlığının opaklığı — halka bir durum göstergesi değil,
/// yeşile tam doygunlukta boyanınca bileğin kendisiyle yarışıyordu.
const RING_ALPHA: f32 = 0.5;
/// Kılavuz çizgilerinin opaklığı.
const GUIDE_ALPHA: f32 = 0.4;
/// Tolerans bantlarının opaklığı — "burada kenetlenir" ipucu, çizim değil.
const BAND_ALPHA: f32 = 0.08;
/// Motor dururken bilek dolgusunun opaklığı (bkz. `knob_colors`).
const KNOB_IDLE_ALPHA: f32 = 0.5;

/// Joystick çizimini belirleyen durumun tamamı.
///
/// `canvas_cache::Keyed` bunu karşılaştırıp önbelleği düşürüyor; buraya
/// eklenmeyen bir alan çizimi etkiliyorsa kare bayat kalır. Bilek konumu `f32`
/// ve `f32: !Eq` olduğu için bit deseniyle taşınıyor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JoystickKey {
    dx_bits: u32,
    dy_bits: u32,
    dragging: bool,
    connected: bool,
    motor_running: bool,
    /// Tema, `Tokens::for_theme` üzerinden her rengi değiştiriyor.
    dark: bool,
}

impl JoystickKey {
    pub fn new(joystick: Joystick, connected: bool, motor_running: bool, dark: bool) -> Self {
        Self {
            dx_bits: joystick.dx.to_bits(),
            dy_bits: joystick.dy.to_bits(),
            dragging: joystick.dragging,
            connected,
            motor_running,
            dark,
        }
    }
}

pub struct JoystickCanvas<'a> {
    pub joystick: Joystick,
    pub connected: bool,
    pub motor_running: bool,
    pub cache: &'a canvas::Cache,
}

impl JoystickCanvas<'_> {
    /// Halkanın merkezi ve yarıçapı — çizim ve olay işleme aynı hesabı kullanır.
    ///
    /// Bilek merkezi halka çizgisine kadar gidiyor, yani bileğin yarısı halkanın
    /// dışına taşıyor — kumanda kolunun beklenen görüntüsü bu. Taşan yarının
    /// canvas sınırını aşıp kırpılmaması için halka, **bilek yarıçapı kadar**
    /// içeri alınıyor: tam sapmada bileğin dış kenarı canvas kenarına denk gelir.
    fn ring(bounds: Rectangle) -> (Point, f32) {
        let center = Point::new(bounds.width / 2.0, bounds.height / 2.0);
        let available = bounds.width.min(bounds.height) / 2.0;
        let radius = (available - KNOB_SIZE / 2.0 - RING_MARGIN / 2.0).max(1.0);
        (center, radius)
    }

    /// İmleç konumunu halka içine kenetlenmiş offset'e çevirir.
    ///
    /// `dy` yukarı doğru pozitiftir — `control::joystick` bu yönü bekler.
    fn offset(position: Point, center: Point, max_r: f32) -> (f32, f32) {
        let raw_dx = position.x - center.x;
        let raw_dy = center.y - position.y;
        let distance = (raw_dx * raw_dx + raw_dy * raw_dy).sqrt();

        if distance > max_r && distance > 0.0 {
            let factor = max_r / distance;
            (raw_dx * factor, raw_dy * factor)
        } else {
            (raw_dx, raw_dy)
        }
    }

    /// Bilek renkleri: (dolgu, kenarlık, merkez ışığı).
    ///
    /// Merkez ışığı renkli dolgunun üstünde duruyor, bu yüzden dolgunun kendi
    /// tonundan değil beyazdan seçiliyor: koyu temada parlak yeşil, açık temada
    /// koyu yeşil dolgu var ve beyaz ikisinde de okunuyor. Eskiden bu nokta
    /// motor dururken gri, çalışırken yeşilin üstünde yeşildi — biri tema
    /// dışına düşüyordu, diğeri görünmüyordu.
    ///
    /// Motor çalışıyor / duruyor ayrımını artık dolgunun opaklığı taşıyor.
    fn knob_colors(&self, t: &Tokens) -> (Color, Color, Color) {
        let translucent = |color: Color| Color {
            a: KNOB_IDLE_ALPHA,
            ..color
        };

        match (self.joystick.dragging, self.connected, self.motor_running) {
            (true, true, _) => (t.success, t.success, Color::WHITE),
            (true, false, _) => (t.error, t.error, Color::WHITE),
            (false, true, true) => (t.success, t.success, Color::WHITE),
            (false, true, false) => (translucent(t.success), t.success, Color::WHITE),
            (false, false, _) => (t.control, t.on_surface_muted, t.on_surface_muted),
        }
    }
}

impl canvas::Program<Message> for JoystickCanvas<'_> {
    /// Sürükleme durumu `AppState` içinde tutulur — widget ağacında ikinci bir
    /// doğruluk kaynağı oluşmaz.
    type State = ();

    fn update(
        &self,
        _state: &mut Self::State,
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        let (center, max_r) = Self::ring(bounds);

        match event {
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let position = cursor.position_in(bounds)?;
                let (dx, dy) = Self::offset(position, center, max_r);
                Some(
                    canvas::Action::publish(Message::JoystickMoved { dx, dy, max_r }).and_capture(),
                )
            }

            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if !self.joystick.dragging {
                    return None;
                }
                // Halka dışına taşan imleç de kenetlenerek izlenir.
                let position = cursor.position_in(bounds).or_else(|| {
                    cursor
                        .position()
                        .map(|p| Point::new(p.x - bounds.x, p.y - bounds.y))
                })?;
                let (dx, dy) = Self::offset(position, center, max_r);
                Some(canvas::Action::publish(Message::JoystickMoved {
                    dx,
                    dy,
                    max_r,
                }))
            }

            // Bırakma olayı canvas'a hiç ulaşmayabilir: sürüklerken imleç başka
            // bir widget'ın üstüne kayarsa olayı o yutar ve bilek asılı kalır.
            // Bu yüzden asıl güvence `main::pointer_release` aboneliğinde;
            // buradaki dal yalnızca hızlı yoldur.
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if !self.joystick.dragging {
                    return None;
                }
                Some(canvas::Action::publish(Message::JoystickReleased))
            }

            _ => None,
        }
    }

    /// Geometri `canvas::Cache` üzerinden çizilir.
    ///
    /// `view` her mesaj turunda koşuyor; kamera açıkken bu saniyede ~30 kez
    /// demek. Önbelleksiz sürümde halka, kılavuzlar, bantlar ve bilek her
    /// turda yeniden tesselate ediliyordu — oysa hiçbiri motor telemetrisiyle
    /// değişmiyor. Önbellek yalnızca `JoystickKey` değişince düşer.
    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let t = Tokens::for_theme(theme);
        let (center, max_r) = Self::ring(bounds);

        let geometry = self.cache.draw(renderer, bounds.size(), |frame| {
            // ── Halka zemini ────────────────────────────────
            let ring = Path::circle(center, max_r);
            frame.fill(&ring, t.surface_sunken);
            frame.stroke(
                &ring,
                Stroke::default()
                    .with_width(LINE_WIDTH)
                    .with_color(if self.connected {
                        Color {
                            a: RING_ALPHA,
                            ..t.success
                        }
                    } else {
                        t.outline
                    }),
            );

            // ── Tolerans bantları (eksen snap görseli) ──────
            let band = Color {
                a: BAND_ALPHA,
                ..t.outline_variant
            };
            let diameter = max_r * 2.0;
            let band_size = diameter * SNAP_BAND;
            frame.fill_rectangle(
                Point::new(center.x - band_size / 2.0, center.y - max_r),
                Size::new(band_size, diameter),
                band,
            );
            frame.fill_rectangle(
                Point::new(center.x - max_r, center.y - band_size / 2.0),
                Size::new(diameter, band_size),
                band,
            );

            // ── Kılavuz çizgileri ───────────────────────────
            let guides = Path::new(|builder| {
                builder.move_to(Point::new(center.x - max_r, center.y));
                builder.line_to(Point::new(center.x + max_r, center.y));
                builder.move_to(Point::new(center.x, center.y - max_r));
                builder.line_to(Point::new(center.x, center.y + max_r));
            });
            frame.stroke(
                &guides,
                Stroke::default().with_width(LINE_WIDTH).with_color(Color {
                    a: GUIDE_ALPHA,
                    ..t.outline
                }),
            );

            // ── Merkez noktası ──────────────────────────────
            frame.fill(
                &Path::circle(center, CENTER_DOT / 2.0),
                t.on_surface_variant,
            );

            // ── Bilek ───────────────────────────────────────
            let knob_center = Point::new(center.x + self.joystick.dx, center.y - self.joystick.dy);
            let knob = Path::circle(knob_center, KNOB_SIZE / 2.0);

            let (knob_fill, knob_border, light) = self.knob_colors(&t);
            frame.fill(&knob, knob_fill);
            frame.stroke(
                &knob,
                Stroke::default()
                    .with_width(LINE_WIDTH)
                    .with_color(knob_border),
            );
            frame.fill(&Path::circle(knob_center, KNOB_LIGHT / 2.0), light);
        });

        vec![geometry]
    }

    fn mouse_interaction(
        &self,
        _state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if self.joystick.dragging {
            mouse::Interaction::Grabbing
        } else if cursor.is_over(bounds) {
            mouse::Interaction::Grab
        } else {
            mouse::Interaction::default()
        }
    }
}
