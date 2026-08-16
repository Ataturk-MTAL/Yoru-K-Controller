//! Durum makinesi — Slint `main.rs`'teki `ui.on_*` callback'lerinin karşılığı.
//!
//! MVU kuralı: burada bloklayan I/O yok. Paket gönderimi `Backend` üzerinden
//! kuyruğa bırakılır, port listeleme `Task::perform` ile arka plana atılır.

use std::sync::atomic::Ordering;

use iced::{Point, Task};
use tracing::info;

use control::{joystick_calculate, keyboard_calculate, JoystickInput};
use protocol::{
    brake_packet, gps_enable_packet, light_packet, simple_packet, Command, RobotResponse,
};
use transport::RobotEvent;

use crate::backend;
use crate::camera::{self, CameraUpdate};
use crate::map;
use crate::message::{Dir, Message};
use crate::state::{App, CameraChoice, BAUD_RATE};

pub fn update(app: &mut App, message: Message) -> Task<Message> {
    match message {
        // ── Bağlantı ────────────────────────────────────
        Message::SerialModeSelected(is_serial) => {
            app.is_serial = is_serial;
            Task::none()
        }

        Message::PortSelected(port) => {
            app.serial_port = Some(port);
            Task::none()
        }

        Message::PortsRefreshRequested => {
            Task::perform(backend::list_serial_ports(), Message::PortsLoaded)
        }

        Message::PortsLoaded(ports) => {
            // Seçili port listeden düştüyse seçimi bırak.
            if let Some(current) = &app.serial_port {
                if !ports.contains(current) {
                    app.serial_port = None;
                }
            }
            if app.serial_port.is_none() {
                app.serial_port = ports.first().cloned();
            }
            app.available_ports = ports;
            Task::none()
        }

        Message::HostChanged(host) => {
            app.tcp_host = host;
            Task::none()
        }

        Message::TcpPortChanged(port) => {
            // Slint LineEdit'te olduğu gibi yalnızca rakam kabul edilir.
            if port.chars().all(|c| c.is_ascii_digit()) && port.len() <= 5 {
                app.tcp_port = port;
            }
            Task::none()
        }

        Message::ConnectPressed => {
            if app.is_serial {
                let Some(port) = app.serial_port.clone() else {
                    return Task::none();
                };
                info!(cmd = "BAĞLAN_SERİ", port = %port, baud = BAUD_RATE, "Seri bağlantı kuruluyor");
                app.backend
                    .connection
                    .lock()
                    .unwrap()
                    .connect_serial(&port, BAUD_RATE);
            } else {
                let (Some(port), true) = (app.tcp_port_value(), app.tcp_host_valid()) else {
                    return Task::none();
                };
                let host = app.tcp_host.clone();
                info!(cmd = "BAĞLAN_TCP", host = %host, port, "TCP bağlantısı kuruluyor");
                app.backend
                    .connection
                    .lock()
                    .unwrap()
                    .connect_tcp(&host, port);
            }
            Task::none()
        }

        Message::DisconnectPressed => {
            info!(cmd = "BAĞLANTI_KES", "Bağlantı kesiliyor");
            app.backend.connection.lock().unwrap().disconnect();
            stop_motor(app);
            Task::none()
        }

        // ── Motor ───────────────────────────────────────
        Message::StartPressed => {
            let packet = simple_packet(Command::SetStart);
            info!(cmd = "BAŞLAT", pkt = ?packet, "Motor başlatma komutu gönderiliyor");
            app.backend.send(packet);
            app.backend.motor_running.store(true, Ordering::Relaxed);
            app.motor_running = true;
            Task::none()
        }

        Message::StopPressed | Message::EmergencyStop => {
            let packet = simple_packet(Command::SetStop);
            info!(cmd = "DURDUR", pkt = ?packet, "Motor durdurma komutu gönderiliyor");
            app.backend.send(packet);
            stop_motor(app);
            Task::none()
        }

        Message::GearSelected(gear) => {
            let gear = gear.clamp(1, 3);
            info!(cmd = "VİTES", gear, "Vites değiştirildi");
            app.gear = gear;
            app.backend.gear.store(gear, Ordering::Relaxed);
            Task::none()
        }

        Message::IntervalSelected(ms) => {
            let ms = ms.clamp(10, 50);
            info!(
                cmd = "GÖND_ARALIK",
                interval_ms = ms,
                "Gönderim aralığı değiştirildi"
            );
            app.send_interval_ms = ms;
            app.backend.send_interval_ms.store(ms, Ordering::Relaxed);
            Task::none()
        }

        Message::LightToggled => {
            let on = !app.light_on;
            let packet = light_packet(on);
            info!(cmd = "IŞIK", on, pkt = ?packet, "Işık komutu gönderiliyor");
            app.backend.send(packet);
            app.light_on = on;
            Task::none()
        }

        Message::BrakeToggled => {
            let on = !app.brake_on;
            let packet = brake_packet(on);
            info!(cmd = "FREN", on, pkt = ?packet, "Fren komutu gönderiliyor");
            app.backend.send(packet);
            app.brake_on = on;
            Task::none()
        }

        // Sol/sağ ters bayrakları karşılıklı dışlar (Slint mutex kuralı).
        Message::ReverseLeftToggled => {
            let on = !app.reverse_left;
            info!(cmd = "TERS_SOL", reverse = on, "Sol motor ters çevrildi");
            app.reverse_left = on;
            app.backend.reverse_left.store(on, Ordering::Relaxed);
            if on {
                app.reverse_right = false;
                app.backend.reverse_right.store(false, Ordering::Relaxed);
            }
            Task::none()
        }

        Message::ReverseRightToggled => {
            let on = !app.reverse_right;
            info!(cmd = "TERS_SAĞ", reverse = on, "Sağ motor ters çevrildi");
            app.reverse_right = on;
            app.backend.reverse_right.store(on, Ordering::Relaxed);
            if on {
                app.reverse_left = false;
                app.backend.reverse_left.store(false, Ordering::Relaxed);
            }
            Task::none()
        }

        // ── Joystick ────────────────────────────────────
        Message::JoystickMoved { dx, dy, max_r } => {
            app.joystick.dragging = true;
            app.joystick.dx = dx;
            app.joystick.dy = dy;

            if app.motor_running {
                let speeds = joystick_calculate(&JoystickInput {
                    dx,
                    dy,
                    max_radius: max_r,
                });
                apply_speeds(app, speeds.left, speeds.right);
            }
            Task::none()
        }

        // Global fare-bırakma aboneliği her tıklamada bu mesajı üretir;
        // sürükleme yokken hızlara dokunmamalı (klavyeyle sürerken bir butona
        // basmak robotu durdurmasın).
        Message::JoystickReleased => {
            if !app.joystick.dragging {
                return Task::none();
            }
            app.joystick = Default::default();
            apply_speeds(app, 0, 0);
            Task::none()
        }

        // ── Klavye ──────────────────────────────────────
        Message::DirPressed(dir) => {
            set_dir(app, dir, true);
            Task::none()
        }

        Message::DirReleased(dir) => {
            set_dir(app, dir, false);
            Task::none()
        }

        // ── Görünüm ─────────────────────────────────────
        Message::TabSelected(tab) => {
            app.tab = tab;
            Task::none()
        }

        Message::ThemeToggled => {
            app.is_dark = !app.is_dark;
            Task::none()
        }

        Message::ModalOpened(modal) => {
            app.modal = Some(modal);
            Task::none()
        }

        Message::ModalClosed => {
            app.modal = None;
            Task::none()
        }

        // ── Kamera ──────────────────────────────────────
        Message::CameraSelected(choice) => {
            app.selected_camera = Some(choice);
            Task::none()
        }

        Message::CamerasRefreshRequested => {
            Task::perform(camera::list_cameras(), Message::CamerasLoaded)
        }

        Message::CamerasLoaded(cameras) => {
            app.available_cameras = cameras
                .into_iter()
                .map(|(index, name)| CameraChoice { index, name })
                .collect();

            let still_present = app
                .selected_camera
                .as_ref()
                .is_some_and(|choice| app.available_cameras.contains(choice));
            if !still_present {
                app.selected_camera = app.available_cameras.first().cloned();
            }
            Task::none()
        }

        Message::CameraStartPressed => {
            let Some(choice) = app.selected_camera.clone() else {
                return Task::none();
            };
            info!(
                cmd = "KAMERA_BAŞLAT",
                index = choice.index,
                "Kamera açılıyor"
            );
            app.camera_error.clear();
            app.camera_running = true; // "başlıyor" durumu; Started olayı doğrular
            app.camera.start(choice.index);
            Task::none()
        }

        Message::CameraStopPressed => {
            info!(cmd = "KAMERA_DURDUR", "Kamera kapatılıyor");
            app.camera.stop();
            app.camera.set_detection(false);
            app.detection_enabled = false;
            app.detection_count = 0;
            app.camera_running = false;
            app.camera_frame = None;
            app.camera_fps = 0;
            Task::none()
        }

        Message::DetectionToggled => {
            let enabled = !app.detection_enabled;
            app.detection_enabled = enabled;
            app.camera.set_detection(enabled);
            if !enabled {
                app.detection_count = 0;
            }
            Task::none()
        }

        Message::Camera(update) => {
            apply_camera_update(app, update);
            Task::none()
        }

        // ── Harita ──────────────────────────────────────
        Message::MapPanned { dx, dy } => {
            app.map.pan(dx, dy);
            request_tiles(app)
        }

        Message::MapZoomedAt { zoom, pivot } => {
            app.map.zoom_at(zoom, pivot);
            request_tiles(app)
        }

        Message::MapZoomSelected(zoom) => {
            let pivot = Point::new(app.map.viewport.width / 2.0, app.map.viewport.height / 2.0);
            app.map.zoom_at(zoom, pivot);
            request_tiles(app)
        }

        Message::MapResized(size) => {
            app.map.set_viewport(size);
            request_tiles(app)
        }

        Message::TileLoaded(coord, Some(handle)) => {
            app.map.insert_tile(coord, handle);
            Task::none()
        }

        Message::TileLoaded(coord, None) => {
            app.map.drop_pending(coord);
            Task::none()
        }

        Message::GpsToggled => {
            let enabled = !app.gps_active;
            let packet = gps_enable_packet(enabled);
            info!(cmd = "GPS", on = enabled, pkt = ?packet, "GPS yayın komutu gönderiliyor");
            app.backend.send(packet);
            app.gps_active = enabled;
            if enabled {
                app.gps_point_count = 0;
                app.gps_lat = 0.0;
                app.gps_lon = 0.0;
            }
            Task::none()
        }

        // ── Robot olayları ──────────────────────────────
        Message::Robot(event) => {
            let first_fix = apply_robot_event(app, event);
            if first_fix {
                // İlk konum geldiğinde harita robotun üstüne ortalanır.
                app.map.center_on(app.gps_lat as f64, app.gps_lon as f64);
                request_tiles(app)
            } else {
                Task::none()
            }
        }
    }
}

