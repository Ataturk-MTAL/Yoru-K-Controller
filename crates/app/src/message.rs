//! Uygulama mesajları — MVU'nun "olan biten" tarafı.
//!
//! Adlandırma geçmiş zaman: `StartPressed`, `PortsLoaded`. Slint'teki
//! `callback send-start()` / `callback joystick-moved(...)` bildirimlerinin
//! karşılığıdır.

use iced::{Point, Size};
use transport::RobotEvent;

use crate::camera::CameraUpdate;
use crate::map::{TileCoord, TileOutcome};
use crate::state::CameraChoice;

/// Sol paneldeki görünüm sekmesi.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Camera,
    Map,
}

/// Üst üste açılan bilgi penceresi (Slint'teki AboutWindow / ShortcutsWindow).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Modal {
    About,
    Shortcuts,
}

/// Ayrık bir listede bir adım — gönderim aralığı ve harita zoom'u için.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Down,
    Up,
}

/// Klavye sürüş yönü — WASD ve ok tuşları aynı yöne düşer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dir {
    Forward,
    Backward,
    Left,
    Right,
}

#[derive(Debug, Clone)]
pub enum Message {
    // ── Bağlantı ────────────────────────────────────────
    SerialModeSelected(bool),
    PortSelected(String),
    PortsRefreshRequested,
    PortsLoaded(Vec<String>),
    HostChanged(String),
    TcpPortChanged(String),
    ConnectPressed,
    DisconnectPressed,

    // ── Motor ───────────────────────────────────────────
    StartPressed,
    StopPressed,
    GearSelected(u8),
    IntervalSelected(u8),
    LightToggled,
    BrakeToggled,
    ReverseLeftToggled,
    ReverseRightToggled,

    // ── Joystick (canvas) ───────────────────────────────
    /// Bilek offset'i piksel cinsinden: `dx` sağa +, `dy` yukarı +, `max_r` halka yarıçapı.
    JoystickMoved {
        dx: f32,
        dy: f32,
        max_r: f32,
    },
    JoystickReleased,

    // ── Klavye ──────────────────────────────────────────
    DirPressed(Dir),
    DirReleased(Dir),
    EmergencyStop,

    // ── Kısayol niyetleri ───────────────────────────────
    //
    // `main::hotkeys` bir **fn pointer** olmak zorunda (`event::listen_with`
    // yakalayan closure kabul etmiyor), yani abonelik `App`'i göremiyor.
    // Duruma bağlı kısayollar bu yüzden "ne istendi"yi bildiriyor, "ne
    // yapılacak"a `update` karar veriyor. Kısayolun düğmeyle aynı kapılardan
    // geçmesi de böyle garanti altına alınıyor: düğme `on_press_maybe` ile
    // kilitliyken kısayol robota paket göndermemeli.
    /// Enter — bağlıysa keser, değilse bağlanır.
    ConnectionToggleRequested,
    /// F5 — port ve kamera listelerini birlikte yeniler.
    RefreshRequested,
    /// R — motoru başlatır (durdurma zaten `Space`'te).
    MotorStartRequested,
    /// V — kamerayı açar ya da kapatır.
    CameraToggleRequested,
    /// X — Seri Port ↔ TCP-IP.
    TransportToggleRequested,
    /// `[` / `]` — gönderim aralığında bir adım.
    IntervalStepped(Step),
    /// `−` / `+` — harita zoom'u; yalnızca Harita sekmesinde çalışır.
    MapZoomStepped(Step),

    // ── Görünüm ─────────────────────────────────────────
    TabSelected(Tab),
    ThemeToggled,
    ModalOpened(Modal),
    ModalClosed,

    // ── Kamera ──────────────────────────────────────────
    CameraSelected(CameraChoice),
    CamerasRefreshRequested,
    CamerasLoaded(Vec<(usize, String)>),
    CameraStartPressed,
    CameraStopPressed,
    DetectionToggled,
    Camera(CameraUpdate),

