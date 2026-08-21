//! OpenStreetMap tile haritası — `crates/app/src/map.rs`'in iced portu.
//!
//! Slint sürümünde tile'lar `Flickable` içinde `Image` widget'larıydı ve
//! yükleme `slint::spawn_local` ile sürülüyordu. iced'de tile'lar tek bir
//! `Canvas` üzerine çizilir, indirme ise `Task::perform` ile yapılır.
//!
//! Koordinat modeli: `offset_x/offset_y`, görünür alanın sol-üst köşesinin
//! **dünya pikseli** cinsinden konumudur (dünya boyutu `256 * 2^zoom`).

use std::collections::{BTreeMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use tokio::sync::Semaphore;

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

/// Tek bir tile isteğinin üst sınırı.
///
/// reqwest'in varsayılan istek timeout'u YOK. Timeout'suz takılan bir bağlantı
/// tile'ı `pending` kümesinde sonsuza kadar tutuyordu; `take_missing_tiles`
/// bekleyenleri atladığı için o kare kalıcı olarak "yükleniyor" iskeletinde
/// kalıyor ve bir daha hiç denenmiyordu. Timeout dolduğunda `fetch_tile` `None`
/// dönüyor, `drop_pending` tile'ı başarısız işaretliyor ve sonraki görünüm
/// değişiminde yeniden isteniyor.
const TILE_TIMEOUT: Duration = Duration::from_secs(10);

/// Aynı anda uçabilecek en fazla tile indirmesi.
///
/// OpenStreetMap Tile Usage Policy en fazla iki indirme thread'i istiyor.
/// Sınır uygulanmadan önce `request_tiles` görünür alandaki tüm eksik
/// tile'ları tek `Task::batch` ile açıyordu — 1280x948 pencerede ilk boyamada
/// ~25 eşzamanlı GET, her zoom degisiminde bir o kadar daha. Sürekli bu tempo
/// IP bloğuna yol açar; bloklanan istemcide harita tamamen boş kalır ve arıza
/// istemci tarafından anlaşılamaz.
const MAX_CONCURRENT_TILE_FETCHES: usize = 2;

/// İndirme kuyruğunun kapısı.
///
/// `Task::batch` hâlâ tüm eksik tile'lar için birer future üretiyor; bunlar
/// `acquire()` üzerinde park ediyor ve ağ trafiği ikiyle sınırlı kalıyor.
static TILE_PERMITS: Semaphore = Semaphore::const_new(MAX_CONCURRENT_TILE_FETCHES);

/// Paylaşılan HTTP istemcisi — bağlantı havuzu tek noktada.
fn client() -> &'static reqwest::Client {
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(TILE_TIMEOUT)
            .build()
            .unwrap_or_else(|error| {
                // Kurulum hatası yutulmaz: timeout'suz istemciyle devam etmek,
                // sessizce haritasız kalmaktan iyi — ama hangisi olduğu yazılı.
                eprintln!(
                    "Tile HTTP istemcisi kurulamadı ({error}); timeout'suz istemciye düşülüyor"
                );
                reqwest::Client::new()
            })
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TileCoord {
    pub z: u32,
    pub x: i64,
    pub y: i64,
}

/// Bir indirme isteğinin ait olduğu görünüm kuşağı.
///
/// Kuyruk iki genişliğinde olduğu için sıradaki istekler artık bekliyor; zoom
/// değişimi eski seviyenin tile'larını geçersiz kılsa bile o istekler kuyruğun
/// başını tutmaya devam ederdi (`zoom_at` yalnızca `pending` kümesini temizler,
/// uçmakta olan future'a ulaşamaz). Kuşak sayacı, bayat bir isteğin kuyruğa hiç
/// girmeden düşmesini sağlıyor.
#[derive(Debug, Clone)]
pub struct TileEpoch {
    current: Arc<AtomicU64>,
    issued: u64,
}

impl TileEpoch {
    fn is_stale(&self) -> bool {
        self.current.load(Ordering::Relaxed) != self.issued
    }
}

/// Bir tile isteğinin sonucu.
///
/// `Option<Handle>` yetmiyordu: "indirilemedi" ile "artık bu görünüme ait
/// değil" aynı `None`'a düşüyor ve ikisi de `drop_pending`'e gidiyordu. Kuşak
/// sayacı monotonik ama `coord.z` DEĞİL — kullanıcı z15'ten z16'ya çıkıp geri
/// dönerse `coord.z == self.zoom` koruması "bu sonuç güncel" demiyor. O turda
/// bayat bir sonuç, canlı bir isteği `pending`'den düşürüp `failed`'e yazıyor:
/// sahte "Harita çevrimdışı" rozeti yanıyor ve tile bir sonraki kaydırmada
/// ikinci kez isteniyor — yani eşzamanlılık sınırının korumaya çalıştığı
/// sunucuya fazladan GET gidiyor.
#[derive(Debug, Clone)]
pub enum TileOutcome {
    Loaded(iced_image::Handle),
    /// İstek gerçekten başarısız oldu — iskelet çapraz işaretli çizilir.
    Failed,
    /// Kuşağı geçti; taze bir istek zaten uçuyor. Hiçbir şeye dokunulmamalı.
    Stale,
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
    /// Görünüm kuşağı — her zoom değişiminde artar (bkz. `TileEpoch`).
    epoch: Arc<AtomicU64>,
    /// Çizimi etkileyen her değişiklikte artan sayaç.
    ///
    /// `MapCanvas` geometrisini `canvas::Cache` üstünde tutuyor; önbelleğin ne
    /// zaman düşeceğini bu sayı söylüyor. Tek tek alanları (`zoom`, `offset_*`,
    /// `tiles`, `failed`, `viewport`) karşılaştırmak yerine sayaç kullanılıyor:
    /// `tiles` bir `BTreeMap`, uzunluğu değişmeden içeriği değişebilir ve
    /// karşılaştırma o durumu kaçırırdı.
    revision: u64,
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
            epoch: Arc::new(AtomicU64::new(0)),
            revision: 0,
        };
        state.center_on(DEFAULT_LAT, DEFAULT_LON);
        state
    }
}

