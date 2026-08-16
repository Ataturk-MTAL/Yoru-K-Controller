//! Kamera + nesne tespiti hattı — `crates/app/src/bridge.rs` içindeki
//! `frame-bridge` ve `detection-worker` thread'lerinin iced karşılığı.
//!
//! Zincir: `camera-worker` (nokhwa) → `SyncSender(cap=1)` → `frame-pump`
//! (bbox çizimi burada) → tokio kanalı → `Subscription` → UI.
//!
//! Kare düşürme iki yerde: kamera thread'i kanal doluyken decode'u atlar,
//! pump ise UI kanalı doluyken kareyi düşürür. Böylece görüntü hep anlık kalır.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use iced::futures::channel::mpsc as mpsc_futures;
use iced::futures::{SinkExt, Stream};
use iced::widget::image as iced_image;
use iced::Subscription;
use tokio::sync::mpsc as tokio_mpsc;

use vision::camera::{spawn_camera, CameraEvent, RgbaFrame};
use vision::{draw_detections, SharedDetections, SharedFrame, YoloDetector};

/// Tespit hattının hedef periyodu — Slint sürümüyle aynı (maks ~3 fps).
const DETECTION_INTERVAL_MS: u64 = 300;
/// UI kanalı kapasitesi — dolduğunda kare düşürülür.
const UI_QUEUE: usize = 2;
/// Yerel model dosyası; yoksa usls hub'dan indirilir.
const LOCAL_MODEL: &str = "v26-n-det.onnx";

/// Kamera hattından UI'a giden güncellemeler.
#[derive(Debug, Clone)]
pub enum CameraUpdate {
    Frame(iced_image::Handle),
    Fps(u32),
    Started,
    Error(String),
    Stopped,
    DetectionCount(usize),
}

/// `Subscription` builder'ı fn pointer olmak zorunda — alıcı uç burada park eder.
static UPDATES: OnceLock<Mutex<Option<tokio_mpsc::Receiver<CameraUpdate>>>> = OnceLock::new();

pub struct Camera {
    /// Her `spawn_camera` çağrısına klonlanır — kanal uygulama boyunca yaşar.
    frame_tx: mpsc::SyncSender<RgbaFrame>,
    event_tx: mpsc::Sender<CameraEvent>,
    /// Aktif kamera thread'inin durdurma bayrağı.
    stop_flag: Mutex<Option<Arc<AtomicBool>>>,
    detection_enabled: Arc<AtomicBool>,
    latest_frame: SharedFrame,
    latest_detections: SharedDetections,
}

impl Camera {
    /// Kalıcı thread'leri kurar: kare pompası, olay pompası, tespit işçisi.
    pub fn start_pipeline() -> Arc<Self> {
        // cap=1: bridge meşgulken kamera thread'i decode'u atlar.
        let (frame_tx, frame_rx) = mpsc::sync_channel::<RgbaFrame>(1);
        let (event_tx, event_rx) = mpsc::channel::<CameraEvent>();
        let (ui_tx, ui_rx) = tokio_mpsc::channel::<CameraUpdate>(UI_QUEUE);

        let _ = UPDATES.set(Mutex::new(Some(ui_rx)));

        let camera = Arc::new(Self {
            frame_tx,
            event_tx,
            stop_flag: Mutex::new(None),
            detection_enabled: Arc::new(AtomicBool::new(false)),
            latest_frame: Arc::new(Mutex::new(None)),
            latest_detections: Arc::new(Mutex::new(Vec::new())),
        });

        spawn_frame_pump(
            frame_rx,
            ui_tx.clone(),
            camera.latest_frame.clone(),
            camera.latest_detections.clone(),
            camera.detection_enabled.clone(),
        );
        spawn_event_pump(event_rx, ui_tx.clone());
        spawn_detection_worker(
            ui_tx,
            camera.latest_frame.clone(),
            camera.latest_detections.clone(),
            camera.detection_enabled.clone(),
        );

        camera
    }

    /// Kamerayı açar; önceki kamera thread'i varsa durdurulur.
    pub fn start(&self, index: usize) {
        self.stop();

        let stop_flag = Arc::new(AtomicBool::new(false));
        *self.stop_flag.lock().unwrap() = Some(stop_flag.clone());

        spawn_camera(
            index,
            self.frame_tx.clone(),
            stop_flag,
            self.event_tx.clone(),
        );
    }

    /// Kamerayı durdurur — thread çıkar, `cam.stop_stream()` donanımı serbest bırakır.
    pub fn stop(&self) {
        if let Some(flag) = self.stop_flag.lock().unwrap().take() {
            flag.store(true, Ordering::Relaxed);
        }
    }

    pub fn set_detection(&self, enabled: bool) {
        self.detection_enabled.store(enabled, Ordering::Relaxed);
    }
}

/// Kare pompası — bbox çizimi ve FPS sayımı burada, UI thread'i dışında.
fn spawn_frame_pump(
    frame_rx: mpsc::Receiver<RgbaFrame>,
    ui_tx: tokio_mpsc::Sender<CameraUpdate>,
    latest_frame: SharedFrame,
    latest_detections: SharedDetections,
    detection_enabled: Arc<AtomicBool>,
) {
    thread::Builder::new()
        .name("frame-pump".into())
        .spawn(move || {
            let mut frames = 0u32;
            let mut window = Instant::now();

            while let Ok((bytes, width, height)) = frame_rx.recv() {
                // Tespit işçisi en son kareyi buradan alır.
                *latest_frame.lock().unwrap() = Some((bytes.clone(), width, height));

                let handle = render(
                    &bytes,
                    width,
                    height,
                    &latest_detections,
                    &detection_enabled,
                );
                let _ = ui_tx.try_send(CameraUpdate::Frame(handle));

                frames += 1;
                if window.elapsed() >= Duration::from_secs(1) {
                    let _ = ui_tx.try_send(CameraUpdate::Fps(frames));
                    frames = 0;
                    window = Instant::now();
                }
            }
        })
        .expect("frame-pump thread başlatılamadı");
}

