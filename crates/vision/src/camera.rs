use std::{
    sync::{Arc, mpsc::SyncSender},
    thread,
};

use nokhwa::{
    Camera,
    pixel_format::RgbAFormat,
    utils::{CameraIndex, RequestedFormat, RequestedFormatType},
};

/// Ham kamera karesi: (RGBA8 baytlar, genişlik, yükseklik)
pub type RgbaFrame = (Arc<Vec<u8>>, u32, u32);

/// Kamera worker thread'i başlatır
/// `frame_tx` kapasitesi 1 olmalı → meşgulken kare düşürülür (Python m_busy pattern)
pub fn spawn_camera(
    camera_index: usize,
    frame_tx: SyncSender<RgbaFrame>,
) -> thread::JoinHandle<()> {
    thread::Builder::new()
        .name("camera-worker".into())
        .spawn(move || {
            let fmt = RequestedFormat::new::<RgbAFormat>(
                RequestedFormatType::AbsoluteHighestFrameRate,
            );

            let mut cam = match Camera::new(
                CameraIndex::Index(camera_index as u32),
                fmt,
            ) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Kamera açılamadı (index {camera_index}): {e}");
                    return;
                }
            };

            if let Err(e) = cam.open_stream() {
                eprintln!("Kamera stream açılamadı: {e}");
                return;
            }

            loop {
                match cam.frame() {
                    Ok(buf) => {
                        match buf.decode_image::<RgbAFormat>() {
                            Ok(decoded) => {
                                let w = decoded.width();
                                let h = decoded.height();
                                let bytes = Arc::new(decoded.into_raw());
                                // try_send: kanal doluysa kareyi düşür
                                let _ = frame_tx.try_send((bytes, w, h));
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

            let _ = cam.stop_stream();
        })
        .expect("camera-worker thread başlatılamadı")
}

/// Bağlı kamera listesi döndürür: (index, isim)
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