impl MapState {
    /// Çizim durumu değişti — `canvas::Cache` bir sonraki karede düşsün.
    fn touch(&mut self) {
        self.revision = self.revision.wrapping_add(1);
    }

    /// Çizim durumunun sürüm numarası (bkz. `revision`).
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Verilen koordinatı görünür alanın ortasına alır.
    pub fn center_on(&mut self, lat: f64, lon: f64) {
        let (world_x, world_y) = lat_lon_to_world(lat, lon, self.zoom);
        self.offset_x = world_x - self.viewport.width as f64 / 2.0;
        self.offset_y = world_y - self.viewport.height as f64 / 2.0;
        self.touch();
    }

    /// Fare sürüklemesi — harita imleçle birlikte hareket eder.
    pub fn pan(&mut self, dx: f32, dy: f32) {
        self.offset_x -= dx as f64;
        self.offset_y -= dy as f64;
        self.clamp_to_world();
        self.touch();
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

        // Zoom seviyesi değişince eski tile'lar geçersiz. Kuşak sayacı da
        // artar: kuyrukta bekleyen eski istekler permit almadan düşsün.
        self.tiles.clear();
        self.pending.clear();
        self.failed.clear();
        self.epoch.fetch_add(1, Ordering::Relaxed);
        self.clamp_to_world();
        self.touch();
    }

    pub fn set_viewport(&mut self, size: Size) {
        self.viewport = size;
        self.clamp_to_world();
        self.touch();
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
        // `prune` görünürden uzaklaşan tile'ları atmış olabilir.
        self.touch();

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

    /// Şu anki görünüm kuşağı — `fetch_tile`'a verilir.
    pub fn epoch(&self) -> TileEpoch {
        TileEpoch {
            current: Arc::clone(&self.epoch),
            issued: self.epoch.load(Ordering::Relaxed),
        }
    }

    /// İnen tile'ı yerleştirir.
    pub fn insert_tile(&mut self, coord: TileCoord, handle: iced_image::Handle) {
        self.pending.remove(&coord);
        self.failed.remove(&coord);
        if coord.z == self.zoom {
            self.tiles.insert(coord, handle);
            self.touch();
        }
    }

    /// İndirilemeyen tile'ı bekleyenlerden düşürür — sonraki denemeye açık kalır.
    pub fn drop_pending(&mut self, coord: TileCoord) {
        self.pending.remove(&coord);
        // `insert_tile` ile aynı koruma: başka bir zoom seviyesine ait bir
        // sonuç bu seviyenin çizimini etkilemez. Kuşağı geçmiş bir istek
        // indirmeyi hiç denemeden `None` döndüğü için, koruma olmadan
        // `has_failures()` bir tur "çevrimdışı" rozeti yakıyordu.
        if coord.z == self.zoom {
            self.failed.insert(coord);
            self.touch();
        }
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
pub async fn fetch_tile(coord: TileCoord, epoch: TileEpoch) -> (TileCoord, TileOutcome) {
    // Kuyruğa girmeden önce: zoom bu future daha ilk kez uyanmadan değişmiş
    // olabilir. Sırada beklemesinin anlamı yok, üstelik önündeki canlı
    // indirmeleri geciktirir.
    if epoch.is_stale() {
        return (coord, TileOutcome::Stale);
    }

    // Kuyruğa gir. `permit` düşene kadar en fazla iki indirme uçar.
    let permit = match TILE_PERMITS.acquire().await {
        Ok(permit) => permit,
        // Semaphore yalnızca kapatılınca hata verir; bu uygulamada kapatan yok.
        Err(error) => {
            eprintln!("Tile kuyruğu kapandı: {error}");
            return (coord, TileOutcome::Failed);
        }
    };

    // Sıra beklerken de değişmiş olabilir.
    if epoch.is_stale() {
        return (coord, TileOutcome::Stale);
    }

    let client = client();
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
            return (coord, TileOutcome::Failed);
        }
    };

    if !response.status().is_success() {
        eprintln!("Tile HTTP hatası {url}: {}", response.status());
        return (coord, TileOutcome::Failed);
    }

    let bytes = match response.bytes().await {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("Tile okuma hatası {url}: {error}");
            return (coord, TileOutcome::Failed);
        }
    };

    // Ağ işi bitti; PNG çözme sırayı tutmasın — sıradaki tile hemen başlasın.
    drop(permit);

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

    match decoded {
        Some(handle) => (coord, TileOutcome::Loaded(handle)),
        // Çözme başarısız: gerçek bir hata, kuşak sorunu değil.
        None => (coord, TileOutcome::Failed),
    }
}
