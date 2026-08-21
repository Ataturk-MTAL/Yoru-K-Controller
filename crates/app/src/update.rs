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
use crate::map::{self, TileOutcome};
use crate::message::{Dir, Message, Step, Tab};
use crate::state::{App, CameraChoice, BAUD_RATE, INTERVALS_MS};

pub fn update(app: &mut App, message: Message) -> Task<Message> {
    // Modal açıkken klavye, farenin geçemediği yerden geçmemeli
    // (bkz. `Message::blocked_by_modal`).
    if app.modal.is_some() && message.blocked_by_modal() {
        return Task::none();
    }

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
                app.status_text = format!("Bağlanıyor — {port}");
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
                app.status_text = format!("Bağlanıyor — {host}:{port}");
            }

            // Erken dönüşlerden SONRA: gerçekten istek gitmediyse arayüz
            // "bağlanıyor" demez. Eski hata da temizlenir, yoksa yeni denemenin
            // sonucu bir öncekinin nedeniyle karışır.
            app.connecting = true;
            app.last_error = None;
            Task::none()
        }

        Message::DisconnectPressed => {
            info!(cmd = "BAĞLANTI_KES", "Bağlantı kesiliyor");
            app.backend.connection.lock().unwrap().disconnect();
            app.connecting = false;
            app.last_error = None;
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

        // `motor_control::peripheral_row` kapısı — ikisi de robota paket
        // gönderiyor. Kapı olmadan `l`/`b` bağlantı yokken arayüz durumunu
        // çeviriyor ama paket hiçbir yere gitmiyordu: sonra bağlanınca durum
        // çubuğu "Fren" yazıyor, robotta fren açık değil. Yanlış bir güvenlik
        // okuması; `RobotResponse::Brake` gelmedikçe kendiliğinden düzelmiyor.
        Message::LightToggled | Message::BrakeToggled if !app.connected => Task::none(),

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

        // ── Kısayol niyetleri ───────────────────────────
        //
        // Hepsi mevcut mesajlara devrediyor; kopyalanan tek şey **kapı**, yani
        // düğmenin `on_press_maybe` koşulu. Kapılar burada tekrar yazılmasaydı
        // klavye, düğmenin kilitli olduğu durumlarda robota paket gönderirdi.
        Message::ConnectionToggleRequested => {
            if app.connected {
                return update(app, Message::DisconnectPressed);
            }
            // `connection_panel::connect_button`'ün kapısı: bağlanma sürerken
            // ikinci basış ikinci bir bağlantı denemesi başlatır.
            if app.connecting || !app.can_connect() {
                return Task::none();
            }
            update(app, Message::ConnectPressed)
        }

        // İki listeyi birlikte yeniliyor: hangisinin tazeleneceği aktif sekmeye
        // bağlı olsaydı, yan panel her sekmede görünür olduğu için Kamera
        // sekmesindeyken port listesi kısayolsuz kalırdı.
        Message::RefreshRequested => Task::batch([
            update(app, Message::PortsRefreshRequested),
            update(app, Message::CamerasRefreshRequested),
        ]),

        Message::MotorStartRequested => {
            // `motor_control::motor_button` kapısı.
            if !app.connected || app.motor_running {
                return Task::none();
            }
            update(app, Message::StartPressed)
        }

        Message::CameraToggleRequested => {
            // `camera_view` kapısı: açılış sırasında ne başlat ne durdur.
            if app.camera_starting {
                return Task::none();
            }
            if app.camera_running {
                return update(app, Message::CameraStopPressed);
            }
            if app.selected_camera.is_none() {
                return Task::none();
            }
            update(app, Message::CameraStartPressed)
        }

        Message::TransportToggleRequested => {
            // Bağlıyken taşıma biçimini değiştirmek arayüzü hattın gerçeğinden
            // ayırır: "TCP-IP" yazarken açık olan seri port olurdu.
            if app.connected || app.connecting {
                return Task::none();
            }
            update(app, Message::SerialModeSelected(!app.is_serial))
        }

        Message::IntervalStepped(step) => {
            // `motor_control::interval_row` kapısı (`editable = !motor_running`).
            // Kapı olmadan `[`/`]` motor ÇALIŞIRKEN gönderim periyodunu
            // değiştiriyordu: `IntervalSelected` `backend.send_interval_ms`
            // atomiğine yazıyor, onu da robota hız paketi akıtan
            // `spawn_periodic_send` okuyor. 10 ms'den 50 ms'ye çıkmak, tuş
            // bırakmayla robotun onu görmesi arasındaki gecikmeyi beşe katlar.
            if app.motor_running {
                return Task::none();
            }
            let current = INTERVALS_MS
                .iter()
                .position(|ms| *ms == app.send_interval_ms)
                .unwrap_or(0);
            let Some(next) = step_index(current, step, INTERVALS_MS.len()) else {
                return Task::none();
            };
            update(app, Message::IntervalSelected(INTERVALS_MS[next]))
        }

        // Zoom yalnızca Harita sekmesinde: `+`/`−` Kamera sekmesindeyken de
        // çalışsaydı, görünmeyen bir haritanın zoom'u sessizce değişirdi.
        Message::MapZoomStepped(step) => {
            if app.tab != Tab::Map {
                return Task::none();
            }
            let zoom = match step {
                Step::Up => app.map.zoom.saturating_add(1),
                Step::Down => app.map.zoom.saturating_sub(1),
            };
            update(app, Message::MapZoomSelected(zoom))
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
            // `camera_running` burada AÇILMIYOR: kamera henüz çalışmıyor,
            // yalnızca açılıyor. Eskiden burada true yazılıyordu ve kamera hiç
            // açılamasa bile arayüz kendini "çalışıyor" sanıyordu.
            app.camera_starting = true;
            app.camera.start(choice.index);
            Task::none()
        }

        Message::CameraStopPressed => {
            info!(cmd = "KAMERA_DURDUR", "Kamera kapatılıyor");
            app.camera.stop();
            app.camera.set_detection(false);
            app.detection_enabled = false;
            app.detection_count = 0;
            app.camera_starting = false;
            app.camera_running = false;
            app.camera_frame = None;
            app.camera_fps = 0;
            Task::none()
        }

        // Tespit işçisi ölmüşse anahtar hiçbir şey yapmaz — durumu değiştirip
        // "açık" göstermek, çalışan bir tespit varmış izlenimi verir.
        Message::DetectionToggled if app.detection_error.is_some() => Task::none(),

        // Düğme yalnızca kamera çalışırken çiziliyor; `n` kısayolu o koşulu
        // görmediği için kapı burada tekrarlanıyor.
        Message::DetectionToggled if !app.camera_running => Task::none(),

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

        Message::TileLoaded(coord, TileOutcome::Loaded(handle)) => {
            app.map.insert_tile(coord, handle);
            Task::none()
        }

        Message::TileLoaded(coord, TileOutcome::Failed) => {
            app.map.drop_pending(coord);
            Task::none()
        }

        // Kuşağı geçmiş sonuç hiçbir şeye dokunmaz. `drop_pending` çağırmak,
        // aynı tile için uçmakta olan TAZE isteği `pending`'den düşürüp
        // `failed`'e yazardı: sahte "Harita çevrimdışı" rozeti ve bir sonraki
        // kaydırmada aynı tile için ikinci bir GET.
        Message::TileLoaded(_, TileOutcome::Stale) => Task::none(),

        Message::GpsToggled => {
            // `map_view::gps_button` kapısı — paket gönderiyor, bağlantı şart.
            // Düğme zaten kilitli; kapı burada da olmalı çünkü `g` kısayolu
            // düğmenin kilidini görmez.
            if !app.connected {
                return Task::none();
            }
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

        // Pencere kapanırken robot son hızında kalmamalı: STOP + (0,0) hız
        // paketi gidip periyodik gönderim durduktan sonra çıkılır.
        Message::CloseRequested => {
            let packet = simple_packet(Command::SetStop);
            info!(cmd = "DURDUR", pkt = ?packet, "Kapanış: motor durdurma komutu gönderiliyor");
            app.backend.send(packet);
            stop_motor(app);
            // `stop()` değil `shutdown()`: kamera thread'inin yanında tespit
            // işçisi de sonlandırılmalı, yoksa ONNX Runtime'ın global yıkımı
            // canlı bir çıkarımla yarışıyor (bkz. `Camera::shutdown`).
            app.camera.shutdown();
            iced::exit()
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

/// Ayrık bir listede bir adım — sınırda `None`.
///
/// Sarmalama yok: `]`'e basılı tutmak en yüksek aralıkta kalır, en düşüğe
/// atlamaz. Gönderim aralığı sürüş hissini doğrudan değiştiriyor, sınırda
/// başa dönmek istenmeyen bir sıçrama olur.
fn step_index(current: usize, step: Step, len: usize) -> Option<usize> {
    match step {
        Step::Up if current + 1 < len => Some(current + 1),
        Step::Down if current > 0 => Some(current - 1),
        _ => None,
    }
}

/// Görünür alandaki eksik tile'lar için indirme görevleri üretir.
fn request_tiles(app: &mut App) -> Task<Message> {
    let missing = app.map.take_missing_tiles();
    if missing.is_empty() {
        return Task::none();
    }

    // Kuşak, `fetch_tile` içindeki kuyruk için: zoom değişirse istek kuyruğa
    // hiç girmeden `TileOutcome::Stale` döner (`map::TileEpoch`).
    let epoch = app.map.epoch();

    Task::batch(missing.into_iter().map(move |coord| {
        Task::perform(map::fetch_tile(coord, epoch.clone()), |(coord, handle)| {
            Message::TileLoaded(coord, handle)
        })
    }))
}

fn apply_camera_update(app: &mut App, update: CameraUpdate) {
    match update {
        CameraUpdate::Frame(handle) => app.camera_frame = Some(handle),
        CameraUpdate::Fps(fps) => app.camera_fps = fps,
        CameraUpdate::Started => {
            app.camera_starting = false;
            app.camera_running = true;
            app.camera_error.clear();
        }
        CameraUpdate::Error(message) => {
            app.camera_starting = false;
            app.camera_running = false;
            app.camera_frame = None;
            app.camera_fps = 0;
            app.camera_error = message;
        }
        CameraUpdate::Stopped => {
            app.camera_starting = false;
            app.camera_running = false;
            app.camera_frame = None;
            app.camera_fps = 0;
        }
        CameraUpdate::DetectionCount(count) => app.detection_count = count,

        // Kalıcı durum: işçi bir daha doğmayacak. Anahtar kapatılır ve bir
        // daha açılamaz (bkz. `Message::DetectionToggled` koruması).
        CameraUpdate::DetectionUnavailable(reason) => {
            app.detection_error = Some(reason);
            app.detection_enabled = false;
            app.detection_count = 0;
            app.camera.set_detection(false);
        }
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
            app.connecting = false;
            app.last_error = None;
            app.status_text = format!("Bağlı — {endpoint}");
            false
        }

        // Nedeni `Error` kolu bir tur önce bıraktı; kullanıcıya kopmayı ve
        // nedenini birlikte gösteriyoruz. Neden yoksa (kullanıcı kendisi
        // kesti — `connection.rs:62` çıplak `Disconnected` yayar) düz metin.
        RobotEvent::Disconnected => {
            stop_motor(app);
            app.connected = false;
            app.connecting = false;
            app.gps_active = false;
            app.status_text = match &app.last_error {
                Some(reason) => format!("Bağlantı kesildi — {reason}"),
                None => "Bağlantı kesildi".into(),
            };
            false
        }

        RobotEvent::Error(message) => {
            // Bağlanma denemesi sırasında gelen hata denemeyi bitirir.
            app.connecting = false;
            app.last_error = Some(message.clone());
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