/// Görünür alandaki eksik tile'lar için indirme görevleri üretir.
fn request_tiles(app: &mut App) -> Task<Message> {
    let missing = app.map.take_missing_tiles();
    if missing.is_empty() {
        return Task::none();
    }

    Task::batch(missing.into_iter().map(|coord| {
        Task::perform(map::fetch_tile(coord), |(coord, handle)| {
            Message::TileLoaded(coord, handle)
        })
    }))
}

fn apply_camera_update(app: &mut App, update: CameraUpdate) {
    match update {
        CameraUpdate::Frame(handle) => app.camera_frame = Some(handle),
        CameraUpdate::Fps(fps) => app.camera_fps = fps,
        CameraUpdate::Started => {
            app.camera_running = true;
            app.camera_error.clear();
        }
        CameraUpdate::Error(message) => {
            app.camera_running = false;
            app.camera_frame = None;
            app.camera_fps = 0;
            app.camera_error = message;
        }
        CameraUpdate::Stopped => {
            app.camera_running = false;
            app.camera_frame = None;
            app.camera_fps = 0;
        }
        CameraUpdate::DetectionCount(count) => app.detection_count = count,
    }
}

/// Motoru durdurur ve tüm sürüş girdilerini sıfırlar.
///
/// Son bir `(0, 0)` hız paketi gönderilir — periyodik gönderici durduğu için
/// robotun elinde kalan son hız değeri ancak bu paketle sıfırlanır.
fn stop_motor(app: &mut App) {
    app.backend.motor_running.store(false, Ordering::Relaxed);
    app.backend.send_zero_speed();
    app.motor_running = false;
    app.reset_drive_inputs();
}

