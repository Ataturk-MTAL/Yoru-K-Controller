# Yörü-K Kontrolcü

Diferansiyel sürüşlü bir mobil robotu **seri port** veya **TCP/IP** üzerinden kontrol eden masaüstü uygulaması.
Rust + Slint ile yazılmıştır; OpenCV bağımlılığı yoktur.

---

## Özellikler

| Kategori | Özellik |
|---|---|
| **Kontrol** | Farekli joystick, WASD / ok tuşu klavye kontrolü |
| **Bağlantı** | Seri port (UART) ve TCP/IP (ESP32 AP modu) |
| **Protokol** | Binary paket formatı, Fletcher-16 checksum |
| **Motor** | Diferansiyel sürüş, 3 vites, motor ters bağlantı desteği |
| **Periferal** | Işık ve fren toggle |
| **Kamera** | Gerçek zamanlı kamera görüntüsü (nokhwa, AVFoundation) |
| **AI Tespit** | YOLOv8 ONNX nesne tespiti, NMS, bbox çizimi |
| **Faz 2** | GPS OSM harita entegrasyonu *(planlandı)* |

---

## Ekran Görünümü

```
┌─────────────────────────────────────────────────────────────────┐
│  🤖 YÖRÜ-K          WASD · Space=DUR · L=Işık · B=Fren · 1/2/3 │
├──────────────────────────────┬──────────────────────────────────┤
│                              │  ┌─ BAĞLANTI ──────────────────┐ │
│                              │  │  [ Serial ]  [ TCP/IP ]     │ │
│       KAMERA GÖRÜNTÜSÜ       │  │  Port: /dev/tty...  [↻]     │ │
│     (YOLO tespit kutuları)   │  │  [        BAĞLAN        ]   │ │
│                              │  └─────────────────────────────┘ │
│  ┌──────────────────────┐    │  ┌─ MOTOR KONTROL ─────────────┐ │
│  │  person  87%         │    │  │  [ ▶ BAŞLAT ] [ ■ DURDUR ] │ │
│  └──────────────────────┘    │  │  VİTES:  [V1] [V2] [V3]    │ │
│                              │  │  [💡 IŞIK]    [🛑 FREN]     │ │
│  ● Robot bağlı değil         │  │  [↺ Sol Ters] [↻ Sağ Ters] │ │
│  ■ Durdur  ▶ Başlat          │  └─────────────────────────────┘ │
│                              │  ┌─ JOYSTICK ──────────────────┐ │
│                              │  │        ╭─────╮              │ │
│                              │  │       ╭┤  ●  ├╮             │ │
│                              │  │        ╰─────╯              │ │
│                              │  └─────────────────────────────┘ │
│                              │  ┌─ HIZ GÖSTERGESİ ────────────┐ │
│                              │  │  Sol: ████████░░  +72       │ │
│                              │  │  Sağ: ████████░░  +72       │ │
│                              │  │  Vites: V2     ● Çalışıyor  │ │
│                              │  └─────────────────────────────┘ │
├──────────────────────────────┴──────────────────────────────────┤
│  ● Bağlı — /dev/tty.usbserial-0001               v0.1.0        │
└─────────────────────────────────────────────────────────────────┘
```

---

## Gereksinimler

### Derleme Araçları

