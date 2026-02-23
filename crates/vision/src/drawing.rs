use ab_glyph::{Font, FontArc, GlyphId, ScaleFont};
use image::{Rgba, RgbaImage};
use imageproc::{
    drawing::draw_hollow_rect_mut,
    rect::Rect,
};

use crate::detection::Detection;

/// Sistem fontunu yükler (harici dosya gerekmez)
fn load_font() -> Option<FontArc> {
    #[cfg(target_os = "macos")]
    {
        let candidates = [
            "/System/Library/Fonts/Supplemental/Arial.ttf",
            "/System/Library/Fonts/Monaco.ttf",
            "/System/Library/Fonts/Helvetica.ttc",
        ];
        for path in &candidates {
            if let Ok(bytes) = std::fs::read(path) {
                if let Ok(f) = FontArc::try_from_vec(bytes) {
                    return Some(f);
                }
            }
        }
    }
    #[cfg(target_os = "linux")]
    {
        let candidates = [
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/TTF/DejaVuSans.ttf",
        ];
        for path in &candidates {
            if let Ok(bytes) = std::fs::read(path) {
                if let Ok(f) = FontArc::try_from_vec(bytes) {
                    return Some(f);
                }
            }
        }
    }
    None
}

/// Class ID'ye göre renk üretir (HSV tabanlı, 80 sınıf için eşit aralıklı)
pub fn class_color(class_id: usize) -> Rgba<u8> {
    let hue = (class_id as f32 / 80.0) * 360.0;
    let (r, g, b) = hsv_to_rgb(hue, 0.8, 0.9);
    Rgba([r, g, b, 255])
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;
    let (r1, g1, b1) = match h as u32 {
        0..=59    => (c, x, 0.0),
        60..=119  => (x, c, 0.0),
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

/// Alpha blending: rengi piksele uygular
fn blend_pixel(img: &mut RgbaImage, px: u32, py: u32, color: Rgba<u8>, alpha: f32) {
    if px >= img.width() || py >= img.height() { return; }
    let existing = *img.get_pixel(px, py);
    img.put_pixel(px, py, Rgba([
        (color[0] as f32 * alpha + existing[0] as f32 * (1.0 - alpha)) as u8,
        (color[1] as f32 * alpha + existing[1] as f32 * (1.0 - alpha)) as u8,
        (color[2] as f32 * alpha + existing[2] as f32 * (1.0 - alpha)) as u8,
        255,
    ]));
}

/// RGBA görüntüsü üzerine tespit kutularını çizer
pub fn draw_detections(img: &mut RgbaImage, detections: &[Detection]) {
    // Font başlangıçta yüklenir; None ise fallback kullanılır
    let font = load_font();

    for det in detections {
        let color  = class_color(det.class_id);
        let x      = det.x1 as i32;
        let y      = det.y1 as i32;
        let width  = (det.x2 - det.x1).max(1.0) as u32;
        let height = (det.y2 - det.y1).max(1.0) as u32;

        // Bounding box: 2px kalınlık
        let rect = Rect::at(x, y).of_size(width, height);
        draw_hollow_rect_mut(img, rect, color);
        if width > 2 && height > 2 {
            draw_hollow_rect_mut(
                img,
                Rect::at(x + 1, y + 1).of_size(width - 2, height - 2),
                color,
            );
        }

        // Etiket — class_name usls'den geliyor (auto-detect from ONNX metadata)
        let label = format!("{} {:.0}%", det.class_name, det.confidence * 100.0);
        let label_y = y.saturating_sub(20).max(0);

        if let Some(ref f) = font {
            draw_text_glyph(img, x, label_y, &label, color, f, 13.0);
        } else {
            draw_text_minimal(img, x, label_y, &label, color);
        }
    }
}

/// ab_glyph ile gerçek metin çizimi (sistem fontu varsa)
fn draw_text_glyph(
    img: &mut RgbaImage,
    x: i32,
    y: i32,
    text: &str,
    color: Rgba<u8>,
    font: &FontArc,
    font_size: f32,
) {
    use ab_glyph::{point, PxScale};

    let scale  = PxScale::from(font_size);
    let scaled = font.as_scaled(scale);
    let ascent = scaled.ascent();

    // Metin genişliği (arka plan kutusu için)
    let text_w: f32 = text.chars().map(|c| scaled.h_advance(font.glyph_id(c))).sum();
    let text_h = font_size + 4.0;

    // Yarı-saydam arka plan
    let bx = x.max(0) as u32;
    let by = y.max(0) as u32;
    let bw = (text_w as u32 + 6).min(img.width().saturating_sub(bx));
    let bh = (text_h as u32 + 2).min(img.height().saturating_sub(by));
    for dy in 0..bh {
        for dx in 0..bw {
            blend_pixel(img, bx + dx, by + dy, Rgba([0, 0, 0, 255]), 0.6);
        }
    }

    // Glyph rasterizasyonu
    let mut pen_x      = x as f32 + 3.0;
    let baseline_y     = y as f32 + ascent + 2.0;
    let mut prev: Option<GlyphId> = None;

    for c in text.chars() {
        let gid = font.glyph_id(c);
        if let Some(p) = prev { pen_x += scaled.kern(p, gid); }

        let glyph = gid.with_scale_and_position(scale, point(pen_x, baseline_y));
        pen_x += scaled.h_advance(gid);
        prev = Some(gid);

        if let Some(outlined) = font.outline_glyph(glyph) {
            let bounds = outlined.px_bounds();
            outlined.draw(|gx, gy, cov| {
                let px = bounds.min.x as i32 + gx as i32;
                let py = bounds.min.y as i32 + gy as i32;
                if px >= 0 && py >= 0 {
                    blend_pixel(img, px as u32, py as u32, color, cov);
                }
            });
        }
    }
}

/// Fallback: arka plan + minimal piksel işaretleri (font yoksa)
fn draw_text_minimal(img: &mut RgbaImage, x: i32, y: i32, text: &str, color: Rgba<u8>) {
    let char_w = 7i32;
    let char_h = 12i32;
    let tw = text.len() as i32 * char_w;

    // Arka plan
    for dy in 0..(char_h + 4) {
        for dx in 0..(tw + 4) {
            let px = (x + dx).max(0) as u32;
            let py = (y + dy).max(0) as u32;
            blend_pixel(img, px, py, Rgba([0, 0, 0, 255]), 0.6);
        }
    }

    // Her karakter için 2-piksel nokta (okunabilirlik için)
    for (i, _) in text.chars().enumerate() {
        let px = (x + 2 + i as i32 * char_w).max(0) as u32;
        let py = (y + 4).max(0) as u32;
        blend_pixel(img, px, py, color, 1.0);
        if py + 1 < img.height() { blend_pixel(img, px, py + 1, color, 1.0); }
    }
}
