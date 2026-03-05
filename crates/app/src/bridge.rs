use std::{
    sync::{
        mpsc,
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU8, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use tracing::debug;

use slint::Global;

use control::joystick::{JoystickInput, MotorSpeeds};
use control::joystick_calculate;
use control::{KeyboardState, keyboard_calculate};
use protocol::{speed_packet, RobotResponse};
use transport::{ConnectionManager, RobotEvent};
use vision::camera::{RgbaFrame, CameraEvent};
use vision::{SharedFrame, SharedDetections, YoloDetector, draw_detections};

use crate::AppWindow;
use crate::AppState;
use crate::map::lat_lon_to_tile;

/// Paylaşılan uygulama durumu (thread'ler arası)
pub struct SharedState {
    pub speeds:            Arc<Mutex<MotorSpeeds>>,
    pub gear:              Arc<AtomicU8>,
    pub motor_running:     Arc<AtomicBool>,
    pub reverse_left:      Arc<AtomicBool>,
    pub reverse_right:     Arc<AtomicBool>,
    pub send_interval_ms:  Arc<AtomicU8>,
    pub connection:        Arc<Mutex<ConnectionManager>>,
    pub gps_track:         Arc<Mutex<Vec<(f32, f32)>>>,
    pub keyboard:          Arc<Mutex<KeyboardState>>,
}

impl SharedState {
    pub fn new(connection: ConnectionManager) -> Self {
        Self {
            speeds:           Arc::new(Mutex::new(MotorSpeeds::default())),
            gear:             Arc::new(AtomicU8::new(1)),
            motor_running:    Arc::new(AtomicBool::new(false)),
            reverse_left:     Arc::new(AtomicBool::new(false)),
            reverse_right:    Arc::new(AtomicBool::new(false)),
            send_interval_ms: Arc::new(AtomicU8::new(30)),
            connection:       Arc::new(Mutex::new(connection)),
            gps_track:        Arc::new(Mutex::new(Vec::new())),
            keyboard:         Arc::new(Mutex::new(KeyboardState::default())),
        }
    }
}

/// Robot event'lerini Slint UI'a ileten köprü thread'i
/// `recv_timeout` ile meşgul bekleme yok — CPU boşa gitmiyor
pub fn run_bridge(
    ui_weak:           slint::Weak<AppWindow>,
    robot_rx:          mpsc::Receiver<RobotEvent>,
    camera_rx:         mpsc::Receiver<RgbaFrame>,
    cam_ev_rx:         mpsc::Receiver<CameraEvent>,
    gps_track:         Arc<Mutex<Vec<(f32, f32)>>>,
    latest_frame:      SharedFrame,       // detection thread'e en son frame'i ilet
    latest_detections: SharedDetections,   // detection sonuçları — frame-bridge okur
    detection_enabled: Arc<AtomicBool>,    // detection açık mı — frame-bridge kontrol eder
    shared_state:      Arc<SharedState>,   // bağlantı kopunca motoru durdurmak için
) {
    // ── Robot event thread ──────────────────────────────
    {
        let ui_weak = ui_weak.clone();
        let gps_track = gps_track.clone();
        let shared_state = shared_state.clone();
        thread::Builder::new()
            .name("robot-bridge".into())
            .spawn(move || {
                loop {
                    // blocking recv — CPU kullanmaz, event gelince uyanır
                    let event = match robot_rx.recv_timeout(Duration::from_millis(100)) {
                        Ok(e)  => e,
                        Err(_) => continue, // timeout → döngü başına dön
                    };

                    debug!(event = ?event, "Robot event alındı");
                    match &event {
                        RobotEvent::Packet(RobotResponse::Gps { lat, lon }) => {
                            let (lat, lon) = (*lat, *lon);
                            let snapshot = {
                                let mut track = gps_track.lock().unwrap();
                                track.push((lat, lon));
                                let lat_min = track.iter().map(|p| p.0).fold(f32::INFINITY, f32::min);
                                let lat_max = track.iter().map(|p| p.0).fold(f32::NEG_INFINITY, f32::max);
                                let lon_min = track.iter().map(|p| p.1).fold(f32::INFINITY, f32::min);
                                let lon_max = track.iter().map(|p| p.1).fold(f32::NEG_INFINITY, f32::max);
                                let count   = track.len() as i32;
                                (lat, lon, lat_min, lat_max, lon_min, lon_max, count)
                            };
                            let w = ui_weak.clone();
                            let _ = slint::invoke_from_event_loop(move || {
                                let Some(ui) = w.upgrade() else { return };
                                let state = AppState::get(&ui);
                                let (lat, lon, lat_min, lat_max, lon_min, lon_max, count) = snapshot;
                                state.set_gps_lat(lat);
                                state.set_gps_lon(lon);
                                state.set_gps_lat_min(lat_min);
                                state.set_gps_lat_max(lat_max);
                                state.set_gps_lon_min(lon_min);
                                state.set_gps_lon_max(lon_max);
                                state.set_gps_point_count(count);

                                // Harita marker pozisyonunu güncelle
                                let zoom = state.get_map_zoom() as u32;
                                let (tx, ty) = lat_lon_to_tile(lat as f64, lon as f64, zoom);
                                state.set_map_marker_x((tx * 256.0) as f32);
                                state.set_map_marker_y((ty * 256.0) as f32);
                            });
                        }
                        _ => {
                            let w  = ui_weak.clone();
                            let ev = event.clone();
                            let ss = shared_state.clone();
                            let _ = slint::invoke_from_event_loop(move || {
                                let Some(ui) = w.upgrade() else { return };
                                let state = AppState::get(&ui);
                                match ev {
                                    RobotEvent::Connected(msg) => {
                                        // Yeni bağlantıda motoru durdur (farklı cihaz olabilir)
                                        ss.motor_running.store(false, Ordering::Relaxed);
                                        *ss.speeds.lock().unwrap() = MotorSpeeds::default();
                                        *ss.keyboard.lock().unwrap() = KeyboardState::default();
                                        state.set_connected(true);
                                        state.set_motor_running(false);
                                        state.set_status_text(format!("Bağlı — {msg}").into());
                                    }
                                    RobotEvent::Disconnected => {
                                        // Bağlantı koptu — motoru ve keyboard'u sıfırla
                                        ss.motor_running.store(false, Ordering::Relaxed);
                                        *ss.speeds.lock().unwrap() = MotorSpeeds::default();
                                        *ss.keyboard.lock().unwrap() = KeyboardState::default();
                                        state.set_connected(false);
                                        state.set_motor_running(false);
                                        state.set_gps_active(false);
                                        state.set_status_text("Bağlantı kesildi".into());
                                    }
                                    RobotEvent::Error(e) => {
                                        state.set_status_text(e.into());
                                    }
                                    RobotEvent::Packet(resp) => match resp {
                                        RobotResponse::Status(running) => state.set_motor_running(running),
                                        RobotResponse::Speed { gear, left, right } => {
                                            state.set_gear(gear as i32);
                                            state.set_left_speed(left as i32);
                                            state.set_right_speed(right as i32);
                                        }
                                        RobotResponse::Light(on)  => state.set_light_on(on),
                                        RobotResponse::Brake(on)  => state.set_brake_on(on),
                                        RobotResponse::Gps { .. } => {}
                                        RobotResponse::Unknown(_) => {}
                                    },
                                }
                            });
                        }
                    }
                }
            })
            .expect("robot-bridge thread başlatılamadı");
    }

    // ── Kamera event thread (Started/Error/Stopped) ─────
    {
        let ui_weak = ui_weak.clone();
        thread::Builder::new()
            .name("cam-event-bridge".into())
            .spawn(move || {
                loop {
                    let ev = match cam_ev_rx.recv_timeout(Duration::from_millis(500)) {
                        Ok(e)  => e,
                        Err(_) => continue,
                    };
                    let w = ui_weak.clone();
                    let _ = slint::invoke_from_event_loop(move || {
                        let Some(ui) = w.upgrade() else { return };
                        let state = AppState::get(&ui);
                        match ev {
                            CameraEvent::Started       => {
                                state.set_camera_running(true);
                                state.set_camera_error("".into());
                            }
                            CameraEvent::Error(msg)    => {
                                state.set_camera_running(false);
                                state.set_camera_error(msg.into());
                            }
                            CameraEvent::Stopped       => {
                                state.set_camera_running(false);
                            }
                        }
                    });
                }
            })
            .expect("cam-event-bridge thread başlatılamadı");
    }

    // ── Kamera frame thread — tam hız, blocking recv ────
    // Pipeline: kamera-worker → SyncSender(cap=4) → frame-bridge → invoke_from_event_loop → UI
    // SharedPixelBuffer oluşturma (= memcpy) burada yapılır — UI thread'ini bloklamaz.
    //
    // Kritik: `frame_pending` AtomicBool ile invoke kuyruğu sınırlanır.
    // UI önceki frame'i işlemeden yeni closure kuyruğa GİRMEZ → latency birikimi önlenir.
    let frame_pending = Arc::new(AtomicBool::new(false));
    thread::Builder::new()
        .name("frame-bridge".into())
        .spawn(move || {
            let mut fps_counter = 0u32;
            let mut fps_timer   = Instant::now();

            loop {
                // blocking — kamera frame gelene kadar CPU kullanmaz
                let (bytes, w, h) = match camera_rx.recv() {
                    Ok(f)  => f,
                    Err(_) => break, // kanal kapandı (kamera durdu + sender drop)
                };

                // En son frame'i detection thread'e paylaştır (Arc clone — sıfır kopya)
                {
                    let mut guard = latest_frame.lock().unwrap();
                    *guard = Some((bytes.clone(), w, h));
                }

                // FPS hesapla
                fps_counter += 1;
                let elapsed = fps_timer.elapsed();
                let fps_update = if elapsed >= Duration::from_secs(1) {
                    let fps = fps_counter;
                    fps_counter = 0;
                    fps_timer   = Instant::now();
                    Some(fps)
                } else {
                    None
                };

                // UI önceki frame'i henüz işlemediyse bu frame'i atla — kuyruk birikmez
                if frame_pending.compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed).is_err() {
                    continue;
                }

                // Detection açıksa: son bbox'ları frame'e çiz (30fps overlay)
                // Detection kapalıysa: ham frame'i doğrudan gönder
                let final_bytes = if detection_enabled.load(Ordering::Relaxed) {
                    let dets = latest_detections.lock().unwrap().clone();
                    if !dets.is_empty() {
                        if let Some(mut rgba_img) = image::RgbaImage::from_raw(w, h, bytes.to_vec()) {
                            draw_detections(&mut rgba_img, &dets);
                            rgba_img.into_raw()
                        } else {
                            bytes.to_vec()
                        }
                    } else {
                        bytes.to_vec()
                    }
                } else {
                    bytes.to_vec()
                };

                let pixel_buf =
                    slint::SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(&final_bytes, w, h);

                // UI'a pixel_buf gönder — Image oluşturma UI thread'inde
                let wu = ui_weak.clone();
                let fp = frame_pending.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    fp.store(false, Ordering::Relaxed); // UI işledi, bir sonraki frame serbest
                    let Some(ui) = wu.upgrade() else { return };
                    let frame = slint::Image::from_rgba8(pixel_buf);
                    let state = AppState::get(&ui);
                    state.set_camera_frame(frame);
                    if let Some(fps) = fps_update {
                        state.set_camera_fps(fps as i32);
                    }
                });
            }

            // Kanal kapandıktan sonra FPS sıfırla
            let wu = ui_weak.clone();
            let _ = slint::invoke_from_event_loop(move || {
                let Some(ui) = wu.upgrade() else { return };
                AppState::get(&ui).set_camera_fps(0);
            });
        })
        .expect("frame-bridge thread başlatılamadı");
}