- **Rust** 1.75+ ([rustup.rs](https://rustup.rs))
- **macOS** ARM64 / x86-64 (Linux ve Windows de desteklenir)

### Rust Bağımlılıkları (otomatik indirilir)

| Crate | Versiyon | Amaç |
|---|---|---|
| `slint` | 1.15.1 | UI framework |
| `ort` | 2.0.0-rc.11 | ONNX Runtime (YOLO inference) |
| `nokhwa` | 0.10 | Kamera yakalama (AVFoundation) |
| `serialport` | 4.6 | Seri port iletişimi |
| `tokio` | 1.41 | Async TCP runtime |
| `imageproc` | 0.26 | OpenCV'siz bbox çizimi |
| `image` | 0.25 | Görüntü işleme |
| `ndarray` | 0.17 | ONNX tensor verisi |

> **Not:** `ort` crate'i `download-binaries` özelliğiyle ONNX Runtime'ı ilk derlemede otomatik indirir, harici kurulum gerekmez.

---

## Kurulum ve Derleme

### 1. Projeyi klonla

```bash
git clone <repo-url> Yoru-K-Controller-Rust
cd Yoru-K-Controller-Rust
```

### 2. YOLO modelini hazırla

```bash
# Python ortamında (bir kez):
pip install ultralytics
yolo export model=yolov8n.pt format=onnx opset=12

# Modeli assets/ klasörüne taşı:
mkdir -p assets
cp yolov8n.onnx assets/
```

### 3. Derle ve çalıştır

```bash
cargo build --release
cargo run --release
```

Geliştirme modunda:

```bash
cargo run
```

---

## Proje Yapısı

```
Yoru-K-Controller-Rust/
├── Cargo.toml                  ← Workspace manifest
├── Cargo.lock
├── assets/
│   └── yolov8n.onnx            ← YOLO modeli (git'e dahil değil)
│
└── crates/
    ├── protocol/               ← Saf Rust, bağımlılık yok
    │   └── src/
    │       ├── packet.rs       ← Paket oluşturma, Fletcher-16
    │       └── types.rs        ← Command / RobotResponse enum'ları
    │
    ├── transport/              ← Serial + TCP worker thread'leri
    │   └── src/
    │       ├── framing.rs      ← try_parse_packet() — stream çerçeveleme
    │       ├── serial_worker.rs
    │       ├── tcp_worker.rs
    │       └── connection.rs   ← ConnectionManager (birleşik arayüz)
    │
    ├── control/                ← Joystick + klavye algoritmaları
    │   └── src/
    │       ├── joystick.rs     ← Diferansiyel sürüş hesabı
    │       └── keyboard.rs     ← WASD durum makinesi
    │
    ├── vision/                 ← Kamera + YOLO (ağır bağımlılıklar izole)
    │   └── src/
    │       ├── camera.rs       ← nokhwa frame yakalama
    │       ├── detection.rs    ← ort ONNX inference, NMS
    │       └── drawing.rs      ← imageproc ile bbox çizimi
    │
    └── app/                    ← Binary crate, her şeyi bağlar
        ├── build.rs            ← slint_build::compile()
        ├── src/
        │   ├── main.rs         ← Giriş, Slint event loop, callback bağlantıları
        │   └── bridge.rs       ← Slint ↔ Rust thread köprüsü
        └── ui/
            ├── app.slint       ← Ana pencere, klavye handler'ları
            ├── theme.slint     ← Renkler, fontlar, spacing sabitleri
            ├── state.slint     ← Global AppState singleton
            └── components/
                ├── connection_panel.slint
                ├── motor_control.slint
                ├── joystick_panel.slint
                ├── speed_display.slint
                └── camera_view.slint
```

---

## İletişim Protokolü

### Paket Formatı

```
[ 0xAA ][ CMD ][ LEN ][ DATA... ][ CK2 ][ CK1 ]
```

| Alan | Boyut | Açıklama |
|---|---|---|
| `START_BYTE` | 1 byte | Sabit `0xAA` |
| `CMD` | 1 byte | Komut kodu |
| `LEN` | 1 byte | Veri alanı uzunluğu (byte) |
| `DATA` | LEN byte | Komuta özgü veri |
| `CK2, CK1` | 2 byte | Fletcher-16 checksum (`pkt[1..]` üzerinden) |

### Komut Kodları

| Kod | Sabit | Açıklama |
|---|---|---|
| `0x00` | `SetStop` | Motor durdur |
| `0x01` | `SetSpeed` | Hız paketi gönder |
| `0x02` | `SetLight` | Işık aç/kapat |
| `0x03` | `SetBrake` | Fren aç/kapat |
| `0xFF` | `SetStart` | Motor başlat |
| `0x10` | `GetStatus` | Motor durumu sorgula |
| `0x11` | `GetSpeed` | Anlık hız sorgula |
| `0x14` | `GetGps` | GPS koordinatı sorgula *(Faz 2)* |

### Hız Paketi Detayı

```
AA 01 07 [vites] 4C [sol_yön] [sol_abs] 52 [sağ_yön] [sağ_abs] [CK2] [CK1]
```

- `vites`: `1`, `2` veya `3`
- `4C` = `'L'` (sol motor işaretçisi)
- `52` = `'R'` (sağ motor işaretçisi)
- `yön`: `0x46` = `'F'` (ileri) / `0x42` = `'B'` (geri)
- `abs`: mutlak hız değeri `0–100`

### Fletcher-16 Checksum

```rust
// pkt[1..] üzerinden — START_BYTE hariç
let mut sum1: u32 = 0;
let mut sum2: u32 = 0;
for &b in data {
    sum1 = (sum1 + b as u32) % 255;
    sum2 = (sum2 + sum1) % 255;
}
[sum2 as u8, sum1 as u8]  // paketin sonuna [CK2, CK1] olarak eklenir
```

---

## Kontrol Algoritmaları

### Joystick — Diferansiyel Sürüş

```
norm_x = dx / max_radius  (−1.0 … 1.0)
norm_y = dy / max_radius  (−1.0 … 1.0)

base  = norm_y × 100
turn  = norm_x × 50

sol_motor  = clamp(base − turn, −100, 100)
sağ_motor  = clamp(base + turn, −100, 100)
```

- Ölü bölge: `|dx| < 15px && |dy| < 15px` → hız = 0
- Joystick halka sınırına kenetlenir (daire dışına çıkmaz)

### Klavye (WASD / Ok Tuşları)

| Tuş | Sol Motor | Sağ Motor |
|---|---|---|
| W / ↑ | +100 | +100 |
| S / ↓ | −100 | −100 |
| A / ← | −50 | +50 |
| D / → | +50 | −50 |
| W+D | +50 | +100 |
| W+A | +100 | +50 |

### Motor Ters Bağlantı (Sırt Sırta Montaj)

Motorlar karşılıklı monte edildiğinde sol veya sağ motor yönü yazılımdan tersine çevrilebilir.
`↺ Sol Ters` / `↻ Sağ Ters` butonlarından biri aktifken diğeri devre dışı kalır (mutex).

---

## Thread Mimarisi

```
Ana Thread (Slint Event Loop)
    │
    ├── serial_thread ────────────────────────────────────────┐
    │   serialport blocking I/O                               │
    │   mpsc::Sender<RobotEvent> ──────────────────────────►  │
    │                                                         │
    ├── tokio runtime (TCP) ───────────────────────────────── │
    │   tokio::net::TcpStream async                          │
    │   mpsc::Sender<RobotEvent> ──────────────────────────► │
    │                                                         ▼
    ├── camera_thread ──────────────────────────────► bridge_thread
    │   nokhwa AVFoundation                          │  Kanalları poll eder
    │   SyncSender<RgbaFrame> (capacity=1)          │  slint::invoke_from_event_loop()
    │   → frame drop: meşgulken yeni frame düşer   │  ~120 Hz (8ms uyku)
    │                                                │
    └── periodic_send_thread                         │
        50ms döngü, 20Hz hız gönderimi              │
        Hysteresis filtresi: |Δhız| ≤ 2 → atla     │
```

**Frame drop mekanizması:** `SyncSender::try_send` kapasitesi 1'dir. Kamera thread'i yeni frame üretirken önceki henüz işlenmediyse yeni frame sessizce düşer — Python'daki `m_busy` AtomicInt'in Rust karşılığı.

---

## Klavye Kısayolları

| Kısayol | İşlev |
|---|---|
| `W` / `↑` | İleri |
| `S` / `↓` | Geri |
| `A` / `←` | Sol dönüş |
| `D` / `→` | Sağ dönüş |
| `Space` | **Acil durdurma** |
| `1` / `2` / `3` | Vites seç |
| `L` | Işık aç/kapat |
| `B` | Fren aç/kapat |

---

## TCP/IP Bağlantısı (ESP32 AP Modu)

ESP32, `192.168.4.1:80` adresinde erişilebilir bir AP açar.
Bağlantı panelinde TCP/IP modunu seçip bu adresi girdikten sonra Bağlan butonuna basın.

---

## YOLO Nesne Tespiti

### Model Hazırlama (bir kez)

```bash
pip install ultralytics
yolo export model=yolov8n.pt format=onnx opset=12
cp yolov8n.onnx assets/
```

> `opset=12` geniş uyumluluk sağlar.

### Pipeline

```
RGBA frame
    │
    ▼ RGBA → RGB dönüşümü
    │
    ▼ Letterbox resize (640×640, gri padding #727272)
    │
    ▼ HWC → CHW, normalize [0,255] → [0.0,1.0]
    │
    ▼ ort ONNX inference (shape: [1,3,640,640])
    │
    ▼ Çıktı [1,84,8400]: xywh + 80 class score
    │
    ▼ Confidence filtrele (varsayılan: 0.50)
    │
    ▼ Koordinatları orijinal boyuta geri ölçekle (letterbox ters dönüşüm)
    │
    ▼ NMS (IoU threshold: 0.45)
    │
    ▼ imageproc ile bbox çizimi (OpenCV yok)
```

**80 COCO sınıfı** desteklenir: person, car, bicycle, dog, cat…

---

## Faz 2: GPS Harita (Planlandı)

Slint `maps` örneği referans alınarak OSM tile tabanlı harita entegrasyonu:

- **Tile sistemi:** `BTreeMap<TileCoord, slint::Image>` — yüklü / yükleniyor ayrımı
- **Async tile fetch:** `tokio::spawn` + `reqwest::Client`
  - URL: `https://tile.openstreetmap.org/{z}/{x}/{y}.png`
- **Görüntü decode:** `tokio::task::spawn_blocking` içinde `image::load_from_memory()`
- **Slint render:** `Flickable` + `Image` grid, `VecModel<Tile>` binding
- **Robot marker:** GPS `lat/lon` → tile piksel koordinatına çevrilip `Image` üstünde konumlandırılır
- **Veri kaynağı:** Protokol `0x14 GetGps` yanıtı → `RobotResponse::Gps { lat, lon }` (Faz 1'de altyapı hazır)

---

## Geliştirici Notları

### Yeni Slint State Ekleme

State'ler `crates/app/ui/state.slint` dosyasındaki `AppState` global'ine eklenir.
Rust tarafında `AppState::get(&ui).set_xxx(value)` ile güncellenir (her zaman Slint event loop'undan çağrılmalı).

### Yeni Paket Türü Ekleme

1. `crates/protocol/src/types.rs` → `Command` enum'una kod ekle
2. `crates/protocol/src/packet.rs` → paket fonksiyonu yaz + birim test ekle
3. `crates/transport/src/framing.rs` → `parse_response()` match kolunu ekle
4. `crates/app/src/bridge.rs` → `RobotEvent::Packet` match'ine UI güncellemesi ekle

### Birim Testleri

```bash
cargo test -p protocol   # Paket yapısı, Fletcher-16, reverse flag
cargo test -p control    # Joystick ölü bölge, klavye kombinasyonları
cargo test -p transport  # Framing parser
cargo test               # Tüm workspace
```

---

## Lisans

MIT

---

## Referanslar

- [Alfred Weirich — Rust + ORT + ONNX + YOLO (3 Bölüm)](https://medium.com/@alfred.weirich/rust-ort-onnx-real-time-yolo-on-a-live-webcam-part-1-b6edfb50bf9b)
- [Slint UI Framework](https://slint.dev)
- [Slint Maps Örneği — OSM tile + async](https://github.com/slint-ui/slint/blob/master/examples/maps/main.rs)
- [han-minhee/yolo-rust-ort](https://github.com/han-minhee/yolo-rust-ort)
- [nokhwa — Pure Rust kamera kütüphanesi](https://github.com/l1npengtul/nokhwa)
