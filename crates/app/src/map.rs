//! OpenStreetMap tile haritası — `crates/app/src/map.rs`'in iced portu.
//!
//! Slint sürümünde tile'lar `Flickable` içinde `Image` widget'larıydı ve
//! yükleme `slint::spawn_local` ile sürülüyordu. iced'de tile'lar tek bir
//! `Canvas` üzerine çizilir, indirme ise `Task::perform` ile yapılır.
//!
//! Koordinat modeli: `offset_x/offset_y`, görünür alanın sol-üst köşesinin
//! **dünya pikseli** cinsinden konumudur (dünya boyutu `256 * 2^zoom`).

use std::collections::{BTreeMap, HashSet};
use std::sync::OnceLock;

use iced::widget::image as iced_image;
use iced::{Point, Rectangle, Size};

/// OSM tile kenar uzunluğu.
pub const TILE_SIZE: f64 = 256.0;
pub const MIN_ZOOM: u32 = 1;
pub const MAX_ZOOM: u32 = 19;

/// Görünür alanın dışında kaç tile halkası bellekte tutulur.
const KEEP_RINGS: i64 = 2;

/// Varsayılan merkez — Mersin (Slint sürümüyle aynı).
const DEFAULT_LAT: f64 = 36.8121;
const DEFAULT_LON: f64 = 34.6415;
const DEFAULT_ZOOM: u32 = 15;

static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TileCoord {
    pub z: u32,
    pub x: i64,
    pub y: i64,
}

pub struct MapState {
    pub zoom: u32,
    pub offset_x: f64,
    pub offset_y: f64,
    pub viewport: Size,
    pub tiles: BTreeMap<TileCoord, iced_image::Handle>,
    /// İndirilmesi istenmiş ama henüz gelmemiş tile'lar — çift istek olmasın.
    pending: HashSet<TileCoord>,
    /// İndirilemeyen tile'lar — canvas oraya "yok" iskeleti çizer.
    ///
    /// Sadece çizim durumu; yeniden denemeyi engellemez. Tile yeniden
    /// istendiğinde (`take_missing_tiles`) buradan düşer, yoksa bir kez patlayan
    /// tile ömür boyu boş kalırdı.
    failed: HashSet<TileCoord>,
}

impl Default for MapState {
    fn default() -> Self {
        let mut state = Self {
            zoom: DEFAULT_ZOOM,
            offset_x: 0.0,
            offset_y: 0.0,
            viewport: Size::new(800.0, 600.0),
            tiles: BTreeMap::new(),
            pending: HashSet::new(),
            failed: HashSet::new(),
        };
        state.center_on(DEFAULT_LAT, DEFAULT_LON);
        state
    }
}

impl MapState {
    /// Verilen koordinatı görünür alanın ortasına alır.
    pub fn center_on(&mut self, lat: f64, lon: f64) {
        let (world_x, world_y) = lat_lon_to_world(lat, lon, self.zoom);
        self.offset_x = world_x - self.viewport.width as f64 / 2.0;
        self.offset_y = world_y - self.viewport.height as f64 / 2.0;
    }

    /// Fare sürüklemesi — harita imleçle birlikte hareket eder.
    pub fn pan(&mut self, dx: f32, dy: f32) {
        self.offset_x -= dx as f64;
        self.offset_y -= dy as f64;
        self.clamp_to_world();
    }

    /// Zoom değiştirir; `pivot` altındaki coğrafi nokta yerinde kalır.
    pub fn zoom_at(&mut self, zoom: u32, pivot: Point) {
        let zoom = zoom.clamp(MIN_ZOOM, MAX_ZOOM);
        if zoom == self.zoom {
            return;
        }

        let scale = 2f64.powi(zoom as i32 - self.zoom as i32);
        self.offset_x = (self.offset_x + pivot.x as f64) * scale - pivot.x as f64;
        self.offset_y = (self.offset_y + pivot.y as f64) * scale - pivot.y as f64;
        self.zoom = zoom;

        // Zoom seviyesi değişince eski tile'lar geçersiz.
        self.tiles.clear();
        self.pending.clear();
        self.failed.clear();
        self.clamp_to_world();
    }

    pub fn set_viewport(&mut self, size: Size) {
        self.viewport = size;
        self.clamp_to_world();
    }

    /// Dünya boyutu (piksel).
    pub fn world_size(&self) -> f64 {
        TILE_SIZE * (1i64 << self.zoom) as f64
    }

    /// Görünür alanı dünya sınırları içinde tutar — boş kenar oluşmaz.
    fn clamp_to_world(&mut self) {
        let world = self.world_size();
        let max_x = (world - self.viewport.width as f64).max(0.0);
        let max_y = (world - self.viewport.height as f64).max(0.0);
        self.offset_x = self.offset_x.clamp(0.0, max_x);
        self.offset_y = self.offset_y.clamp(0.0, max_y);
    }

    /// Görünür alandaki tile koordinatları.
    pub fn visible_tiles(&self) -> Vec<TileCoord> {
        let count = 1i64 << self.zoom;
        let min_x = (self.offset_x / TILE_SIZE).floor() as i64;
        let min_y = (self.offset_y / TILE_SIZE).floor() as i64;
        let max_x = ((self.offset_x + self.viewport.width as f64) / TILE_SIZE).ceil() as i64;
        let max_y = ((self.offset_y + self.viewport.height as f64) / TILE_SIZE).ceil() as i64;

        let mut coords = Vec::new();
        for x in min_x..max_x {
            for y in min_y..max_y {
                if x < 0 || y < 0 || x >= count || y >= count {
                    continue;
                }
                coords.push(TileCoord { z: self.zoom, x, y });
            }
        }
        coords
    }

