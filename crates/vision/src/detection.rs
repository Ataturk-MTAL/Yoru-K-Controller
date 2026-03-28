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
    /// ORT config + YOLO config'i verilen device ile derler.
    fn build(model_path: &str, device: usls::Device) -> Result<Runtime<YOLO>> {
        // macOS CoreML: 1 dry-run → model ANE/GPU için JIT compile edilir
        // Diğer: 0 dry-run
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
        #[cfg(target_os = "macos")]
        {
            config = config
                .with_model_coreml_static_input_shapes(true)
                .with_model_coreml_compute_units(2)
                .with_model_coreml_model_format(0);
        }

        let config = config.commit()?;
        YOLO::new(config)
    }

    /// ONNX model dosyasından dedektör oluşturur.
    /// `model_path`: yerel dosya yolu veya "" (usls hub'dan otomatik indirir)
    pub fn new(model_path: &str) -> Result<Self> {
        // Windows: DirectML dene → desteklenmiyorsa CPU'ya düş
        #[cfg(target_os = "windows")]
        {
            let dml = usls::Device::DirectMl(0);
            eprintln!("[detection] Execution provider: {:?}", dml);
            match Self::build(model_path, dml) {
                Ok(runtime) => return Ok(Self { runtime }),
                Err(e) => {
                    eprintln!("[detection] DirectML başarısız, CPU'ya geçiliyor: {e}");
                    let runtime = Self::build(model_path, usls::Device::Cpu(0))?;
                    eprintln!("[detection] Execution provider: Cpu(0)");
                    return Ok(Self { runtime });
                }
            }
        }

        // macOS: CoreML
        #[cfg(target_os = "macos")]
        let device = usls::Device::CoreMl;

        // Linux / diğer
        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        let device = usls::Device::Cpu(0);

        #[allow(unreachable_code)]
        {
            eprintln!("[detection] Execution provider: {:?}", device);
            let runtime = Self::build(model_path, device)?;
            Ok(Self { runtime })
        }
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
                let x1 = hbb.x();
                let y1 = hbb.y();
                let x2 = x1 + hbb.w();
                let y2 = y1 + hbb.h();

                let confidence = hbb.confidence().unwrap_or(0.0);
                let class_id   = hbb.id().unwrap_or(0);
                let class_name = hbb.name().unwrap_or("?").to_string();

                detections.push(Detection { x1, y1, x2, y2, confidence, class_id, class_name });
            }
        }

        Ok(detections)
    }
}
