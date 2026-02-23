pub mod camera;
pub mod detection;
pub mod drawing;

pub use camera::*;
pub use detection::*;
pub use drawing::*;

use std::sync::{Arc, Mutex};

/// En son kamera frame'ini detection thread ile paylaşmak için.
/// frame-bridge yazar, detection-worker alır (take ile).
pub type SharedFrame = Arc<Mutex<Option<RgbaFrame>>>;

/// En son detection sonuçlarını frame-bridge ile paylaşmak için.
/// detection-worker yazar, frame-bridge okur (clone).
pub type SharedDetections = Arc<Mutex<Vec<Detection>>>;