/// Hesaplanan hızı hem arka plana hem göstergeye yazar.
fn apply_speeds(app: &mut App, left: i8, right: i8) {
    app.backend.set_speeds(left, right);
    app.left_speed = left;
    app.right_speed = right;
}

/// Klavye tuş durumunu günceller ve hızı yeniden hesaplar.
///
/// Motor çalışmıyorken tuşlar yok sayılır (Slint `update_keyboard` ile aynı kural).
fn set_dir(app: &mut App, dir: Dir, pressed: bool) {
    if !app.motor_running {
        return;
    }

    match dir {
        Dir::Forward => app.keyboard.forward = pressed,
        Dir::Backward => app.keyboard.backward = pressed,
        Dir::Left => app.keyboard.left = pressed,
        Dir::Right => app.keyboard.right = pressed,
    }

    let speeds = keyboard_calculate(&app.keyboard);
    apply_speeds(app, speeds.left, speeds.right);
}

/// Robot olayını uygular. Dönüş: bu olay **ilk GPS sabitlemesi** miydi?
fn apply_robot_event(app: &mut App, event: RobotEvent) -> bool {
    match event {
        RobotEvent::Connected(endpoint) => {
            // Yeni bağlantı farklı bir cihaz olabilir — motor daima durur.
            stop_motor(app);
            app.connected = true;
            app.status_text = format!("Bağlı — {endpoint}");
            false
        }

        RobotEvent::Disconnected => {
            stop_motor(app);
            app.connected = false;
            app.gps_active = false;
            app.status_text = "Bağlantı kesildi".into();
            false
        }

        RobotEvent::Error(message) => {
            app.status_text = message;
            false
        }

        RobotEvent::Packet(response) => match response {
            RobotResponse::Status(running) => {
                app.motor_running = running;
                app.backend.motor_running.store(running, Ordering::Relaxed);
                false
            }
            RobotResponse::Speed { gear, left, right } => {
                app.gear = gear.clamp(1, 3);
                app.left_speed = left;
                app.right_speed = right;
                false
            }
            RobotResponse::Light(on) => {
                app.light_on = on;
                false
            }
            RobotResponse::Brake(on) => {
                app.brake_on = on;
                false
            }
            RobotResponse::Gps { lat, lon } => {
                app.gps_lat = lat;
                app.gps_lon = lon;
                app.gps_point_count += 1;
                app.gps_point_count == 1
            }
            RobotResponse::Unknown(_) => false,
        },
    }
}
