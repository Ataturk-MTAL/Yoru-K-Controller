//! Uygulama mesajları — MVU'nun "olan biten" tarafı.
//!
//! Adlandırma geçmiş zaman: `StartPressed`, `PortsLoaded`. Slint'teki
//! `callback send-start()` / `callback joystick-moved(...)` bildirimlerinin
//! karşılığıdır.

use iced::widget::image;
use iced::{Point, Size};
use transport::RobotEvent;

use crate::camera::CameraUpdate;
use crate::map::TileCoord;
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
    TileLoaded(TileCoord, Option<image::Handle>),
    GpsToggled,

    // ── Robot olayları (transport aboneliği) ────────────
    Robot(RobotEvent),
}
