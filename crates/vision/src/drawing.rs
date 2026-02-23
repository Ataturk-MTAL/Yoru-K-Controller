use image::{Rgba, RgbaImage};
use imageproc::{
    drawing::draw_hollow_rect_mut,
    rect::Rect,
};

use crate::detection::Detection;

/// COCO sınıf isimleri (80 sınıf)
pub const COCO_CLASSES: &[&str] = &[
    "person", "bicycle", "car", "motorcycle", "airplane", "bus", "train", "truck",
    "boat", "traffic light", "fire hydrant", "stop sign", "parking meter", "bench",
    "bird", "cat", "dog", "horse", "sheep", "cow", "elephant", "bear", "zebra",
    "giraffe", "backpack", "umbrella", "handbag", "tie", "suitcase", "frisbee",
    "skis", "snowboard", "sports ball", "kite", "baseball bat", "baseball glove",
    "skateboard", "surfboard", "tennis racket", "bottle", "wine glass", "cup",
    "fork", "knife", "spoon", "bowl", "banana", "apple", "sandwich", "orange",
    "broccoli", "carrot", "hot dog", "pizza", "donut", "cake", "chair", "couch",
    "potted plant", "bed", "dining table", "toilet", "tv", "laptop", "mouse",
    "remote", "keyboard", "cell phone", "microwave", "oven", "toaster", "sink",
    "refrigerator", "book", "clock", "vase", "scissors", "teddy bear", "hair drier",
    "toothbrush",
];

/// Class ID'ye göre renk üretir (HSV tabanlı)
pub fn class_color(class_id: usize) -> Rgba<u8> {
    // 80 sınıf için eşit aralıklı renkler
    let hue = (class_id as f32 / 80.0) * 360.0;
    let (r, g, b) = hsv_to_rgb(hue, 0.8, 0.9);
    Rgba([r, g, b, 255])
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;
    let (r1, g1, b1) = match h as u32 {
        0..=59   => (c, x, 0.0),
        60..=119 => (x, c, 0.0),
        120..=179 => (0.0, c, x),
        180..=239 => (0.0, x, c),
        240..=299 => (x, 0.0, c),
        _          => (c, 0.0, x),
    };
    (
        ((r1 + m) * 255.0) as u8,
        ((g1 + m) * 255.0) as u8,
        ((b1 + m) * 255.0) as u8,
    )
}

/// RGBA görüntüsü üzerine tespit kutularını çizer (OpenCV'siz)
pub fn draw_detections(img: &mut RgbaImage, detections: &[Detection]) {
    // Font: ab_glyph ile veya basit imageproc metin
    // Şimdilik sadece kutu çizimi; metin için fontconfig gerekebilir
    for det in detections {
        let color  = class_color(det.class_id);
        let x      = det.x1 as i32;
        let y      = det.y1 as i32;
        let width  = (det.x2 - det.x1).max(1.0) as u32;
        let height = (det.y2 - det.y1).max(1.0) as u32;

        let rect = Rect::at(x, y).of_size(width, height);

        // 2px kalınlık için iç içe iki kutu
        draw_hollow_rect_mut(img, rect, color);
        if width > 2 && height > 2 {
            draw_hollow_rect_mut(
                img,
                Rect::at(x + 1, y + 1).of_size(width - 2, height - 2),
                color,
            );
        }

        // Güven skoru etiketi (sol üst köşe, basit piksel metin)
        let label = format!(
            "{} {:.0}%",
            COCO_CLASSES.get(det.class_id).unwrap_or(&"?"),
            det.confidence * 100.0
        );
        draw_label(img, x, y.saturating_sub(16), &label, color);
    }
}

/// Basit piksel metin çizimi (harici font gerektirmez)
fn draw_label(img: &mut RgbaImage, x: i32, y: i32, text: &str, color: Rgba<u8>) {
    // Her karakter için 6×8 piksel yer ayır (basit bitmap font simülasyonu)
    // Gerçek font için ab_glyph entegrasyonu ileride eklenebilir
    let bg = Rgba([0u8, 0, 0, 180]);
    let char_w = 6i32;
    let char_h = 8i32;
    let text_w = text.len() as i32 * char_w;

    // Arka plan kutusu
    for dy in 0..char_h + 2 {
        for dx in 0..text_w + 4 {
            let px = x + dx;
            let py = y + dy;
            if px >= 0 && py >= 0 && px < img.width() as i32 && py < img.height() as i32 {
                let existing = img.get_pixel(px as u32, py as u32);
                // Alpha blending
                let alpha = bg[3] as f32 / 255.0;
                let blended = Rgba([
                    (bg[0] as f32 * alpha + existing[0] as f32 * (1.0 - alpha)) as u8,
                    (bg[1] as f32 * alpha + existing[1] as f32 * (1.0 - alpha)) as u8,
                    (bg[2] as f32 * alpha + existing[2] as f32 * (1.0 - alpha)) as u8,
                    255,
                ]);
                img.put_pixel(px as u32, py as u32, blended);
            }
        }
    }

    // Metin rengi (tek piksel noktalar — yeterince küçük çözünürlükte okunabilir)
    for (i, _ch) in text.chars().enumerate() {
        let px = x + 2 + i as i32 * char_w;
        let py = y + 2;
        if px >= 0 && py >= 0 && px < img.width() as i32 && py + char_h < img.height() as i32 {
            // Sadece üst piksel satırını renklendir (minimal gösterge)
            img.put_pixel(px as u32, py as u32, color);
        }
    }
}
