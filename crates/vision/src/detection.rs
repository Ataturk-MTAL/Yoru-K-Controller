use anyhow::{anyhow, Result};
use image::{ImageBuffer, Rgb, Rgba};
use ort::session::{Session, builder::GraphOptimizationLevel};

/// Nesne tespiti sonucu
#[derive(Debug, Clone)]
pub struct Detection {
    pub x1:         f32,
    pub y1:         f32,
    pub x2:         f32,
    pub y2:         f32,
    pub confidence: f32,
    pub class_id:   usize,
}

/// YOLOv8 ONNX dedektörü
pub struct YoloDetector {
    session:        Session,
    pub conf_thr:   f32,
    pub iou_thr:    f32,
}

const INPUT_SIZE: u32 = 640;

impl YoloDetector {
    /// ONNX model dosyasından dedektör oluşturur
    pub fn new(model_path: &str) -> Result<Self> {
        let session = Session::builder()?
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            .with_intra_threads(4)?
            .commit_from_file(model_path)
            .map_err(|e| anyhow!("Model yüklenemedi: {e}"))?;

        Ok(Self { session, conf_thr: 0.5, iou_thr: 0.45 })
    }

    /// RGBA8 frame üzerinde nesne tespiti yapar
    /// Döndürür: orijinal koordinatlara ölçeklendirilmiş Detection listesi
    pub fn detect(&mut self, frame_rgba: &[u8], orig_w: u32, orig_h: u32) -> Result<Vec<Detection>> {
        // 1. RGBA → RGB
        let rgb_img: ImageBuffer<Rgb<u8>, Vec<u8>> = {
            let rgba = ImageBuffer::<Rgba<u8>, _>::from_raw(orig_w, orig_h, frame_rgba.to_vec())
                .ok_or_else(|| anyhow!("Frame boyutu hatalı"))?;
            ImageBuffer::from_fn(orig_w, orig_h, |x, y| {
                let p = rgba.get_pixel(x, y);
                Rgb([p[0], p[1], p[2]])
            })
        };

        // 2. Letterbox resize: 640×640, aspect-ratio koruyarak
        let (resized, scale, pad_x, pad_y) = letterbox(&rgb_img, INPUT_SIZE);

        // 3. HWC → CHW, normalize 0–255 → 0.0–1.0 → flat Vec<f32>
        let sz = INPUT_SIZE as usize;
        let mut input_data: Vec<f32> = vec![0.0; 3 * sz * sz];
        for (y, row) in resized.rows().enumerate() {
            for (x, pixel) in row.enumerate() {
                input_data[0 * sz * sz + y * sz + x] = pixel[0] as f32 / 255.0;
                input_data[1 * sz * sz + y * sz + x] = pixel[1] as f32 / 255.0;
                input_data[2 * sz * sz + y * sz + x] = pixel[2] as f32 / 255.0;
            }
        }

        // 4. ORT inference — (shape_vec, data) API
        let shape: Vec<i64> = vec![1, 3, sz as i64, sz as i64];
        let input_tensor = ort::value::Tensor::<f32>::from_array((shape, input_data))?;
        let outputs = self.session.run(ort::inputs!["images" => input_tensor])?;

        // try_extract_tensor döner: (&Shape, &[T])
        // Shape: [i64] dizisi
        let (out_shape, data) = outputs["output0"].try_extract_tensor::<f32>()?;

        // 5. YOLOv8 çıktı: shape = [1, 84, 8400]
        if out_shape.len() < 3 {
            return Err(anyhow!("Beklenmedik çıktı şekli: {:?}", out_shape));
        }

        let num_features = out_shape[1] as usize; // 84
        let num_anchors  = out_shape[2] as usize; // 8400
        let num_classes  = num_features.saturating_sub(4);

        let mut detections: Vec<Detection> = Vec::new();

        for i in 0..num_anchors {
            // row-major: data[feature * num_anchors + anchor]
            let cx = data[0 * num_anchors + i];
            let cy = data[1 * num_anchors + i];
            let w  = data[2 * num_anchors + i];
            let h  = data[3 * num_anchors + i];

            // En yüksek class confidence
            let (class_id, conf) = (0..num_classes)
                .map(|c| (c, data[(4 + c) * num_anchors + i]))
                .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
                .unwrap_or((0, 0.0f32));

            if conf < self.conf_thr { continue; }

            // cx/cy/w/h (640px) → x1/y1/x2/y2 (orijinal koordinatlar)
            let x1_pad = cx - w / 2.0;
            let y1_pad = cy - h / 2.0;
            let x2_pad = cx + w / 2.0;
            let y2_pad = cy + h / 2.0;

            let x1 = ((x1_pad - pad_x) / scale).clamp(0.0, orig_w as f32);
            let y1 = ((y1_pad - pad_y) / scale).clamp(0.0, orig_h as f32);
            let x2 = ((x2_pad - pad_x) / scale).clamp(0.0, orig_w as f32);
            let y2 = ((y2_pad - pad_y) / scale).clamp(0.0, orig_h as f32);

            detections.push(Detection { x1, y1, x2, y2, confidence: conf, class_id });
        }

        // 6. NMS
        Ok(nms(detections, self.iou_thr))
    }
}

/// Letterbox: görüntüyü aspect-ratio koruyarak `size`×`size` alana sığdırır
/// Döndürür: (yeniden boyutlandırılmış görüntü, ölçek faktörü, x padding, y padding)
fn letterbox(img: &ImageBuffer<Rgb<u8>, Vec<u8>>, size: u32) -> (ImageBuffer<Rgb<u8>, Vec<u8>>, f32, f32, f32) {
    let (w, h) = (img.width(), img.height());
    let scale  = (size as f32 / w as f32).min(size as f32 / h as f32);
    let new_w  = (w as f32 * scale) as u32;
    let new_h  = (h as f32 * scale) as u32;
    let pad_x  = (size - new_w) as f32 / 2.0;
    let pad_y  = (size - new_h) as f32 / 2.0;

    let resized = image::imageops::resize(img, new_w, new_h, image::imageops::FilterType::Lanczos3);

    let mut canvas = ImageBuffer::from_pixel(size, size, Rgb([114u8, 114, 114]));
    image::imageops::overlay(&mut canvas, &resized, pad_x as i64, pad_y as i64);

    (canvas, scale, pad_x, pad_y)
}

/// IoU (Intersection over Union) hesaplar
fn iou(a: &Detection, b: &Detection) -> f32 {
    let ix1 = a.x1.max(b.x1);
    let iy1 = a.y1.max(b.y1);
    let ix2 = a.x2.min(b.x2);
    let iy2 = a.y2.min(b.y2);

    let inter = (ix2 - ix1).max(0.0) * (iy2 - iy1).max(0.0);
    let area_a = (a.x2 - a.x1) * (a.y2 - a.y1);
    let area_b = (b.x2 - b.x1) * (b.y2 - b.y1);
    let union  = area_a + area_b - inter;

    if union > 0.0 { inter / union } else { 0.0 }
}

/// Non-Maximum Suppression
fn nms(mut detections: Vec<Detection>, iou_thr: f32) -> Vec<Detection> {
    detections.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
    let mut suppressed = vec![false; detections.len()];
    let mut keep = Vec::new();

    for i in 0..detections.len() {
        if suppressed[i] { continue; }
        keep.push(detections[i].clone());
        for j in (i + 1)..detections.len() {
            if detections[i].class_id == detections[j].class_id
                && iou(&detections[i], &detections[j]) > iou_thr
            {
                suppressed[j] = true;
            }
        }
    }
    keep
}
