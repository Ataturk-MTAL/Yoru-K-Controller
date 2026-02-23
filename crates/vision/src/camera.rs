use std::{
    sync::{Arc, atomic::{AtomicBool, Ordering}, mpsc::SyncSender},
    thread,
};

use nokhwa::{
    Camera,
    pixel_format::RgbAFormat,
    utils::{CameraFormat, CameraIndex, FrameFormat, RequestedFormat, RequestedFormatType, Resolution},
};

/// Ham kamera karesi: (RGBA8 baytlar, genişlik, yükseklik)
pub type RgbaFrame = (Arc<Vec<u8>>, u32, u32);

/// Kamera thread'inden UI'a gönderilen olaylar
pub enum CameraEvent {
    /// Kamera başarıyla açıldı
    Started,
    /// Kamera açılamadı
    Error(String),
    /// Kamera durduruldu (stop_flag veya hata ile)
    Stopped,
}

/// Kamera worker thread'i başlatır.
/// `frame_tx` kapasitesi 1 olmalı → meşgulken kare düşürülür.
/// `stop_flag` true yapılınca thread çıkar, `cam.stop_stream()` çalışır (donanım serbest).
/// `event_tx` ile başlatma başarısı/hatası bildirilir.
pub fn spawn_camera(
    camera_index: usize,
    frame_tx: SyncSender<RgbaFrame>,
    stop_flag: Arc<AtomicBool>,
    event_tx: std::sync::mpsc::Sender<CameraEvent>,
) -> thread::JoinHandle<()> {
    thread::Builder::new()
        .name("camera-worker".into())
        .spawn(move || {
            // ── Kamera başlatma (blocking — ama ayrı thread'de) ──
            // Closest(640x480, YUYV, 30fps): düşük çözünürlük → decode CPU yükü azalır.
            // Kamera bu formatı desteklemiyorsa en yakın olanı seçer.
            let fmt = RequestedFormat::new::<RgbAFormat>(
                RequestedFormatType::Closest(CameraFormat::new(
                    Resolution::new(640, 480),
                    FrameFormat::YUYV,
                    30,
                ))
            );

            let mut cam = match Camera::new(CameraIndex::Index(camera_index as u32), fmt) {
                Ok(c) => c,
                Err(e) => {
                    let _ = event_tx.send(CameraEvent::Error(
                        format!("Kamera açılamadı (index {camera_index}): {e}")
                    ));
                    return;
                }
            };

            // Stop flag: Camera::new sırasında kullanıcı durdurdu mu?
            if stop_flag.load(Ordering::Relaxed) {
                let _ = event_tx.send(CameraEvent::Stopped);
                return;
            }

            if let Err(e) = cam.open_stream() {
                let _ = event_tx.send(CameraEvent::Error(
                    format!("Kamera stream açılamadı: {e}")
                ));
                return;
            }

            // Seçilen kamera formatını logla
            {
                let fmt = cam.camera_format();
                eprintln!("[camera] Açık format: {}x{} {:?} {}fps",
                    fmt.resolution().width_x,
                    fmt.resolution().height_y,
                    fmt.format(),
                    fmt.frame_rate(),
                );
            }
            let _ = event_tx.send(CameraEvent::Started);

            // ── Frame döngüsü ─────────────────────────────────────
            // cam.frame() → frame_raw() → channel.recv() — AVFoundation'da BLOCKING.
            // Yani busy-loop yok: kamera yeni frame üretene kadar thread uyur.
            // chan_full: bridge henüz frame'i almadıysa decode atlanır (CPU tasarrufu)
            let mut chan_full = false;
            loop {
                // Durdurma sinyali — her frame'den önce kontrol et
                if stop_flag.load(Ordering::Relaxed) {
                    break;
                }

                // Blocking: yeni kamera frame'i gelene kadar bekler (CPU kullanmaz)
                match cam.frame() {
                    Ok(buf) => {
                        if stop_flag.load(Ordering::Relaxed) { break; }

                        // Kanal doluysa decode'u atla — boşuna CPU harcama
                        // cam.frame() zaten blocking, bir sonraki frame'de tekrar gelir
                        if chan_full {
                            chan_full = false; // reset: bridge'in almasına izin ver
                            continue;
                        }

                        // Decode (YUYV→RGBA) — sadece kanal boşken yapılır
                        match buf.decode_image::<RgbAFormat>() {
                            Ok(decoded) => {
                                let w = decoded.width();
                                let h = decoded.height();
                                let bytes = Arc::new(decoded.into_raw());
                                if frame_tx.try_send((bytes, w, h)).is_err() {
                                    chan_full = true;
                                }
                            }
                            Err(e) => eprintln!("Kare decode hatası: {e}"),
                        }
                    }
                    Err(e) => {
                        eprintln!("Kamera okuma hatası: {e}");
                        break;
                    }
                }
            }

            // ── Temizlik — donanım kaynakları serbest bırakılır ──
            let _ = cam.stop_stream();
            let _ = event_tx.send(CameraEvent::Stopped);
        })
        .expect("camera-worker thread başlatılamadı")
}

/// Bağlı kameraları listeler: (index, isim)
/// Not: Bu çağrı birkaç saniye sürebilir — arka plan thread'inden çağır.
pub fn list_cameras() -> Vec<(usize, String)> {
    nokhwa::query(nokhwa::utils::ApiBackend::Auto)
        .unwrap_or_default()
        .into_iter()
        .map(|info| {
            let idx = match info.index() {
                CameraIndex::Index(i) => *i as usize,
                CameraIndex::String(s) => s.parse().unwrap_or(0),
            };
            (idx, info.human_name().to_string())
        })
        .collect()
}