/// Periyodik hız gönderim thread'i — aralık `send_interval_ms` ile belirlenir
/// Motor çalışırken mevcut hızı HER DÖNGÜDE gönderir (ESP32 timeout'a düşmesin).
/// Motor durdurulduğunda son bir (0,0) paketi gönderilir ve keyboard sıfırlanır.
pub fn run_periodic_send(state: Arc<SharedState>) {
    thread::Builder::new()
        .name("periodic-send".into())
        .spawn(move || {
            let mut was_running = false;

            loop {
                let interval = state.send_interval_ms.load(Ordering::Relaxed) as u64;
                thread::sleep(Duration::from_millis(interval.max(5)));

                let running = state.motor_running.load(Ordering::Relaxed);

                // Motor durdurulduğunda: son (0,0) paketi gönder, keyboard sıfırla
                if was_running && !running {
                    *state.speeds.lock().unwrap() = MotorSpeeds::default();
                    *state.keyboard.lock().unwrap() = KeyboardState::default();
                    let gear  = state.gear.load(Ordering::Relaxed);
                    let rev_l = state.reverse_left.load(Ordering::Relaxed);
                    let rev_r = state.reverse_right.load(Ordering::Relaxed);
                    let pkt = speed_packet(0, 0, gear, rev_l, rev_r);
                    state.connection.lock().unwrap().send(pkt);
                    debug!(cmd = "HIZ_PKT", "Motor durdu → son (0,0) paketi gönderildi");
                }
                was_running = running;

                if !running {
                    continue;
                }

                let speeds = *state.speeds.lock().unwrap();
                let gear   = state.gear.load(Ordering::Relaxed);
                let rev_l  = state.reverse_left.load(Ordering::Relaxed);
                let rev_r  = state.reverse_right.load(Ordering::Relaxed);

                let pkt = speed_packet(speeds.left, speeds.right, gear, rev_l, rev_r);
                debug!(
                    cmd = "HIZ_PKT",
                    left = speeds.left,
                    right = speeds.right,
                    gear,
                    "Hız paketi gönderiliyor"
                );
                state.connection.lock().unwrap().send(pkt);
            }
        })
        .expect("periodic-send thread başlatılamadı");
}

