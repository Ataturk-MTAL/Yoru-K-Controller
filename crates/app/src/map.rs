//! OpenStreetMap tile fetcher — Slint maps örneğinden adapte edilmiştir.
//!
//! `MapWorld` struct'ı tile cache'i yönetir, async olarak tile indirir,
//! ve Slint UI'a `MapTile` model olarak sunar.

use std::collections::BTreeMap;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use slint::{Global, Rgba8Pixel, SharedPixelBuffer, VecModel};

use crate::{AppState, AppWindow, MapTile};

const TILE_SIZE: f64 = 256.0;

/// OSM tile koordinatı
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
struct TileCoord {
    z: u32,
    x: isize,
    y: isize,
}

/// Tile dünyası — yüklenmiş ve yüklenmekte olan tile'ları yönetir
pub struct MapWorld {
    client: reqwest::Client,
    loaded_tiles: BTreeMap<TileCoord, slint::Image>,
    loading_tiles: BTreeMap<TileCoord, Pin<Box<dyn Future<Output = slint::Image>>>>,
    pub zoom_level: u32,
    pub visible_width: f64,
    pub visible_height: f64,
    pub offset_x: f64,
    pub offset_y: f64,
}

impl MapWorld {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            loaded_tiles: BTreeMap::new(),
            loading_tiles: BTreeMap::new(),
            zoom_level: 15,
            visible_width: 800.0,
            visible_height: 600.0,
            offset_x: 0.0,
            offset_y: 0.0,
        }
    }

    /// Merkez lat/lon'dan offset hesapla (ilk yükleme için)
    pub fn center_on(&mut self, lat: f64, lon: f64) {
        let (tx, ty) = lat_lon_to_tile(lat, lon, self.zoom_level);
        self.offset_x = tx * TILE_SIZE - self.visible_width / 2.0;
        self.offset_y = ty * TILE_SIZE - self.visible_height / 2.0;
    }

    /// Zoom seviyesini değiştir — mouse pozisyonuna göre pan
    pub fn set_zoom_level(&mut self, zoom: u32, ox: f64, oy: f64) {
        if self.zoom_level == zoom { return; }
        self.loaded_tiles.clear();
        self.loading_tiles.clear();
        let exp2 = f64::exp2(zoom as f64 - self.zoom_level as f64);
        self.offset_x += ox;
        self.offset_y += oy;
        self.offset_x *= exp2;
        self.offset_y *= exp2;
        self.offset_x -= ox;
        self.offset_y -= oy;
        self.zoom_level = zoom;
        self.request_visible_tiles();
    }

    /// Görünür tile'ları hesapla ve eksikleri indir
    pub fn request_visible_tiles(&mut self) {
        let m = 1isize << self.zoom_level;
        let min_x = (self.offset_x / TILE_SIZE).floor() as isize;
        let min_y = (self.offset_y / TILE_SIZE).floor() as isize;
        let max_x = (((self.offset_x + self.visible_width) / TILE_SIZE).ceil() as isize + 1).min(m);
        let max_y = (((self.offset_y + self.visible_height) / TILE_SIZE).ceil() as isize + 1).min(m);

        // Çok uzaktaki tile'ları sil
        const KEEP: isize = 10;
        let keep = |c: &TileCoord| {
            c.z == self.zoom_level
                && c.x > min_x - KEEP && c.x < max_x + KEEP
                && c.y > min_y - KEEP && c.y < max_y + KEEP
        };
        self.loading_tiles.retain(|c, _| keep(c));
        self.loaded_tiles.retain(|c, _| keep(c));

        for x in min_x..max_x {
            for y in min_y..max_y {
                // Negatif veya sınır dışı tile'ları atla
                if x < 0 || y < 0 || x >= m || y >= m { continue; }

                let coord = TileCoord { z: self.zoom_level, x, y };
                if self.loaded_tiles.contains_key(&coord) { continue; }

                self.loading_tiles.entry(coord).or_insert_with(|| {
                    let url = format!(
                        "https://tile.openstreetmap.org/{}/{}/{}.png",
                        coord.z, coord.x, coord.y
                    );
                    let client = self.client.clone();
                    Box::pin(async move {
                        let resp = match client
                            .get(&url)
                            .header("User-Agent", "Yoru-K Controller/1.0")
                            .send()
                            .await
                        {
                            Ok(r) => r,
                            Err(e) => { eprintln!("Tile yükleme hatası {url}: {e}"); return slint::Image::default(); }
                        };
                        if !resp.status().is_success() {
                            eprintln!("Tile HTTP hatası {url}: {}", resp.status());
                            return slint::Image::default();
                        }
                        let bytes = match resp.bytes().await {
                            Ok(b) => b,
                            Err(e) => { eprintln!("Tile okuma hatası {url}: {e}"); return slint::Image::default(); }
                        };
                        // Image decode — blocking thread'de
                        let buffer = tokio::task::spawn_blocking(move || {
                            let img = match image::load_from_memory(&bytes) {
                                Ok(i) => i,
                                Err(e) => { eprintln!("Tile decode hatası: {e}"); return None; }
                            };
                            let rgba = img
                                .resize_exact(
                                    TILE_SIZE as u32,
                                    TILE_SIZE as u32,
                                    image::imageops::FilterType::Nearest,
                                )
                                .into_rgba8();
                            let buf = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(
                                rgba.as_raw(), rgba.width(), rgba.height(),
                            );
                            Some(buf)
                        })
                        .await
                        .unwrap();
                        buffer.map(|b| slint::Image::from_rgba8(b)).unwrap_or_default()
                    })
                });
            }
        }
    }

    /// Yüklenmekte olan tile'ları poll et
    pub fn poll(&mut self, cx: &mut Context, changed: &mut bool) {
        self.loading_tiles.retain(|coord, future| {
            match future.as_mut().poll(cx) {
                Poll::Ready(image) => {
                    self.loaded_tiles.insert(*coord, image);
                    *changed = true;
                    false
                }
                Poll::Pending => true,
            }
        });
    }

    /// Yüklenmiş tile'ları Slint model'e dönüştür
    pub fn to_tile_model(&self) -> slint::ModelRc<MapTile> {
        let tiles: Vec<MapTile> = self.loaded_tiles.iter().map(|(coord, image)| {
            MapTile {
                x: (coord.x as f32 * TILE_SIZE as f32),
                y: (coord.y as f32 * TILE_SIZE as f32),
                tile: image.clone(),
            }
        }).collect();
        slint::ModelRc::new(VecModel::from(tiles))
    }

    /// GPS lat/lon → tile dünya piksel koordinatı
    pub fn lat_lon_to_pixel(&self, lat: f64, lon: f64) -> (f64, f64) {
        let (tx, ty) = lat_lon_to_tile(lat, lon, self.zoom_level);
        (tx * TILE_SIZE, ty * TILE_SIZE)
    }

    /// Harita dünya boyutu (piksel)
    pub fn world_size(&self) -> f64 {
        TILE_SIZE * (1isize << self.zoom_level) as f64
    }
}

