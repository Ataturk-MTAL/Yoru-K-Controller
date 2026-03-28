use anyhow::Result;
use usls::{models::YOLO, Config, Model, ORTConfig, Runtime, Scale, Version, Y};

/// Nesne tespiti sonucu (bizim ara format — bridge.rs'e geçirilir)
#[derive(Debug, Clone)]
pub struct Detection {
    pub x1:         f32,
    pub y1:         f32,
    pub x2:         f32,
    pub y2:         f32,
    pub confidence: f32,
    pub class_id:   usize,
    pub class_name: String,
}

/// usls YOLO dedektörü
pub struct YoloDetector {
    runtime: Runtime<YOLO>,
}

impl YoloDetector {
    /// Platform'a göre en iyi execution provider seçer.
    /// Cargo.toml'daki platform bazlı usls feature'larıyla eşleşir:
    ///   macOS   → CoreML (ANE/GPU)
    ///   Windows → DirectML (GPU)
    ///   Linux   → CPU
    fn best_device() -> usls::Device {
        #[cfg(target_os = "macos")]
        { return usls::Device::CoreMl; }

        #[cfg(target_os = "windows")]
        { return usls::Device::DirectMl(0); }

        #[allow(unreachable_code)]
        usls::Device::Cpu(0)
    }

    /// ONNX model dosyasından dedektör oluşturur.
    /// `model_path`: yerel dosya yolu veya "" (usls hub'dan otomatik indirir)
    pub fn new(model_path: &str) -> Result<Self> {
        let device = Self::best_device();
        eprintln!("[detection] Execution provider: {:?}", device);

        // macOS CoreML: 1 dry-run → model ANE/GPU için JIT compile edilir
        // Diğer: 0 dry-run (ORT dynamic shape Concat bug workaround)
        #[cfg(target_os = "macos")]
        let num_dry: usize = 1;
        #[cfg(not(target_os = "macos"))]
        let num_dry: usize = 0;

        let ort_cfg = ORTConfig::default()
            .with_file(model_path)
            .with_device(device)
            .with_dtype(usls::DType::Fp32)
            .with_num_dry_run(num_dry)
            .with_num_intra_threads(2)
            .with_num_inter_threads(1);

        // YOLO26: end-to-end NMS, output [1, 300, 6]
        // with_model_ixx: sabit 640×640 — ORT/CoreML static shape için zorunlu
        let mut config = Config::yolo_detect()
            .with_version(Version::from(26_u8))
            .with_scale(Scale::N)
            .with_model(ort_cfg)
            .with_model_ixx(0, 0, 1)    // batch = 1
            .with_model_ixx(0, 1, 3)    // channels = 3 (RGB)
            .with_model_ixx(0, 2, 640)  // height = 640
            .with_model_ixx(0, 3, 640)  // width = 640
            .with_class_confs(&[0.5]);

        // macOS CoreML: ANE için optimize
        // compute_units=2 → CPUAndNeuralEngine
        // static_input_shapes=true → sabit 640×640 ile graph compile
        // model_format=0 → MLProgram (Apple Silicon için optimize)
        #[cfg(target_os = "macos")]
        {
            config = config
                .with_model_coreml_static_input_shapes(true)
                .with_model_coreml_compute_units(2)
                .with_model_coreml_model_format(0);
        }

        let config = config.commit()?;
        let runtime = YOLO::new(config)?;
        Ok(Self { runtime })
    }

    /// RGBA8 frame üzerinde nesne tespiti yapar.
    /// Döndürür: orijinal koordinatlara ölçeklendirilmiş Detection listesi
    pub fn detect(&mut self, frame_rgba: &[u8], orig_w: u32, orig_h: u32) -> Result<Vec<Detection>> {
        // RGBA → RGB (usls Image::from_u8s RGB bekliyor: width×height×3)
        let rgb_bytes: Vec<u8> = frame_rgba.chunks_exact(4)
            .flat_map(|p| [p[0], p[1], p[2]])
            .collect();

        let img = usls::Image::from_u8s(&rgb_bytes, orig_w, orig_h)?;

        // usls inference — letterbox + normalize + NMS + decode içerde
        let results: Vec<Y> = self.runtime.forward(&[img])?;

        let mut detections = Vec::new();
        if let Some(y) = results.into_iter().next() {
            for hbb in &y.hbbs {
                // usls Hbb: x, y, w, h (top-left origin) → f32
                let x1 = hbb.x();
                let y1 = hbb.y();
                let x2 = x1 + hbb.w();
                let y2 = y1 + hbb.h();

                // confidence ve id Option<T> döndürür
                let confidence = hbb.confidence().unwrap_or(0.0);
                let class_id   = hbb.id().unwrap_or(0);
                let class_name = hbb.name().unwrap_or("?").to_string();

                detections.push(Detection { x1, y1, x2, y2, confidence, class_id, class_name });
            }
        }

        Ok(detections)
    }
}
