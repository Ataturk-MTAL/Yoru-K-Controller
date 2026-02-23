slint::include_modules!();

mod bridge;

use std::{
    sync::{mpsc, Arc, atomic::Ordering},
    sync::mpsc::sync_channel,
};

use anyhow::Result;
use slint::Global;
use tracing_subscriber::EnvFilter;

use protocol::{simple_packet, light_packet, brake_packet, gps_enable_packet, Command};
use transport::{ConnectionManager, RobotEvent};
use vision::camera::spawn_camera;

use bridge::{SharedState, run_bridge, run_periodic_send, update_joystick_speeds};

fn main() -> Result<()> {
    // ── Loglama ─────────────────────────────────────────
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
    let (robot_tx, robot_rx)    = mpsc::channel::<RobotEvent>();
    let (camera_tx, camera_rx)  = sync_channel(1); // capacity=1: meşgulken frame düşür

    // ── Paylaşılan durum ─────────────────────────────────
    let conn    = ConnectionManager::new(robot_tx.clone());
    let state   = Arc::new(SharedState::new(conn));

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

    // ═══════════════════════════════════════════════════
    // Slint → Rust callback bağlantıları
    // ═══════════════════════════════════════════════════

    // Bağlantı
    let s = state.clone();
    ui.on_connect_serial(move |port, baud| {
        s.connection.lock().unwrap().connect_serial(&port, baud as u32);
    });

    let s = state.clone();
    ui.on_connect_tcp(move |host, port| {
        s.connection.lock().unwrap().connect_tcp(&host, port as u16);
    });

    let s = state.clone();
    ui.on_disconnect(move || {
        s.connection.lock().unwrap().disconnect();
        s.motor_running.store(false, Ordering::Relaxed);
    });

    ui.on_refresh_ports(move || {
        // Şimdilik placeholder — port güncelleme bridge'e taşınabilir
    });

    // Motor kontrol
    let s = state.clone();
    ui.on_send_start(move || {
        let pkt = simple_packet(Command::SetStart);
        s.connection.lock().unwrap().send(pkt);
        s.motor_running.store(true, Ordering::Relaxed);
    });

    let s = state.clone();
    ui.on_send_stop(move || {
        let pkt = simple_packet(Command::SetStop);
        s.connection.lock().unwrap().send(pkt);
        s.motor_running.store(false, Ordering::Relaxed);
        *s.speeds.lock().unwrap() = Default::default();
    });

    let s = state.clone();
    ui.on_send_light(move |on| {
        let pkt = light_packet(on);
        s.connection.lock().unwrap().send(pkt);
    });

    let s = state.clone();
    ui.on_send_brake(move |on| {
        let pkt = brake_packet(on);
        s.connection.lock().unwrap().send(pkt);
    });

    let s = state.clone();
    ui.on_set_gear(move |g| {
        s.gear.store(g as u8, Ordering::Relaxed);
    });

    let s = state.clone();
    ui.on_set_reverse_left(move |v| {
        s.reverse_left.store(v, Ordering::Relaxed);
    });

    let s = state.clone();
    ui.on_set_reverse_right(move |v| {
        s.reverse_right.store(v, Ordering::Relaxed);
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

    // Kamera — ui'yi move etmeden önce weak ref al
    let ui_weak_cam = ui.as_weak();
    ui.on_start_camera(move |index| {
        let tx = camera_tx.clone();
        spawn_camera(index as usize, tx);
        if let Some(u) = ui_weak_cam.upgrade() {
            AppState::get(&u).set_camera_running(true);
        }
    });

    let ui_weak_stopcam = ui.as_weak();
    ui.on_stop_camera(move || {
        if let Some(u) = ui_weak_stopcam.upgrade() {
            AppState::get(&u).set_camera_running(false);
        }
    });

    // GPS — yayın başlat
    let s = state.clone();
    let ui_weak_gps_on = ui.as_weak();
    ui.on_enable_gps(move || {
        // Yeni GPS oturumu — önceki izi temizle
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

    // ═══════════════════════════════════════════════════
    // Arka plan thread'leri
    // ═══════════════════════════════════════════════════

    // Köprü: Rust → Slint (gps_track Arc'ı da iletiliyor)
    run_bridge(ui.as_weak(), robot_rx, camera_rx, state.gps_track.clone());

    // Periyodik hız gönderimi: 20Hz
    run_periodic_send(state.clone());

    // ── Slint event loop (blocking) ──────────────────────
    ui.run()?;

    Ok(())
}
