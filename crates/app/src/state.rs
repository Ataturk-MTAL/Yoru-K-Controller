//! Uygulama modeli — Slint `AppState` global'inin MVU karşılığı.
//!
//! Tek doğruluk kaynağı burasıdır; `view` yalnızca bu alanları okur, hiçbir
//! atomiği ya da mutex'i görmez. `Backend` içindeki atomikler bu alanların
//! arka plan thread'lerine yansıtılmış kopyasıdır.

use std::fmt;
use std::sync::Arc;

use control::KeyboardState;
use iced::widget::image;
use iced::Theme;

use crate::backend::Backend;
use crate::camera::Camera;
use crate::map::MapState;
use crate::message::{Modal, Tab};
use crate::theme;

/// Seri port hızı — Slint tarafında da sabitti (UI'da seçici yok).
pub const BAUD_RATE: u32 = 115_200;

/// Gönderim aralığı seçenekleri (ms) — Slint ComboBox modeliyle aynı.
pub const INTERVALS_MS: [u8; 5] = [10, 20, 30, 40, 50];

/// Kamera seçeneği — `pick_list` `Display` istediği için sarmalanır.
/// Etiket biçimi Slint sürümüyle aynı: `"0: FaceTime HD Camera"`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CameraChoice {
    pub index: usize,
    pub name: String,
}

impl fmt::Display for CameraChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.index, self.name)
    }
}

/// Joystick bileğinin anlık konumu (halka merkezine göre piksel).
#[derive(Debug, Clone, Copy, Default)]
pub struct Joystick {
    pub dragging: bool,
    /// Sağa doğru +.
    pub dx: f32,
    /// Yukarı doğru +.
    pub dy: f32,
}

pub struct App {
    pub backend: Arc<Backend>,
    pub camera: Arc<Camera>,

    // ── Görünüm ─────────────────────────────────────────
    pub is_dark: bool,
    pub tab: Tab,
    pub modal: Option<Modal>,

    // ── Bağlantı ────────────────────────────────────────
    pub connected: bool,
    /// Bağlantı isteği gönderildi, `Connected`/`Error` olayı henüz gelmedi.
    ///
    /// Seri port açılışı ve TCP el sıkışması gözle görülür sürüyor; bu bayrak
    /// olmadan BAĞLAN'a basmak hiçbir şey yapmamış gibi görünüyordu.
    pub connecting: bool,
    pub is_serial: bool,
    pub status_text: String,
    /// Bağlantı hattının bildirdiği son hata.
    ///
    /// `transport` her kopmada `Error(neden)` ve hemen ardından `Disconnected`
    /// yayıyor (`serial_worker.rs:72-73`, `tcp_worker.rs:83-84`). İki ayrı MVU
    /// turu olduğu için ikincisi birincinin yazdığı `status_text`'i eziyordu:
    /// gerçek neden ("Yazma hatası: …") görünmeden "Bağlantı kesildi"ye
    /// dönüşüyordu. Neden burada duruyor, `Disconnected` kolu onu okuyor.
    pub last_error: Option<String>,
    pub serial_port: Option<String>,
    pub available_ports: Vec<String>,
    pub tcp_host: String,
    pub tcp_port: String,

    // ── Motor ───────────────────────────────────────────
    pub motor_running: bool,
    pub gear: u8,
    pub send_interval_ms: u8,
    pub reverse_left: bool,
    pub reverse_right: bool,

    // ── Çevre birimleri ─────────────────────────────────
    pub light_on: bool,
    pub brake_on: bool,

    // ── Hız göstergesi ──────────────────────────────────
    pub left_speed: i8,
    pub right_speed: i8,

    // ── Girdi ───────────────────────────────────────────
    pub joystick: Joystick,
    pub keyboard: KeyboardState,

    // ── Kamera ──────────────────────────────────────────
    pub camera_running: bool,
    /// `start()` çağrıldı, `Started`/`Error` olayı henüz gelmedi.
    ///
    /// Eskiden `camera_running` başlatma anında doğrulanıyordu: kamera hiç
    /// açılmasa bile arayüz "çalışıyor" gösteriyor, kullanıcı da hata
    /// mesajından önce boş bir DURDUR butonuna bakıyordu.
    pub camera_starting: bool,
    pub camera_frame: Option<image::Handle>,
    pub camera_fps: u32,
    pub camera_error: String,
    pub available_cameras: Vec<CameraChoice>,
    pub selected_camera: Option<CameraChoice>,
    pub detection_enabled: bool,
    pub detection_count: usize,
    /// Tespit hattının neden hiç çalışmayacağı (model yüklenemedi).
    ///
    /// `camera_error`'dan ayrı: kamera sağlam, ölen yalnızca tespit işçisi.
    /// İşçi uygulama boyunca bir kez doğuyor, dolayısıyla bu durum kalıcıdır —
    /// `Some` olduğu sürece "🔍 Algıla" düğmesi kilitli kalır.
    pub detection_error: Option<String>,

    // ── GPS / Harita ────────────────────────────────────
    pub gps_active: bool,
    pub gps_lat: f32,
    pub gps_lon: f32,
    pub gps_point_count: usize,
    pub map: MapState,
}

impl App {
    pub fn new(backend: Arc<Backend>, camera: Arc<Camera>) -> Self {
        Self {
            backend,
            camera,

            is_dark: true,
            tab: Tab::Camera,
            modal: None,

            connected: false,
            connecting: false,
            is_serial: true,
            status_text: "Hazır...".into(),
            last_error: None,
            serial_port: None,
            available_ports: Vec::new(),
            tcp_host: "192.168.4.1".into(),
            tcp_port: "80".into(),

            motor_running: false,
            gear: 1,
            send_interval_ms: 30,
            reverse_left: false,
            reverse_right: false,

            light_on: false,
            brake_on: false,

            left_speed: 0,
            right_speed: 0,

            joystick: Joystick::default(),
            keyboard: KeyboardState::default(),

            camera_running: false,
            camera_starting: false,
            camera_frame: None,
            camera_fps: 0,
            camera_error: String::new(),
            available_cameras: Vec::new(),
            selected_camera: None,
            detection_enabled: false,
            detection_count: 0,
            detection_error: None,

            gps_active: false,
            gps_lat: 0.0,
            gps_lon: 0.0,
            gps_point_count: 0,
            map: MapState::default(),
        }
    }

    pub fn theme(&self) -> Theme {
        if self.is_dark {
            theme::dark()
        } else {
            theme::light()
        }
    }

    /// TCP host alanı geçerli bir IPv4 adresi mi? (Slint `validate-host` karşılığı)
    pub fn tcp_host_valid(&self) -> bool {
        self.tcp_host.parse::<std::net::Ipv4Addr>().is_ok()
    }

    pub fn tcp_port_value(&self) -> Option<u16> {
        self.tcp_port.parse::<u16>().ok().filter(|port| *port > 0)
    }

    /// BAĞLAN butonu basılabilir mi?
    pub fn can_connect(&self) -> bool {
        if self.is_serial {
            self.serial_port.is_some()
        } else {
            self.tcp_host_valid() && self.tcp_port_value().is_some()
        }
    }

    /// Motor durduğunda tüm sürüş girdilerini sıfırlar.
    pub fn reset_drive_inputs(&mut self) {
        self.keyboard = KeyboardState::default();
        self.joystick = Joystick::default();
        self.left_speed = 0;
        self.right_speed = 0;
    }
}