/// Joystick girdisinden motor hızlarını günceller.
/// Motor çalışmıyorsa hız güncellenmez — gereksiz veri gönderilmesini engeller.
pub fn update_joystick_speeds(state: &SharedState, dx: f32, dy: f32, max_r: f32) {
    if !state.motor_running.load(Ordering::Relaxed) {
        return;
    }
    let s = joystick_calculate(&JoystickInput { dx, dy, max_radius: max_r });
    debug!(
        dx = format!("{dx:.1}"),
        dy = format!("{dy:.1}"),
        max_r = format!("{max_r:.1}"),
        left = s.left,
        right = s.right,
        "Joystick → hız hesaplandı"
    );
    *state.speeds.lock().unwrap() = s;
}

/// Klavye tuş basma/bırakma olayını işler.
/// KeyboardState güncellenir, hız hesaplanır ve `state.speeds`'e yazılır.
pub fn update_keyboard(state: &SharedState, key: &str, pressed: bool) {
    if !state.motor_running.load(Ordering::Relaxed) {
        return;
    }
    {
        let mut kb = state.keyboard.lock().unwrap();
        match key {
            "w" => kb.forward  = pressed,
            "s" => kb.backward = pressed,
            "a" => kb.left     = pressed,
            "d" => kb.right    = pressed,
            _   => return,
        }
    }
    let kb = state.keyboard.lock().unwrap();
    let s = keyboard_calculate(&kb);
    debug!(
        key, pressed,
        left = s.left, right = s.right,
        "Klavye → hız hesaplandı"
    );
    *state.speeds.lock().unwrap() = s;
}

