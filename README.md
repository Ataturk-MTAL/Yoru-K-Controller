# Yörü-K Kontrolcü

Diferansiyel sürüşlü bir mobil robotu **seri port** veya **TCP/IP** üzerinden kontrol eden masaüstü uygulaması.
Rust + [iced](https://iced.rs) ile yazılmıştır; OpenCV bağımlılığı yoktur.

---

## Özellikler

| Kategori | Özellik |
|---|---|
| **Kontrol** | Fareyle joystick, WASD / ok tuşu klavye kontrolü |
| **Bağlantı** | Seri port (UART) ve TCP/IP (ESP32 AP modu) |
| **Protokol** | Binary paket formatı, Fletcher-16 checksum |
| **Motor** | Diferansiyel sürüş, 3 vites, motor ters bağlantı desteği |
| **Çevre birimi** | Işık ve fren toggle |
| **Kamera** | Gerçek zamanlı kamera görüntüsü (nokhwa, AVFoundation) |
| **AI Tespit** | YOLO26 ONNX nesne tespiti, bbox çizimi |
| **Harita** | OpenStreetMap tile haritası + GPS robot işaretçisi |
| **Tema** | Koyu / açık tema, Material 3 rol modeli |

---

## Ekran Görünümü

```
┌─────────────────────────────────────────────────────────────────┐
│  YÖRÜ-K      [📷 Kamera] [🛰 Harita]   Kısayollar Hakkında  🌙  │
├──────────────────────────────┬──────────────────────────────────┤
│  ● Yörü-K bağlı değil        │  BAĞLANTI                        │
│                              │  [ Serial ]  [ TCP/IP ]          │
│                              │  Port: /dev/tty...        [↻]    │
│       KAMERA GÖRÜNTÜSÜ       │  [         BAĞLAN          ]     │
│     (YOLO tespit kutuları)   │  ● Bağlantı yok                  │
│                              │  ─────────────────────────────   │
│                   [10 FPS]   │  MOTOR KONTROL                   │
│                   [2 nesne]  │  [     ▶ Motor BAŞLAT      ]     │
│                              │  Vites:         [−] V1 [+]       │
│                              │  Gönd. Aralığı: [−] 30 ms [+]    │
│                              │  [💡 IŞIK]     [🛑 FREN]         │
│                              │  [↺ Sol Ters]  [↻ Sağ Ters]      │
│  ┌──────────────────────────┐│  ─────────────────────────────   │
│  │0: Kamera ▾ [↻] [▶ Başlat]││  SÜRÜŞ KUMANDA KOLU              │
│  └──────────────────────────┘│        ╭───────────╮             │
│                              │        │     ●     │             │
│                              │        ╰───────────╯             │
│                              │  HIZ GÖSTERGESİ                  │
│                              │  Sol ▬▬▬▬│▬▬▬▬  +0               │
│                              │  Sağ ▬▬▬▬│▬▬▬▬  +0               │
├──────────────────────────────┴──────────────────────────────────┤
│  ● Serial — Bağlı  │ ● Motor Durdu      Bağlı — /dev/tty...     │
└─────────────────────────────────────────────────────────────────┘
```

---

## Gereksinimler

- **Rust** 1.88+ ([rustup.rs](https://rustup.rs)) — iced 0.14'ün `rust-version` alt sınırı
- **macOS** ARM64 / x86-64 (Linux ve Windows de desteklenir)

Bağımlılıklar `cargo build` sırasında otomatik indirilir:

| Crate | Versiyon | Amaç |
|---|---|---|
| `iced` | 0.14 | UI framework (wgpu renderer) |
| `usls` | 0.2.0-alpha.3 | YOLO26 ONNX çalıştırma (ORT sarmalayıcı) |
| `nokhwa` | 0.10 | Kamera yakalama |
| `serialport` | 4.6 | Seri port iletişimi |
| `tokio` | 1.41 | Async TCP runtime + tile indirme |
| `reqwest` | 0.13 | OSM tile HTTP istemcisi |
| `image` / `imageproc` | 0.25 / 0.26 | Görüntü işleme, bbox çizimi |

> **Not:** ONNX Runtime, `usls`'in `ort-download-binaries` özelliğiyle ilk derlemede otomatik iner; harici kurulum gerekmez. Çalıştırma sağlayıcısı platforma göre seçilir: macOS → CoreML, Windows → DirectML (başarısızsa CPU), Linux → CPU.

---

## Kurulum ve Çalıştırma

```bash
git clone https://github.com/Ataturk-MTAL/Yoru-K-Controller.git
cd Yoru-K-Controller
cargo run --release
```

YOLO modeli ayrıca hazırlanmaz: uygulama çalışma dizininde `v26-n-det.onnx` varsa onu kullanır, yoksa modeli `usls` hub'ından indirir (ilk çalıştırmada ~6 MB).

---

## Proje Yapısı

```
Yoru-K-Controller/
├── Cargo.toml                  ← Workspace manifest
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
    │       ├── detection.rs    ← usls YOLO26 çıkarımı
    │       └── drawing.rs      ← imageproc ile bbox çizimi
    │
    └── app/                    ← Binary crate, arayüz
        ├── build.rs            ← Windows .exe ikonu
        ├── assets/             ← Fontlar (Saira, Space Mono), ikon, logo
        └── src/
            ├── main.rs         ← iced::application, abonelikler, kısayollar
            ├── state.rs        ← Uygulama modeli (tek doğruluk kaynağı)
            ├── message.rs      ← Mesaj enum'u
            ├── update.rs       ← Durum makinesi
            ├── backend.rs      ← Transport köprüsü + periyodik gönderim
            ├── camera.rs       ← Kamera/tespit hattı
            ├── map.rs          ← OSM tile matematiği ve indirme
            ├── theme.rs        ← Renk rolleri, ölçü token'ları
            ├── styles.rs       ← Widget stilleri
            └── view/           ← Görünüm katmanı (panel başına bir dosya)
```

---

## Arayüz Mimarisi

iced **MVU** (Model-View-Update) üzerine kurulu:

```
App (state.rs) ──► view() ──► widget ağacı ──► etkileşim ──► Message
  ▲                                                            │
  └──────────────── update() ◄─────────────────────────────────┘
                       │
                   Task<Message> ──► async iş ──► Message
```

- `view` saf: yalnızca `App` alanlarını okur, hiçbir atomik ya da mutex görmez.
- `update` bloklamaz: paketler `Backend` üzerinden kuyruğa bırakılır; port/kamera listeleme ve tile indirme `Task::perform` ile arka plana gider.
- Donanım olayları (robot paketleri, kamera kareleri) `Subscription` üzerinden mesaja dönüşür.

### Tema

Renkler Material 3 rol modeline göre tanımlı (`theme.rs`):

- Her dolgu rolünün bir `on_*` eşi var — renkli zemin üzerindeki metin rengi tahmine bırakılmaz.
- `*_container` aileleri yumuşak tint yüzeyler için (rozet, uyarı şeridi).
- Yüzey merdiveni ton tabanlı: `surface_dim` → `surface_container_low` → `surface_container` → `surface_container_high` → `surface_container_highest`. Gölge kullanılmaz.
- Hover/pressed ayrı renk değil; taban rengin üstüne `on_*` karışımı (durum katmanı).
- Buton, metin kutusu ve seçim kutusu tek standart yükseklikte (`CONTROL_HEIGHT`).

---

## İletişim Protokolü

Tam referans: [`PROTOCOL.md`](PROTOCOL.md)

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
| `0x04` | `SetGpsEnable` | GPS yayınını başlat/durdur |
| `0xFF` | `SetStart` | Motor başlat |
| `0x10` | `GetStatus` | Motor durumu sorgula |
| `0x11` | `GetSpeed` | Anlık hız sorgula |
| `0x14` | `GetGps` | GPS koordinatı (float32 LE ×2) |

### Hız Paketi

```
AA 01 07 [vites] 4C [sol_yön] [sol_abs] 52 [sağ_yön] [sağ_abs] [CK2] [CK1]
```

- `vites`: `1`, `2` veya `3`
- `4C` = `'L'` (sol motor işaretçisi), `52` = `'R'` (sağ motor işaretçisi)
- `yön`: `0x46` = `'F'` (ileri) / `0x42` = `'B'` (geri)
- `abs`: mutlak hız değeri `0–100`

---

## Kontrol Algoritmaları

### Joystick — Diferansiyel Sürüş

```
norm_x = (dx / max_radius) × 100   (−100 … 100)
norm_y = (dy / max_radius) × 100   (−100 … 100)

|norm_x| < 8  →  norm_x = 0        (eksen snap: düz ileri/geri)
|norm_y| < 8  →  norm_y = 0        (eksen snap: yerinde dönüş)

sol_motor = clamp(norm_y − norm_x, −100, 100)
sağ_motor = clamp(norm_y + norm_x, −100, 100)
```

- Ölü bölge: `|dx| < 15px && |dy| < 15px` → hız = 0
- Bilek, halkanın içinde kalacak biçimde kenetlenir

### Klavye (WASD / Ok Tuşları)

| Tuş | Sol Motor | Sağ Motor |
|---|---|---|
| W / ↑ | +100 | +100 |
| S / ↓ | −100 | −100 |
| A / ← | +100 | −100 |
| D / → | −100 | +100 |
| W + D | 0 | +100 |
| W + A | +100 | 0 |

> Klavye girdileri yalnızca motor çalışırken işlenir; odakta bir metin alanı varsa tuşlar sürüşe gitmez.

### Motor Ters Bağlantı

Motorlar sırt sırta monte edildiğinde sol veya sağ motorun yönü yazılımdan tersine çevrilebilir. İki bayrak karşılıklı dışlar: biri açılınca diğeri kapanır. Tersine çevirme, yön baytı ve mutlak değer hesabından **önce** uygulanır.

---

## Thread Mimarisi

```
iced event loop (ana thread)
    │
    ├── robot-event-pump ──► tokio kanalı ──► Subscription ──► Message::Robot
    │     serial-worker / tokio TCP task olaylarını toplar
    │
    ├── periodic-send
    │     motor çalışırken her `send_interval_ms`'de hız paketi gönderir
    │     (hız değişmese de gönderilir — ESP32 zaman aşımına düşmesin)
    │
    ├── camera-worker ──► SyncSender(cap=1) ──► frame-pump ──► tokio kanalı(cap=2)
    │     nokhwa yakalama                bbox çizimi burada    ──► Message::Camera
    │
    └── detection-worker
          300 ms'de bir çıkarım; kapalıyken uyur (CPU sıfır)
```

Kare düşürme iki noktada: kamera thread'i kanal doluyken decode'u atlar, pump ise UI kanalı doluyken kareyi düşürür. Görüntü böylece hep anlık kalır.

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
| `C` / `M` | Kamera / Harita sekmesi |
| `T` | Tema değiştir |
| `Esc` | Açık pencereyi kapat |

---

## TCP/IP Bağlantısı (ESP32 AP Modu)

ESP32, `192.168.4.1:80` adresinde erişilebilir bir AP açar. Bağlantı panelinde TCP/IP modunu seçip adresi girdikten sonra **BAĞLAN**'a basın. Nagle algoritması kapatılır; küçük paketler beklemeden gider.

---

## Kamera ve Nesne Tespiti

```
RGBA kare (nokhwa)
    │
    ▼ RGB'ye çevir
    ▼ usls YOLO26 çıkarımı (640×640, uçtan uca NMS)
    ▼ Güven eşiği 0.50
    ▼ imageproc ile bbox + etiket çizimi
    ▼ iced image::Handle → ekran
```

Tespit hattı ayrı bir thread'de, saniyede en fazla ~3 kez çalışır; kapalıyken uyur. Kutular her kareye çizildiği için görüntü akıcı kalırken etiketler düzenli güncellenir.

---

## GPS Harita

`▶ GPS Başlat` düğmesi `0x04 SetGpsEnable` paketiyle robotun periyodik konum yayınını açar. Gelen `0x14` yanıtları haritada işaretçiye dönüşür; ilk konum geldiğinde harita robotun üstüne ortalanır.

- Tile kaynağı: `https://tile.openstreetmap.org/{z}/{x}/{y}.png`
- Tile'lar tek bir `Canvas` üzerine çizilir; sürükleyerek kaydırma, tekerlek veya alt çubukla zoom (1–19)
- Görünür alanın dışındaki tile'lar bellekten düşürülür

---

## Geliştirici Notları

### Yeni bir kontrol eklemek

1. `message.rs` → `Message` enum'una bir varyant ekle (geçmiş zaman adlandırma).
2. `update.rs` → varyantı işle; I/O varsa `Task::perform` ile arka plana at.
3. `view/…` → ilgili panele widget'ı koy; buton/alan/seçicilerde `CONTROL_HEIGHT` kullan.

### Yeni bir paket türü eklemek

1. `crates/protocol/src/types.rs` → `Command` enum'una kod ekle
2. `crates/protocol/src/packet.rs` → paket fonksiyonu + birim test
3. `crates/protocol/src/packet.rs::parse_response` → yanıt kolu
4. `crates/app/src/update.rs` → `apply_robot_event` içinde UI güncellemesi

### Testler

```bash
cargo test -p protocol   # Paket yapısı, Fletcher-16, reverse flag
cargo test -p control    # Joystick ölü bölge, klavye kombinasyonları
cargo test -p transport  # Framing parser
cargo test --workspace --exclude app
```

---

## Slint Sürümü

Arayüzün ilk hâli Slint ile yazılmıştı. O sürümün tamamı **`legacy/slint-ui`** branch'inde duruyor:

```bash
git checkout legacy/slint-ui
```

`main` üzerinde yalnızca iced sürümü bulunur; `protocol`, `transport`, `control` ve `vision` crate'leri iki sürümde de aynıdır.

---

## Lisans

[Apache License 2.0](LICENSE) — Copyright 2026 Toroslar Atatürk MTAL Yörü-K Teknoloji Takımı.

Atıf ve değişiklik bildirimi koşulları için [`NOTICE`](NOTICE) dosyasına bakın.

---

## Referanslar

- [iced — Rust GUI kütüphanesi](https://iced.rs)
- [Material Design 3](https://m3.material.io/)
- [usls — Rust görü modeli çalıştırma](https://github.com/jamjamjon/usls)
- [nokhwa — saf Rust kamera kütüphanesi](https://github.com/l1npengtul/nokhwa)
- [OpenStreetMap](https://www.openstreetmap.org)