    // ── Harita ──────────────────────────────────────────
    /// Fare sürüklemesi (canvas pikseli).
    MapPanned {
        dx: f32,
        dy: f32,
    },
    /// Tekerlek ile zoom — `pivot` altındaki nokta sabit kalır.
    MapZoomedAt {
        zoom: u32,
        pivot: Point,
    },
    /// Alt çubuktaki zoom kaydırıcısı.
    MapZoomSelected(u32),
    /// Canvas boyutu değişti — görünür tile kümesi yeniden hesaplanır.
    MapResized(Size),
    TileLoaded(TileCoord, TileOutcome),
    GpsToggled,

    // ── Robot olayları (transport aboneliği) ────────────
    Robot(RobotEvent),

    /// Pencere kapatma isteği — kapanmadan önce robot durdurulur.
    CloseRequested,
}

impl Message {
    /// Modal açıkken bu mesaj yutulmalı mı?
    ///
    /// `modal::view` tam ekran bir `mouse_area` ile açılıyor ve tıklamayı
    /// yutuyor: modal açıkken altındaki HİÇBİR düğme basılamıyor. Klavye aynı
    /// kapıdan geçmiyordu — `hotkeys` yalnızca `event::Status::Captured`
    /// kontrolü yapıyor, klavye olayını da modal katmanındaki hiçbir widget
    /// yakalamıyor.
    ///
    /// Somut sonucu: kullanıcı `?` ile kısayol penceresini açıp "R — Motor
    /// başlat" satırını okurken `R`'ye basınca, göremediği ve durduramadığı
    /// bir motor çalışıyordu. `Enter` (bağlantıyı kes) aynı durumda.
    ///
    /// Joker kol YOK: `Message`'a yeni bir varyant eklendiğinde bu `match`
    /// derlenmez ve sınıflandırma zorunlu kalır.
    pub fn blocked_by_modal(&self) -> bool {
        match self {
            // ── Geçenler ────────────────────────────────
            // Durdurma her koşulda geçer.
            Message::EmergencyStop | Message::StopPressed => false,

            // Modalın kendi denetimi.
            Message::ModalOpened(_) | Message::ModalClosed => false,

            // BIRAKMA olayları geçmeli. Modal açılmadan önce basılı olan bir
            // sürüş tuşunun bırakılması yutulursa `keyboard` durumu o yönde
            // takılı kalır ve robot hareket etmeye devam eder. Basma yutulur,
            // bırakma yutulmaz — asimetri kasıtlı.
            Message::DirReleased(_) | Message::JoystickReleased => false,

            // Kullanıcı niyeti değil: arka plan olayları, yükleme sonuçları,
            // pencere kapanışı. Bunları yutmak telemetriyi dondururdu —
            // "Bağlantı koptu" olayını kaçırmak da yanlış bir güvenlik
            // okuması demek.
            Message::Robot(_)
            | Message::Camera(_)
            | Message::TileLoaded(..)
            | Message::PortsLoaded(_)
            | Message::CamerasLoaded(_)
            | Message::MapResized(_)
            | Message::CloseRequested => false,

            // ── Yutulanlar: hepsi kullanıcı niyeti ──────
            Message::SerialModeSelected(_)
            | Message::PortSelected(_)
            | Message::PortsRefreshRequested
            | Message::HostChanged(_)
            | Message::TcpPortChanged(_)
            | Message::ConnectPressed
            | Message::DisconnectPressed
            | Message::StartPressed
            | Message::GearSelected(_)
            | Message::IntervalSelected(_)
            | Message::LightToggled
            | Message::BrakeToggled
            | Message::ReverseLeftToggled
            | Message::ReverseRightToggled
            | Message::JoystickMoved { .. }
            | Message::DirPressed(_)
            | Message::ConnectionToggleRequested
            | Message::RefreshRequested
            | Message::MotorStartRequested
            | Message::CameraToggleRequested
            | Message::TransportToggleRequested
            | Message::IntervalStepped(_)
            | Message::MapZoomStepped(_)
            | Message::TabSelected(_)
            | Message::ThemeToggled
            | Message::CameraSelected(_)
            | Message::CamerasRefreshRequested
            | Message::CameraStartPressed
            | Message::CameraStopPressed
            | Message::DetectionToggled
            | Message::MapPanned { .. }
            | Message::MapZoomedAt { .. }
            | Message::MapZoomSelected(_)
            | Message::GpsToggled => true,
        }
    }
}