    /// Henüz elde olmayan tile'ları döndürür ve bekleyenler listesine ekler.
    ///
    /// Uzaktaki tile'lar aynı adımda bellekten düşürülür.
    pub fn take_missing_tiles(&mut self) -> Vec<TileCoord> {
        self.prune();

        let mut missing = Vec::new();
        for coord in self.visible_tiles() {
            if self.tiles.contains_key(&coord) || self.pending.contains(&coord) {
                continue;
            }
            self.pending.insert(coord);
            // Yeniden deniyoruz: artık "başarısız" değil, "yükleniyor".
            self.failed.remove(&coord);
            missing.push(coord);
        }
        missing
    }

    /// İnen tile'ı yerleştirir.
    pub fn insert_tile(&mut self, coord: TileCoord, handle: iced_image::Handle) {
        self.pending.remove(&coord);
        self.failed.remove(&coord);
        if coord.z == self.zoom {
            self.tiles.insert(coord, handle);
        }
    }

    /// İndirilemeyen tile'ı bekleyenlerden düşürür — sonraki denemeye açık kalır.
    pub fn drop_pending(&mut self, coord: TileCoord) {
        self.pending.remove(&coord);
        self.failed.insert(coord);
    }

    /// Bu tile indirilemedi mi? (canvas iskeletini seçmek için)
    pub fn is_failed(&self, coord: &TileCoord) -> bool {
        self.failed.contains(coord)
    }

    /// Görünür çevrede indirilemeyen tile var mı? — "çevrimdışı" rozeti için.
    ///
    /// `prune` uzaktaki kayıtları düşürdüğü için küme yalnızca ekrana yakın
    /// başarısızlıkları tutuyor: rozet ekranda gerçekten boşluk varken çıkar.
    pub fn has_failures(&self) -> bool {
        !self.failed.is_empty()
    }

    /// Görünür alandan uzaktaki tile'ları bellekten atar.
    fn prune(&mut self) {
        let min_x = (self.offset_x / TILE_SIZE).floor() as i64 - KEEP_RINGS;
        let min_y = (self.offset_y / TILE_SIZE).floor() as i64 - KEEP_RINGS;
        let max_x =
            ((self.offset_x + self.viewport.width as f64) / TILE_SIZE).ceil() as i64 + KEEP_RINGS;
        let max_y =
            ((self.offset_y + self.viewport.height as f64) / TILE_SIZE).ceil() as i64 + KEEP_RINGS;

        let zoom = self.zoom;
        let keep = |coord: &TileCoord| {
            coord.z == zoom
                && coord.x >= min_x
                && coord.x <= max_x
                && coord.y >= min_y
                && coord.y <= max_y
        };

        self.tiles.retain(|coord, _| keep(coord));
        self.pending.retain(keep);
        self.failed.retain(keep);
    }

    /// Tile'ın ekrandaki dikdörtgeni.
    pub fn tile_rect(&self, coord: TileCoord) -> Rectangle {
        let x = coord.x as f64 * TILE_SIZE - self.offset_x;
        let y = coord.y as f64 * TILE_SIZE - self.offset_y;
        Rectangle::new(
            Point::new(x as f32, y as f32),
            Size::new(TILE_SIZE as f32, TILE_SIZE as f32),
        )
    }

    /// Coğrafi koordinatın ekran konumu.
    pub fn screen_position(&self, lat: f64, lon: f64) -> Point {
        let (world_x, world_y) = lat_lon_to_world(lat, lon, self.zoom);
        Point::new(
            (world_x - self.offset_x) as f32,
            (world_y - self.offset_y) as f32,
        )
    }
}

/// Lat/lon → dünya pikseli (Web Mercator).
pub fn lat_lon_to_world(lat: f64, lon: f64, zoom: u32) -> (f64, f64) {
    let n = 2f64.powi(zoom as i32);
    let x = (lon + 180.0) / 360.0 * n;
    let lat_rad = lat.to_radians();
    let y = (1.0 - lat_rad.tan().asinh() / std::f64::consts::PI) / 2.0 * n;
    (x * TILE_SIZE, y * TILE_SIZE)
}

/// Tek bir tile'ı indirir ve çözer.
///
/// Hata durumunda `None` döner — harita eksik tile ile çalışmaya devam eder.
pub async fn fetch_tile(coord: TileCoord) -> (TileCoord, Option<iced_image::Handle>) {
    let client = CLIENT.get_or_init(reqwest::Client::new);
    let url = format!(
        "https://tile.openstreetmap.org/{}/{}/{}.png",
        coord.z, coord.x, coord.y
    );

    let response = match client
        .get(&url)
        .header("User-Agent", "Yoru-K Controller/1.0")
        .send()
        .await
    {
        Ok(response) => response,
        Err(error) => {
            eprintln!("Tile yükleme hatası {url}: {error}");
            return (coord, None);
        }
    };

    if !response.status().is_success() {
        eprintln!("Tile HTTP hatası {url}: {}", response.status());
        return (coord, None);
    }

    let bytes = match response.bytes().await {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("Tile okuma hatası {url}: {error}");
            return (coord, None);
        }
    };

    // PNG çözme CPU işi — bloklamayan çalıştırıcıya alınır.
    let decoded = tokio::task::spawn_blocking(move || {
        let decoded = image::load_from_memory(&bytes).ok()?;
        let rgba = decoded.into_rgba8();
        let (width, height) = rgba.dimensions();
        Some(iced_image::Handle::from_rgba(
            width,
            height,
            rgba.into_raw(),
        ))
    })
    .await
    .ok()
    .flatten();

    (coord, decoded)
}