/// Lat/lon → tile koordinatı (kesirli)
pub fn lat_lon_to_tile(lat: f64, lon: f64, zoom: u32) -> (f64, f64) {
    let n = 2.0_f64.powi(zoom as i32);
    let x = (lon + 180.0) / 360.0 * n;
    let lat_rad = lat.to_radians();
    let y = (1.0 - lat_rad.tan().asinh() / std::f64::consts::PI) / 2.0 * n;
    (x, y)
}

/// Harita state'ini tutan ve Slint UI ile senkronize eden yapı
pub struct MapState {
    pub world: std::cell::RefCell<MapWorld>,
    pub ui_weak: slint::Weak<AppWindow>,
    pub poll_handle: std::cell::RefCell<Option<slint::JoinHandle<()>>>,
}

impl MapState {
    /// Tile model'i güncelle ve async poll başlat
    pub fn do_poll(self: std::rc::Rc<Self>) {
        if let Some(handle) = self.poll_handle.take() {
            handle.abort();
        }
        self.refresh_ui();
        let s = self.clone();
        let handle = slint::spawn_local(async move {
            std::future::poll_fn(|cx| {
                let mut changed = false;
                s.world.borrow_mut().poll(cx, &mut changed);
                if changed {
                    s.refresh_ui();
                }
                if s.world.borrow().loading_tiles.is_empty() {
                    Poll::Ready(())
                } else {
                    Poll::Pending
                }
            })
            .await;
        })
        .unwrap();
        *self.poll_handle.borrow_mut() = Some(handle);
    }

    /// Slint UI'ı güncelle: tile model + viewport + marker
    fn refresh_ui(&self) {
        let Some(ui) = self.ui_weak.upgrade() else { return };
        let world = self.world.borrow();
        let state = AppState::get(&ui);

        // Tile model
        state.set_map_tiles(world.to_tile_model());

        // Zoom
        state.set_map_zoom(world.zoom_level as i32);

        // GPS marker pozisyonu
        let lat = state.get_gps_lat() as f64;
        let lon = state.get_gps_lon() as f64;
        if lat != 0.0 || lon != 0.0 {
            let (px, py) = world.lat_lon_to_pixel(lat, lon);
            state.set_map_marker_x(px as f32);
            state.set_map_marker_y(py as f32);
        }
    }

    /// Viewport pozisyonunu set et (public function aracılığıyla)
    pub fn set_viewport(&self) {
        let Some(ui) = self.ui_weak.upgrade() else { return };
        let world = self.world.borrow();
        let ws = world.world_size() as f32;
        ui.invoke_set_map_viewport(
            -world.offset_x as f32,
            -world.offset_y as f32,
            ws,
            ws,
        );
    }
}
