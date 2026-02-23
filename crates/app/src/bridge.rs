use std::{
    sync::{
        mpsc,
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU8, Ordering},
    },
    thread,
    time::Duration,
};

use slint::Global;

use control::joystick::{JoystickInput, MotorSpeeds};
use control::keyboard::KeyboardState;
use control::{joystick_calculate, keyboard_calculate};
use protocol::{speed_packet, RobotResponse};
use transport::{ConnectionManager, RobotEvent};
use vision::camera::RgbaFrame;

use crate::AppWindow;
use crate::AppState;

/// Paylaşılan uygulama durumu (thread'ler arası)
pub struct SharedState {
    pub speeds:         Arc<Mutex<MotorSpeeds>>,
    pub keyboard:       Arc<Mutex<KeyboardState>>,
    pub gear:           Arc<AtomicU8>,
    pub motor_running:  Arc<AtomicBool>,
    pub reverse_left:   Arc<AtomicBool>,
    pub reverse_right:  Arc<AtomicBool>,
    pub connection:     Arc<Mutex<ConnectionManager>>,
}

impl SharedState {
    pub fn new(connection: ConnectionManager) -> Self {
        Self {
            speeds:        Arc::new(Mutex::new(MotorSpeeds::default())),
            keyboard:      Arc::new(Mutex::new(KeyboardState::default())),
            gear:          Arc::new(AtomicU8::new(1)),
            motor_running: Arc::new(AtomicBool::new(false)),
            reverse_left:  Arc::new(AtomicBool::new(false)),
            reverse_right: Arc::new(AtomicBool::new(false)),
            connection:    Arc::new(Mutex::new(connection)),
        }
    }
}

/// Robot event'leri ve kamera frame'lerini Slint UI'a ileten köprü thread'i
pub fn run_bridge(
    ui_weak:   slint::Weak<AppWindow>,
    robot_rx:  mpsc::Receiver<RobotEvent>,
    camera_rx: mpsc::Receiver<RgbaFrame>,
) {
    thread::Builder::new()
        .name("bridge".into())
        .spawn(move || {
            loop {
                // ── Robot olayları ───────────────────────────────
                while let Ok(event) = robot_rx.try_recv() {
                    let w = ui_weak.clone();
                    let _ = slint::invoke_from_event_loop(move || {
                        let Some(ui) = w.upgrade() else { return };
                        let state = AppState::get(&ui);
                        match event {
                            RobotEvent::Connected(msg) => {
                                state.set_connected(true);
                                state.set_status_text(
                                    format!("Bağlı — {msg}").into()
                                );
                            }
                            RobotEvent::Disconnected => {
                                state.set_connected(false);
                                state.set_motor_running(false);
                                state.set_status_text("Bağlantı kesildi".into());
                            }
                            RobotEvent::Error(e) => {
                                state.set_status_text(e.into());
                            }
                            RobotEvent::Packet(resp) => match resp {
                                RobotResponse::Status(running) => {
                                    state.set_motor_running(running);
                                }
                                RobotResponse::Speed { gear, left, right } => {
                                    state.set_gear(gear as i32);
                                    state.set_left_speed(left as i32);
                                    state.set_right_speed(right as i32);
                                }
                                RobotResponse::Light(on) => {
                                    state.set_light_on(on);
                                }
                                RobotResponse::Brake(on) => {
                                    state.set_brake_on(on);
                                }
                                RobotResponse::Gps { .. } => {
                                    // Faz 2: harita güncellemesi
                                }
                                RobotResponse::Unknown(_) => {}
                            },
                        }
                    });
                }

                // ── Kamera frame'leri ────────────────────────────
                if let Ok((bytes, w, h)) = camera_rx.try_recv() {
                    let wu = ui_weak.clone();
                    let _ = slint::invoke_from_event_loop(move || {
                        let Some(ui) = wu.upgrade() else { return };
                        let pixel_buf =
                            slint::SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(
                                &bytes, w, h,
                            );
                        let frame = slint::Image::from_rgba8(pixel_buf);
                        AppState::get(&ui).set_camera_frame(frame);
                    });
                }

                thread::sleep(Duration::from_millis(8)); // ~120 Hz
            }
        })
        .expect("bridge thread başlatılamadı");
}

/// Periyodik hız gönderim thread'i (50ms = 20Hz)
/// Python protocol_controller.py'deki 50ms QTimer'ın Rust karşılığı
pub fn run_periodic_send(state: Arc<SharedState>) {
    thread::Builder::new()
        .name("periodic-send".into())
        .spawn(move || {
            let mut prev_left:  i8 = 0;
            let mut prev_right: i8 = 0;
            let mut prev_gear:  u8 = 1;

            loop {
                thread::sleep(Duration::from_millis(50));

                if !state.motor_running.load(Ordering::Relaxed) {
                    continue;
                }

                let speeds = *state.speeds.lock().unwrap();
                let gear   = state.gear.load(Ordering::Relaxed);
                let rev_l  = state.reverse_left.load(Ordering::Relaxed);
                let rev_r  = state.reverse_right.load(Ordering::Relaxed);

                // Hysteresis: fark < 2 ise gönderme (Python: if diff < 2: skip)
                let diff_l = (speeds.left  - prev_left).abs();
                let diff_r = (speeds.right - prev_right).abs();

                if diff_l > 2 || diff_r > 2 || gear != prev_gear {
                    let pkt = speed_packet(speeds.left, speeds.right, gear, rev_l, rev_r);
                    state.connection.lock().unwrap().send(pkt);

                    prev_left  = speeds.left;
                    prev_right = speeds.right;
                    prev_gear  = gear;
                }
            }
        })
        .expect("periodic-send thread başlatılamadı");
}

/// Klavye durumundan motor hızlarını günceller
pub fn update_keyboard_speeds(state: &SharedState) {
    let kb = *state.keyboard.lock().unwrap();
    let s  = keyboard_calculate(&kb);
    *state.speeds.lock().unwrap() = s;
}

/// Joystick girdisinden motor hızlarını günceller
pub fn update_joystick_speeds(state: &SharedState, dx: f32, dy: f32, max_r: f32) {
    let s = joystick_calculate(&JoystickInput { dx, dy, max_radius: max_r });
    *state.speeds.lock().unwrap() = s;
}