/// Kareyi UI'ın gösterebileceği tutamağa çevirir; tespit açıksa kutuları çizer.
fn render(
    bytes: &Arc<Vec<u8>>,
    width: u32,
    height: u32,
    latest_detections: &SharedDetections,
    detection_enabled: &Arc<AtomicBool>,
) -> iced_image::Handle {
    if detection_enabled.load(Ordering::Relaxed) {
        let detections = latest_detections.lock().unwrap().clone();
        if !detections.is_empty() {
            if let Some(mut rgba) = image::RgbaImage::from_raw(width, height, bytes.to_vec()) {
                draw_detections(&mut rgba, &detections);
                return iced_image::Handle::from_rgba(width, height, rgba.into_raw());
            }
        }
    }

    iced_image::Handle::from_rgba(width, height, bytes.to_vec())
}

/// Kamera olayları (Started / Error / Stopped) UI'a aktarılır.
fn spawn_event_pump(
    event_rx: mpsc::Receiver<CameraEvent>,
    ui_tx: tokio_mpsc::Sender<CameraUpdate>,
) {
    thread::Builder::new()
        .name("camera-event-pump".into())
        .spawn(move || {
            while let Ok(event) = event_rx.recv() {
                let update = match event {
                    CameraEvent::Started => CameraUpdate::Started,
                    CameraEvent::Error(message) => CameraUpdate::Error(message),
                    CameraEvent::Stopped => CameraUpdate::Stopped,
                };
                if ui_tx.blocking_send(update).is_err() {
                    break;
                }
            }
        })
        .expect("camera-event-pump thread başlatılamadı");
}

/// Tespit işçisi — kapalıyken uyur, açıkken 300 ms'de bir çıkarım yapar.
fn spawn_detection_worker(
    ui_tx: tokio_mpsc::Sender<CameraUpdate>,
    latest_frame: SharedFrame,
    latest_detections: SharedDetections,
    detection_enabled: Arc<AtomicBool>,
) {
    thread::Builder::new()
        .name("detection-worker".into())
        .spawn(move || {
            let model_path = if std::path::Path::new(LOCAL_MODEL).exists() {
                LOCAL_MODEL
            } else {
                "" // usls hub'dan indirir
            };

            let mut detector = match YoloDetector::new(model_path) {
                Ok(detector) => detector,
                Err(error) => {
                    eprintln!("Detection model yüklenemedi: {error}");
                    return;
                }
            };

            let interval = Duration::from_millis(DETECTION_INTERVAL_MS);
            let mut last_run = Instant::now()
                .checked_sub(interval)
                .unwrap_or_else(Instant::now);

            loop {
                if !detection_enabled.load(Ordering::Relaxed) {
                    let mut detections = latest_detections.lock().unwrap();
                    let had_boxes = !detections.is_empty();
                    detections.clear();
                    drop(detections);

                    if had_boxes {
                        let _ = ui_tx.blocking_send(CameraUpdate::DetectionCount(0));
                    }
                    thread::sleep(Duration::from_millis(100));
                    continue;
                }

                let elapsed = last_run.elapsed();
                if elapsed < interval {
                    thread::sleep(interval - elapsed);
                }

                let frame = latest_frame.lock().unwrap().take();
                let Some((bytes, width, height)) = frame else {
                    thread::sleep(Duration::from_millis(50));
                    continue;
                };

                last_run = Instant::now();

                let started = Instant::now();
                let detections = match detector.detect(&bytes, width, height) {
                    Ok(detections) => detections,
                    Err(error) => {
                        eprintln!("Detection hatası: {error}");
                        continue;
                    }
                };
                eprintln!(
                    "[detection] inference: {}ms, {} nesne",
                    started.elapsed().as_millis(),
                    detections.len()
                );

                let count = detections.len();
                *latest_detections.lock().unwrap() = detections;
                let _ = ui_tx.blocking_send(CameraUpdate::DetectionCount(count));
            }
        })
        .expect("detection-worker thread başlatılamadı");
}

/// Kamera güncellemelerini taşıyan abonelik.
pub fn updates() -> Subscription<CameraUpdate> {
    Subscription::run(update_stream)
}

fn update_stream() -> impl Stream<Item = CameraUpdate> {
    iced::stream::channel(
        UI_QUEUE,
        |mut output: mpsc_futures::Sender<CameraUpdate>| async move {
            let receiver = UPDATES
                .get()
                .and_then(|slot| slot.lock().ok().and_then(|mut guard| guard.take()));

            let Some(mut receiver) = receiver else {
                return;
            };

            while let Some(update) = receiver.recv().await {
                if output.send(update).await.is_err() {
                    break;
                }
            }
        },
    )
}

/// Bağlı kameraları listeler — `nokhwa::query` bloklar, ayrı thread'e alınır.
pub async fn list_cameras() -> Vec<(usize, String)> {
    tokio::task::spawn_blocking(vision::camera::list_cameras)
        .await
        .unwrap_or_default()
}