/// YOLOv8 detection worker thread'i başlatır.
/// `latest_frame`'den en son frame'i alır (preview'u bloklamaz),
/// inference + draw yapar, overlay frame'i UI'a iletir.
/// Model yoksa veya yüklenemezse thread sessizce çıkar.
/// Rate-limit: maksimum 10fps (100ms/frame) — CPU'yu serbest bırakır.
/// `detection_enabled` false iken thread uyur, frame almaz (CPU sıfır).
pub fn run_detection(
    ui_weak:            slint::Weak<AppWindow>,
    latest_frame:       SharedFrame,
    model_path:         String,
    detection_enabled:  Arc<AtomicBool>,
    latest_detections:  SharedDetections,
) {
    // Detection hedef periyodu: 300ms = maks ~3fps
    const DET_INTERVAL_MS: u64 = 300;

    thread::Builder::new()
        .name("detection-worker".into())
        .spawn(move || {
            let mut detector = match YoloDetector::new(&model_path) {
                Ok(d)  => d,
                Err(e) => { eprintln!("Detection model yüklenemedi: {e}"); return; }
            };

            let mut last_inference = Instant::now()
                .checked_sub(Duration::from_millis(DET_INTERVAL_MS))
                .unwrap_or(Instant::now());

            loop {
                // Detection kapalıysa uyku — CPU sıfır
                if !detection_enabled.load(Ordering::Relaxed) {
                    // Kapatıldığında son bbox'ları temizle
                    {
                        let mut dets = latest_detections.lock().unwrap();
                        if !dets.is_empty() {
                            dets.clear();
                            // detection_count sıfırla
                            let wu = ui_weak.clone();
                            let _ = slint::invoke_from_event_loop(move || {
                                let Some(ui) = wu.upgrade() else { return };
                                AppState::get(&ui).set_detection_count(0);
                            });
                        }
                    }
                    thread::sleep(Duration::from_millis(100));
                    continue;
                }

                // Rate-limit
                let elapsed = last_inference.elapsed();
                let interval = Duration::from_millis(DET_INTERVAL_MS);
                if elapsed < interval {
                    thread::sleep(interval - elapsed);
                }

                // En son frame'i al
                let frame = {
                    let mut guard = latest_frame.lock().unwrap();
                    guard.take()
                };

                let Some((bytes, w, h)) = frame else {
                    thread::sleep(Duration::from_millis(50));
                    continue;
                };

                last_inference = Instant::now();

                // Inference
                let t0 = Instant::now();
                let detections = match detector.detect(&bytes, w, h) {
                    Ok(d)  => d,
                    Err(e) => { eprintln!("Detection hatası: {e}"); continue; }
                };
                eprintln!("[detection] inference: {}ms, {} nesne", t0.elapsed().as_millis(), detections.len());

                let count = detections.len() as i32;

                // Sonuçları paylaş — frame-bridge her frame'e çizecek
                *latest_detections.lock().unwrap() = detections;

                // UI'da nesne sayısını güncelle
                let wu = ui_weak.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    let Some(ui) = wu.upgrade() else { return };
                    AppState::get(&ui).set_detection_count(count);
                });
            }
        })
        .expect("detection-worker thread başlatılamadı");
}
