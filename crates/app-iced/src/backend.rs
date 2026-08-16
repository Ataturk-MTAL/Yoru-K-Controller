//! Donanım köprüsü — `crates/app/src/bridge.rs`'in Slint'ten arındırılmış hâli.
//!
//! iced MVU'da UI durumu `AppState` içinde düz alan olarak durur. Burada yalnızca
//! **thread'ler arası paylaşılan** alt küme kalır: periyodik hız göndericisinin
//! okuduğu değerler ve bağlantı yöneticisi.

use std::sync::atomic::{AtomicBool, AtomicI8, AtomicU8, Ordering};
use std::sync::{mpsc, Arc, Mutex, OnceLock};
use std::thread;
use std::time::Duration;

use iced::futures::channel::mpsc as mpsc_futures;
use iced::futures::{SinkExt, Stream};
use iced::Subscription;
use tokio::sync::mpsc as tokio_mpsc;
use tracing::debug;

use protocol::speed_packet;
use transport::{ConnectionManager, RobotEvent};

/// Periyodik gönderim alt sınırı — `send_interval_ms` bunun altına inemez.
const MIN_INTERVAL_MS: u64 = 5;

/// Robot olay akışı — `Subscription` builder'ı bir **fn pointer** olmak zorunda
/// olduğundan (iced 0.14), alıcı uç buraya park edilir ve bir kez `take()` edilir.
static EVENTS: OnceLock<Mutex<Option<tokio_mpsc::UnboundedReceiver<RobotEvent>>>> = OnceLock::new();

/// Arka plan thread'lerinin paylaştığı durum.
pub struct Backend {
    pub left: AtomicI8,
    pub right: AtomicI8,
    pub gear: AtomicU8,
    pub motor_running: AtomicBool,
    pub reverse_left: AtomicBool,
    pub reverse_right: AtomicBool,
    pub send_interval_ms: AtomicU8,
    pub connection: Mutex<ConnectionManager>,
}

impl Backend {
    /// Kanalları kurar, periyodik gönderim thread'ini başlatır.
    ///
    /// Bir kez çağrılmalıdır — olay alıcısı global yuvaya yerleşir.
    pub fn start() -> Arc<Self> {
        let (event_tx, event_rx) = mpsc::channel::<RobotEvent>();
        let (async_tx, async_rx) = tokio_mpsc::unbounded_channel::<RobotEvent>();

        // transport katmanı senkron `mpsc` kullanıyor, iced ise async akış bekliyor.
        // Aradaki pompa: bloklayan `recv()` ayrı bir thread'de döner.
        thread::Builder::new()
            .name("robot-event-pump".into())
            .spawn(move || {
                while let Ok(event) = event_rx.recv() {
                    if async_tx.send(event).is_err() {
                        break; // UI kapandı
                    }
                }
            })
            .expect("robot-event-pump thread başlatılamadı");

        let _ = EVENTS.set(Mutex::new(Some(async_rx)));

        let backend = Arc::new(Self {
            left: AtomicI8::new(0),
            right: AtomicI8::new(0),
            gear: AtomicU8::new(1),
            motor_running: AtomicBool::new(false),
            reverse_left: AtomicBool::new(false),
            reverse_right: AtomicBool::new(false),
            send_interval_ms: AtomicU8::new(30),
            connection: Mutex::new(ConnectionManager::new(event_tx)),
        });

        spawn_periodic_send(backend.clone());
        backend
    }

    /// Paketi aktif transport üzerinden gönderir.
    pub fn send(&self, packet: Vec<u8>) {
        self.connection.lock().unwrap().send(packet);
    }

    pub fn set_speeds(&self, left: i8, right: i8) {
        self.left.store(left, Ordering::Relaxed);
        self.right.store(right, Ordering::Relaxed);
    }

    /// Anlık hız paketini üretir — periyodik gönderici ve acil durdurma kullanır.
    fn speed_packet_now(&self, left: i8, right: i8) -> Vec<u8> {
        speed_packet(
            left,
            right,
            self.gear.load(Ordering::Relaxed),
            self.reverse_left.load(Ordering::Relaxed),
            self.reverse_right.load(Ordering::Relaxed),
        )
    }

    /// Motoru durdururken gönderilen son `(0, 0)` paketi.
    pub fn send_zero_speed(&self) {
        self.set_speeds(0, 0);
        let packet = self.speed_packet_now(0, 0);
        self.send(packet);
    }
}

/// Periyodik hız gönderimi — motor çalışırken her döngüde paket yollar.
///
/// Hız değişmese bile gönderilir: ESP32 tarafındaki bağlantı zaman aşımının
/// tetiklenmemesi buna bağlı (hysteresis filtresi bilerek yok).
fn spawn_periodic_send(backend: Arc<Backend>) {
    thread::Builder::new()
        .name("periodic-send".into())
        .spawn(move || loop {
            let interval = backend.send_interval_ms.load(Ordering::Relaxed) as u64;
            thread::sleep(Duration::from_millis(interval.max(MIN_INTERVAL_MS)));

            if !backend.motor_running.load(Ordering::Relaxed) {
                continue;
            }

            let left = backend.left.load(Ordering::Relaxed);
            let right = backend.right.load(Ordering::Relaxed);
            let packet = backend.speed_packet_now(left, right);
            debug!(cmd = "HIZ_PKT", left, right, "Hız paketi gönderiliyor");
            backend.send(packet);
        })
        .expect("periodic-send thread başlatılamadı");
}

/// Robot olaylarını UI'a taşıyan abonelik.
pub fn events() -> Subscription<RobotEvent> {
    Subscription::run(robot_event_stream)
}

fn robot_event_stream() -> impl Stream<Item = RobotEvent> {
    iced::stream::channel(
        64,
        |mut output: mpsc_futures::Sender<RobotEvent>| async move {
            let receiver = EVENTS
                .get()
                .and_then(|slot| slot.lock().ok().and_then(|mut guard| guard.take()));

            let Some(mut receiver) = receiver else {
                // Backend::start() çağrılmadıysa ya da abonelik yeniden kurulduysa
                // akış sessizce biter; UI çalışmaya devam eder.
                return;
            };

            while let Some(event) = receiver.recv().await {
                if output.send(event).await.is_err() {
                    break;
                }
            }
        },
    )
}

/// Seri port listesi — `available_ports()` senkron çalışır, ayrı thread'e alınır.
pub async fn list_serial_ports() -> Vec<String> {
    tokio::task::spawn_blocking(ConnectionManager::list_serial_ports)
        .await
        .unwrap_or_default()
}
