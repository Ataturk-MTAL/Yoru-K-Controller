slint::include_modules!();

mod bridge;
mod map;

use std::{
    cell::RefCell,
    rc::Rc,
    sync::{mpsc, Arc, Mutex, atomic::{AtomicBool, Ordering}},
    sync::mpsc::sync_channel,
    thread,
};

use anyhow::Result;
use slint::Global;
use tracing::info;
#[cfg(debug_assertions)]
use tracing_subscriber::EnvFilter;

use protocol::{simple_packet, light_packet, brake_packet, gps_enable_packet, Command};
use transport::{ConnectionManager, RobotEvent};
use vision::camera::{spawn_camera, list_cameras, CameraEvent};
use vision::{SharedFrame, SharedDetections};

use bridge::{SharedState, run_bridge, run_periodic_send, update_joystick_speeds, run_detection};
use map::{MapWorld, MapState};

fn main() -> Result<()> {
    // ── Loglama (sadece debug build'de aktif) ────────────
    #[cfg(debug_assertions)]
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env()
            .add_directive("app=debug".parse().unwrap()))
        .init();

    // ── Tokio runtime (TCP için) ─────────────────────────
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()?;
    let _rt_guard = rt.enter();

    // ── Kanallar ─────────────────────────────────────────
    let (robot_tx, robot_rx)   = mpsc::channel::<RobotEvent>();
    let (camera_tx, camera_rx) = sync_channel(1); // cap=1: kanal dolu → decode atlanır, CPU boşa gitmez
    let (cam_ev_tx, cam_ev_rx) = mpsc::channel::<CameraEvent>(); // kamera olayları

    // Aktif kamera stop flag — start_camera'da yeni flag oluşturulur,
    // stop_camera'da true yapılır → kamera thread çıkar, donanım serbest kalır
    let camera_stop: Arc<Mutex<Option<Arc<AtomicBool>>>> = Arc::new(Mutex::new(None));

    // En son kamera frame'i — frame-bridge yazar, detection-worker okur
    let latest_frame: SharedFrame = Arc::new(Mutex::new(None));

    // Detection etkin mi? — buton toggle'ı bu flag'i set eder
    let detection_enabled = Arc::new(AtomicBool::new(false));

    // En son detection sonuçları — detection-worker yazar, frame-bridge okur
    let latest_detections: SharedDetections = Arc::new(Mutex::new(Vec::new()));

    // ── Paylaşılan durum ─────────────────────────────────
    let conn  = ConnectionManager::new(robot_tx.clone());
    let state = Arc::new(SharedState::new(conn));

    // ── Slint penceresi ──────────────────────────────────
    let ui = AppWindow::new()?;

    // ── Başlangıç port listesi ───────────────────────────
    let ports: Vec<slint::SharedString> = ConnectionManager::list_serial_ports()
        .into_iter()
        .map(Into::into)
        .collect();
    AppState::get(&ui).set_available_ports(
        std::rc::Rc::new(slint::VecModel::from(ports)).into()
    );

    // ── Kamera listesi arka planda yükle ─────────────────
    // list_cameras() blocking — UI donmaması için ayrı thread
    {
        let ui_weak = ui.as_weak();
        thread::Builder::new()
            .name("camera-list".into())
            .spawn(move || {
                let cams = list_cameras();
                let labels: Vec<slint::SharedString> = cams
                    .into_iter()
                    .map(|(idx, name)| format!("{idx}: {name}").into())
                    .collect();
                let _ = slint::invoke_from_event_loop(move || {
                    let Some(u) = ui_weak.upgrade() else { return };
                    AppState::get(&u).set_available_cameras(
                        std::rc::Rc::new(slint::VecModel::from(labels)).into()
                    );
                });
            })
            .expect("camera-list thread başlatılamadı");
    }

    // ═══════════════════════════════════════════════════
    // Slint → Rust callback bağlantıları
    // ═══════════════════════════════════════════════════

    // Bağlantı
    let s = state.clone();
    ui.on_connect_serial(move |port, baud| {
        info!(cmd = "BAĞLAN_SERİ", port = %port, baud, "Seri bağlantı kuruluyor");
        s.connection.lock().unwrap().connect_serial(&port, baud as u32);
    });

    let s = state.clone();
    ui.on_connect_tcp(move |host, port| {
        info!(cmd = "BAĞLAN_TCP", host = %host, port, "TCP bağlantısı kuruluyor");
        s.connection.lock().unwrap().connect_tcp(&host, port as u16);
    });

    let s = state.clone();
    ui.on_disconnect(move || {
        info!(cmd = "BAĞLANTI_KES", "Bağlantı kesiliyor");
        s.connection.lock().unwrap().disconnect();
        s.motor_running.store(false, Ordering::Relaxed);
    });

    // AboutWindow referansı — ana pencere kapanırken de erişilebilir
    let about_ref: Rc<RefCell<Option<AboutWindow>>> = Rc::new(RefCell::new(None));

    let ui_weak_about = ui.as_weak();
    let about_ref_show = about_ref.clone();
    ui.on_show_about(move || {
        // Zaten açıksa tekrar oluşturma
        if about_ref_show.borrow().is_some() { return; }

        let Some(ui) = ui_weak_about.upgrade() else { return };
        let about = AboutWindow::new().unwrap();
        about.set_is_dark(AppState::get(&ui).get_is_dark());

        // "Kapat" butonu veya pencere kapatma → hide + referansı temizle
        let about_ref_close = about_ref_show.clone();
        let about_weak = about.as_weak();
        about.on_close_requested(move || {
            if let Some(a) = about_weak.upgrade() {
                a.hide().unwrap();
            }
            about_ref_close.borrow_mut().take();
        });

        // Pencere X butonuyla kapatılırsa da referansı temizle
        let about_ref_x = about_ref_show.clone();
        about.window().on_close_requested(move || {
            about_ref_x.borrow_mut().take();
            slint::CloseRequestResponse::HideWindow
        });

        about.show().unwrap();
        *about_ref_show.borrow_mut() = Some(about);
    });

    // Ana pencere kapanırken AboutWindow'u da kapat
    let about_ref_main = about_ref.clone();
    ui.window().on_close_requested(move || {
        if let Some(about) = about_ref_main.borrow_mut().take() {
            let _ = about.hide();
        }
        slint::CloseRequestResponse::HideWindow
    });

    let ui_weak_validate = ui.as_weak();
    ui.on_validate_host(move |host| {
        let valid = host.parse::<std::net::Ipv4Addr>().is_ok();
        if let Some(ui) = ui_weak_validate.upgrade() {
            AppState::get(&ui).set_tcp_host_valid(valid);
        }
    });

    let ui_weak_ports = ui.as_weak();
    ui.on_refresh_ports(move || {
        let ports: Vec<slint::SharedString> = ConnectionManager::list_serial_ports()
            .into_iter()
            .map(Into::into)
            .collect();
        if let Some(u) = ui_weak_ports.upgrade() {
            AppState::get(&u).set_available_ports(
                std::rc::Rc::new(slint::VecModel::from(ports)).into()
            );
        }
    });

    // Motor kontrol
    let s = state.clone();
    let w = ui.as_weak();
    ui.on_send_start(move || {
        let pkt = simple_packet(Command::SetStart);
        info!(cmd = "BAŞLAT", pkt = ?pkt, "Motor başlatma komutu gönderiliyor");
        s.connection.lock().unwrap().send(pkt);
        s.motor_running.store(true, Ordering::Relaxed);
        if let Some(u) = w.upgrade() {
            AppState::get(&u).set_motor_running(true);
        }
    });

    let s = state.clone();
    let w = ui.as_weak();
    ui.on_send_stop(move || {
        let pkt = simple_packet(Command::SetStop);
        info!(cmd = "DURDUR", pkt = ?pkt, "Motor durdurma komutu gönderiliyor");
        s.connection.lock().unwrap().send(pkt);
        s.motor_running.store(false, Ordering::Relaxed);
        *s.speeds.lock().unwrap() = Default::default();
        if let Some(u) = w.upgrade() {
            AppState::get(&u).set_motor_running(false);
        }
    });

    let s = state.clone();
    let w = ui.as_weak();
    ui.on_send_light(move |on| {
        let pkt = light_packet(on);
        info!(cmd = "IŞIK", on, pkt = ?pkt, "Işık komutu gönderiliyor");
        s.connection.lock().unwrap().send(pkt);
        if let Some(u) = w.upgrade() {
            AppState::get(&u).set_light_on(on);
        }
    });

    let s = state.clone();
    let w = ui.as_weak();
    ui.on_send_brake(move |on| {
        let pkt = brake_packet(on);
        info!(cmd = "FREN", on, pkt = ?pkt, "Fren komutu gönderiliyor");
        s.connection.lock().unwrap().send(pkt);
        if let Some(u) = w.upgrade() {
            AppState::get(&u).set_brake_on(on);
        }
    });

    let s = state.clone();
    ui.on_set_gear(move |g| {
        info!(cmd = "VİTES", gear = g, "Vites değiştirildi");
        s.gear.store(g as u8, Ordering::Relaxed);
    });

    let s = state.clone();
    let w = ui.as_weak();
    ui.on_set_reverse_left(move |v| {
        info!(cmd = "TERS_SOL", reverse = v, "Sol motor ters çevrildi");
        s.reverse_left.store(v, Ordering::Relaxed);
        if v { s.reverse_right.store(false, Ordering::Relaxed); }
        if let Some(u) = w.upgrade() {
            let st = AppState::get(&u);
            st.set_reverse_left(v);
            if v { st.set_reverse_right(false); }
        }
    });

    let s = state.clone();
    let w = ui.as_weak();
    ui.on_set_reverse_right(move |v| {
        info!(cmd = "TERS_SAĞ", reverse = v, "Sağ motor ters çevrildi");
        s.reverse_right.store(v, Ordering::Relaxed);
        if v { s.reverse_left.store(false, Ordering::Relaxed); }
        if let Some(u) = w.upgrade() {
            let st = AppState::get(&u);
            st.set_reverse_right(v);
            if v { st.set_reverse_left(false); }
        }
    });

    // Joystick
    let s = state.clone();
    ui.on_joystick_moved(move |dx, dy, max_r| {
        update_joystick_speeds(&s, dx, dy, max_r);
    });

    let s = state.clone();
    ui.on_joystick_released(move || {
        *s.speeds.lock().unwrap() = Default::default();
    });

    // ── Kamera ───────────────────────────────────────────
    let ui_weak_cam = ui.as_weak();
    let cam_stop_start = camera_stop.clone();
    let cam_ev_tx_start = cam_ev_tx.clone();
    ui.on_start_camera(move |index| {
        // Önceki kamera thread'ini durdur (varsa)
        if let Some(flag) = cam_stop_start.lock().unwrap().take() {
            flag.store(true, Ordering::Relaxed);
        }
        // Yeni stop flag oluştur
        let stop_flag = Arc::new(AtomicBool::new(false));
        *cam_stop_start.lock().unwrap() = Some(stop_flag.clone());

        // UI'da "başlıyor" durumu göster
        if let Some(u) = ui_weak_cam.upgrade() {
            let st = AppState::get(&u);
            st.set_camera_running(true);
            st.set_camera_error("".into());
        }

        spawn_camera(index as usize, camera_tx.clone(), stop_flag, cam_ev_tx_start.clone());
    });

    let ui_weak_stopcam = ui.as_weak();
    let cam_stop_stop = camera_stop.clone();
    ui.on_stop_camera(move || {
        // Stop flag'i true yap → kamera thread çıkar, cam.stop_stream() çalışır
        if let Some(flag) = cam_stop_stop.lock().unwrap().take() {
            flag.store(true, Ordering::Relaxed);
        }
        if let Some(u) = ui_weak_stopcam.upgrade() {
            AppState::get(&u).set_camera_running(false);
        }
    });

    // Kamera listesi yenile — arka planda (blocking)
    let ui_weak_camlist = ui.as_weak();
    ui.on_refresh_cameras(move || {
        let ui_w = ui_weak_camlist.clone();
        thread::Builder::new()
            .name("camera-list-refresh".into())
            .spawn(move || {
                let cams = list_cameras();
                let labels: Vec<slint::SharedString> = cams
                    .into_iter()
                    .map(|(idx, name)| format!("{idx}: {name}").into())
                    .collect();
                let _ = slint::invoke_from_event_loop(move || {
                    let Some(u) = ui_w.upgrade() else { return };
                    AppState::get(&u).set_available_cameras(
                        std::rc::Rc::new(slint::VecModel::from(labels)).into()
                    );
                });
            })
            .expect("camera-list-refresh thread başlatılamadı");
    });

    // GPS — yayın başlat
    let s = state.clone();
    let ui_weak_gps_on = ui.as_weak();
    ui.on_enable_gps(move || {
        s.gps_track.lock().unwrap().clear();
        let pkt = gps_enable_packet(true);
        s.connection.lock().unwrap().send(pkt);
        if let Some(u) = ui_weak_gps_on.upgrade() {
            let st = AppState::get(&u);
            st.set_gps_active(true);
            st.set_gps_point_count(0);
            st.set_gps_lat(0.0);
            st.set_gps_lon(0.0);
            st.set_gps_lat_min(0.0);
            st.set_gps_lat_max(0.0);
            st.set_gps_lon_min(0.0);
            st.set_gps_lon_max(0.0);
        }
    });

    // GPS — yayın durdur
    let s = state.clone();
    let ui_weak_gps_off = ui.as_weak();
    ui.on_disable_gps(move || {
        let pkt = gps_enable_packet(false);
        s.connection.lock().unwrap().send(pkt);
        if let Some(u) = ui_weak_gps_off.upgrade() {
            AppState::get(&u).set_gps_active(false);
        }
    });

    // Detection toggle
    let det_enabled_cb = detection_enabled.clone();
    ui.on_set_detection_enabled(move |v| {
        det_enabled_cb.store(v, Ordering::Relaxed);
    });

    // ═══════════════════════════════════════════════════
    // Harita tile sistemi (MapState — Rc tabanlı, Slint event loop'ta çalışır)
    // ═══════════════════════════════════════════════════
    let mut map_world = MapWorld::new();
    // Varsayılan merkez: Mersin
    map_world.center_on(36.8121, 34.6415);
    map_world.request_visible_tiles();

    let map_state = std::rc::Rc::new(MapState {
        world: std::cell::RefCell::new(map_world),
        ui_weak: ui.as_weak(),
        poll_handle: std::cell::RefCell::new(None),
    });

    // Viewport'u ilk kez ayarla
    map_state.set_viewport();

    // İlk tile yükleme başlat
    map_state.clone().do_poll();

    // Map callback'leri — her callback'te visible_width/height güncellenir
    let ms = map_state.clone();
    let ui_w = ui.as_weak();
    ui.on_map_flicked(move |ox, oy| {
        let Some(u) = ui_w.upgrade() else { return };
        let mut world = ms.world.borrow_mut();
        world.offset_x = -ox as f64;
        world.offset_y = -oy as f64;
        world.visible_width  = u.get_map_visible_width() as f64;
        world.visible_height = u.get_map_visible_height() as f64;
        world.request_visible_tiles();
        drop(world);
        ms.clone().do_poll();
    });

    let ms = map_state.clone();
    let ui_w = ui.as_weak();
    ui.on_map_zoom_changed(move |zoom| {
        let Some(u) = ui_w.upgrade() else { return };
        let mut world = ms.world.borrow_mut();
        world.visible_width  = u.get_map_visible_width() as f64;
        world.visible_height = u.get_map_visible_height() as f64;
        let (vw, vh) = (world.visible_width, world.visible_height);
        world.set_zoom_level(zoom as u32, vw / 2.0, vh / 2.0);
        drop(world);
        ms.set_viewport();
        ms.clone().do_poll();
    });

    let ms = map_state.clone();
    let ui_w = ui.as_weak();
    ui.on_map_zoom_in(move |ox, oy| {
        let Some(u) = ui_w.upgrade() else { return };
        let mut world = ms.world.borrow_mut();
        world.visible_width  = u.get_map_visible_width() as f64;
        world.visible_height = u.get_map_visible_height() as f64;
        let z = (world.zoom_level + 1).min(19);
        world.set_zoom_level(z, ox as f64, oy as f64);
        drop(world);
        ms.set_viewport();
        ms.clone().do_poll();
    });

    let ms = map_state.clone();
    let ui_w = ui.as_weak();
    ui.on_map_zoom_out(move |ox, oy| {
        let Some(u) = ui_w.upgrade() else { return };
        let mut world = ms.world.borrow_mut();
        world.visible_width  = u.get_map_visible_width() as f64;
        world.visible_height = u.get_map_visible_height() as f64;
        let z = world.zoom_level.saturating_sub(1).max(1);
        world.set_zoom_level(z, ox as f64, oy as f64);
        drop(world);
        ms.set_viewport();
        ms.clone().do_poll();
    });

    // ═══════════════════════════════════════════════════
    // Arka plan thread'leri
    // ═══════════════════════════════════════════════════

    // Köprü: Rust → Slint (robot olayları + kamera frame'leri + kamera event'leri)
    run_bridge(
        ui.as_weak(),
        robot_rx,
        camera_rx,
        cam_ev_rx,
        state.gps_track.clone(),
        latest_frame.clone(),
        latest_detections.clone(),
        detection_enabled.clone(),
    );

    // Detection worker — usls hub'dan otomatik indirir (ilk çalıştırmada ~6MB)
    // YOLO26: ~/.cache/usls/ altına HuggingFace'den v26-n-det.onnx indirir
    // Yerel override: uygulama dizininde v26-n-det.onnx varsa direkt kullanır
    let model_path = if std::path::Path::new("v26-n-det.onnx").exists() {
        "v26-n-det.onnx"
    } else {
        ""  // boş → usls YOLO26 hub'dan çeker
    };
    run_detection(ui.as_weak(), latest_frame, model_path.to_string(), detection_enabled, latest_detections);

    // Periyodik hız gönderimi: 20Hz
    run_periodic_send(state.clone());

    // ── Slint event loop (blocking) ──────────────────────
    ui.run()?;

    Ok(())
}
