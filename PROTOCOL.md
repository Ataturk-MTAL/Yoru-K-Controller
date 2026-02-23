# Yörü-K İletişim Protokolü — Teknik Referans

Bu belge, Yörü-K robot kontrolcüsünün ESP32 ile haberleşmesinde kullandığı ikili
(binary) iletişim protokolünü kapsamlı biçimde açıklar.

Referans uygulamalar: `crates/protocol/src/packet.rs` (Rust) ve
`core/protocol_controller.py` (Python/PySide6, referans kaynak).

---

## İçindekiler

1. [Paket Genel Formatı](#1-paket-genel-formatı)
2. [Checksum Algoritmaları](#2-checksum-algoritmaları)
3. [Komut Paketleri (Kontrolcü → Robot)](#3-komut-paketleri-kontrolcü--robot)
4. [Sorgu Paketleri](#4-sorgu-paketleri)
5. [Yanıt Paketleri (Robot → Kontrolcü)](#5-yanıt-paketleri-robot--kontrolcü)
6. [Hız Paketi v2.3 — Ayrıntılı Format](#6-hız-paketi-v23--ayrıntılı-format)
7. [Zamanlama ve Kuyruk Modeli](#7-zamanlama-ve-kuyruk-modeli)
8. [RX Çerçeveleme (Stream Parsing)](#8-rx-çerçeveleme-stream-parsing)
9. [Motor Ters Bağlantı Kuralı](#9-motor-ters-bağlantı-kuralı)
10. [Örnek Byte Dizileri](#10-örnek-byte-dizileri)
11. [Hata Yönetimi](#11-hata-yönetimi)

---

## 1. Paket Genel Formatı

```
┌──────────┬──────────┬──────────┬─────────────────┬──────────┬──────────┐
│  0xAA    │   CMD    │   LEN    │   DATA[0..LEN)  │   CK2    │   CK1    │
│  1 byte  │  1 byte  │  1 byte  │   0–255 bytes   │  1 byte  │  1 byte  │
│ (sabit)  │          │          │                 │ Fletcher │ Fletcher │
└──────────┴──────────┴──────────┴─────────────────┴──────────┴──────────┘
```

| Alan | Boyut | Değer | Açıklama |
|---|---|---|---|
| `START_BYTE` | 1 | `0xAA` | Her paketin ilk baytı, sabit |
| `CMD` | 1 | bkz. §3–§4 | Komut veya sorgu kodu |
| `LEN` | 1 | 0–255 | `DATA` alanının uzunluğu (checksum dahil değil) |
| `DATA` | LEN | komuta özgü | Veri yükü |
| `CK2`, `CK1` | 2 | Fletcher-16 | `pkt[1..]` (START_BYTE hariç tüm önceki baytlar) üzerinden |

**Toplam paket uzunluğu:** `3 + LEN + 2` bayt

> **Önemli:** Checksum `pkt[1..]` üzerinden hesaplanır — yani START_BYTE (`0xAA`) checksum'a **dahil değildir**.
> Rust: `fletcher16(&pkt[1..])` · Python: `_checksum(pkt[1:])`

---

## 2. Checksum Algoritmaları

Uygulama üç checksum yöntemi destekler. **Varsayılan: Fletcher-16.**

### 2.1 Fletcher-16 (Varsayılan)

İki 8-bit toplamın çarpraz bağımlılığını kullanır; bit hatalarını ve yer değiştirmeleri
(transposition) tespit eder. Sonuç 2 bayttır: `[sum2, sum1]`.

```
sum1 = 0,  sum2 = 0
for each byte b in data:
    sum1 = (sum1 + b) mod 255
    sum2 = (sum2 + sum1) mod 255
checksum = [sum2, sum1]
```

```rust
// Rust uygulaması — crates/protocol/src/packet.rs
pub fn fletcher16(data: &[u8]) -> [u8; 2] {
    let mut sum1: u32 = 0;
    let mut sum2: u32 = 0;
    for &b in data {
        sum1 = (sum1 + b as u32) % 255;
        sum2 = (sum2 + sum1) % 255;
    }
    [sum2 as u8, sum1 as u8]
}
```

**Özellikler:**
- Tüm tek-bit hatalarını tespit eder
- Çoğu burst hatayı tespit eder
- Yer değiştirme hatalarını tespit eder
- 2 bayt ek yük

### 2.2 CRC-8 (Alternatif)

Polinom: `x⁸ + x² + x + 1` (`0x07`). Endüstri standardı; I²C ve 1-Wire'da yaygın.

```
crc = 0x00
for each byte b in data:
    crc = crc XOR b
    for 8 iterations:
        if crc & 0x80:
            crc = (crc << 1) XOR 0x07
        else:
            crc = crc << 1
        crc = crc & 0xFF
```

**Özellikler:**
- Tüm tek ve çift bit hatalarını tespit eder
- 1 bayt ek yük
- Fletcher-16'ya kıyasla hesaplama maliyeti daha yüksek

### 2.3 Simple (Basit, Test Amaçlı)

```
checksum = sum(data) & 0xFF
```

**Özellikler:**
- En hızlı hesaplama
- Yer değiştirme hatalarını tespit **etmez**
- Yalnızca düşük gürültülü ortamlarda veya test sırasında kullanın

---

## 3. Komut Paketleri (Kontrolcü → Robot)

### 3.1 Komut Kodu Tablosu

| Kod | Sabit | Yön | LEN | Açıklama |
|---|---|---|---|---|
| `0xFF` | `SetStart` | TX | 0 | Motorları başlat |
| `0x00` | `SetStop` | TX | 0 | Motorları durdur (acil) |
| `0x01` | `SetSpeed` | TX | 7 | Sol + sağ motor hız paketi (v2.3) |
| `0x02` | `SetLight` | TX | 1 | Işık aç / kapat |
| `0x03` | `SetBrake` | TX | 1 | Fren aç / kapat |

### 3.2 START Paketi (`0xFF`)

Motorları etkinleştirir. Başlatmadan önce her zaman gönderilir.

```
AA FF 00 [CK2] [CK1]
```

LEN = 0, DATA alanı boş.

**Örnek:**
```
AA FF 00 FB 00
```

### 3.3 STOP Paketi (`0x00`)

Motorları durdurur. Acil durumlarda `priority=true` ile kuyruğu atlayarak anında gönderilir.
Gönderimden sonra tüm hız değerleri sıfırlanır.

```
AA 00 00 [CK2] [CK1]
```

**Örnek:**
```
AA 00 00 00 00
```

### 3.4 SPEED Paketi (`0x01`) — v2.3 Format

Bkz. [§6](#6-hız-paketi-v23--ayrıntılı-format) (tam ayrıntı).

### 3.5 LIGHT Paketi (`0x02`)

```
AA 02 01 [on] [CK2] [CK1]
```

| Bayt | Değer | Anlam |
|---|---|---|
| `DATA[0]` | `0x01` | Işık aç |
| `DATA[0]` | `0x00` | Işık kapat |

**Örnekler:**
```
AA 02 01 01 [CK2] [CK1]   ← Işık AÇ
AA 02 01 00 [CK2] [CK1]   ← Işık KAPAT
```

### 3.6 BRAKE Paketi (`0x03`)

```
AA 03 01 [on] [CK2] [CK1]
```

`DATA[0]`: `0x01` = fren aktif, `0x00` = fren pasif.

---

## 4. Sorgu Paketleri

Tüm sorgular `_simple_packet(cmd)` ile oluşturulur: `AA [CMD] 00 [CK2] [CK1]`.

| Kod | Sabit | Yanıt Kodu | Açıklama |
|---|---|---|---|
| `0x10` | `GetStatus` | `0x10` | Motor çalışıyor mu? |
| `0x11` | `GetSpeed` | `0x11` | Anlık vites + sol/sağ hız |
| `0x12` | `GetLight` | `0x12` | Işık durumu |
| `0x13` | `GetBrake` | `0x13` | Fren durumu |
| `0x14` | `GetGps` | `0x14` | GPS koordinatı *(Faz 2)* |

**Durum sorgusu örneği:**
```
AA 10 00 [CK2] [CK1]
```

---

## 5. Yanıt Paketleri (Robot → Kontrolcü)

Robot, her sorguya aynı CMD kodu ile yanıt verir.

### 5.1 GET_STATUS Yanıtı (`0x10`)

```
AA 10 01 [status] [CK2] [CK1]
```

| `DATA[0]` | Anlam |
|---|---|
| `0x01` | Motor çalışıyor |
| `0x00` | Motor durmuş |

### 5.2 GET_SPEED Yanıtı (`0x11`)

```
AA 11 07 [gear] 4C [left_dir] [left_abs] 52 [right_dir] [right_abs] [CK2] [CK1]
```

Hız paketi formatının aynısı (§6); `left_abs` ve `right_abs` işaretsiz (0–100).
İşaret kurtarma: `speed = left_abs if left_dir == 0x46 else -left_abs`.

Rust parse kodu (`crates/protocol/src/packet.rs`):
```rust
let left  = if left_dir  == 0x46 { left_abs  } else { -left_abs  };
let right = if right_dir == 0x46 { right_abs } else { -right_abs };
```

### 5.3 GET_LIGHT Yanıtı (`0x12`)

```
AA 12 01 [on] [CK2] [CK1]
```

`DATA[0]`: `0x01` = açık, `0x00` = kapalı.

### 5.4 GET_BRAKE Yanıtı (`0x13`)

```
AA 13 01 [on] [CK2] [CK1]
```

`DATA[0]`: `0x01` = fren aktif, `0x00` = fren pasif.

### 5.5 GET_GPS Yanıtı (`0x14`)

```
AA 14 08 [LAT_B0] [LAT_B1] [LAT_B2] [LAT_B3]
          [LON_B0] [LON_B1] [LON_B2] [LON_B3]
          [CK2] [CK1]
```

| Baytlar | İçerik | Format |
|---|---|---|
| `DATA[0..4]` | Enlem (latitude) | `float32` little-endian (IEEE 754) |
| `DATA[4..8]` | Boylam (longitude) | `float32` little-endian (IEEE 754) |

Python parse:
```python
lat = struct.unpack('<f', data[3:7])[0]
lon = struct.unpack('<f', data[7:11])[0]
```

Rust parse (`crates/protocol/src/packet.rs`):
```rust
let lat = f32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
let lon = f32::from_le_bytes([payload[4], payload[5], payload[6], payload[7]]);
```

---

## 6. Hız Paketi v2.3 — Ayrıntılı Format

Bu format, `protocol_controller.py` yorumundaki "Protocol v2.3" ile etiketlenmiştir.

```
Byte indeksi:  0    1    2    3    4    5    6    7    8    9   10   11
               AA   01   07  [VV] [4C] [DD] [SS] [52] [DD] [SS] CK2  CK1
```

| İndeks | Değer | Açıklama |
|---|---|---|
| `[0]` | `0xAA` | START_BYTE |
| `[1]` | `0x01` | CMD = SetSpeed |
| `[2]` | `0x07` | LEN = 7 (veri uzunluğu) |
| `[3]` | `1`–`3` | Vites (gear): 1, 2 veya 3 |
| `[4]` | `0x4C` | `'L'` — sol motor işaretçisi |
| `[5]` | `0x46` / `0x42` | Yön: `'F'`=ileri, `'B'`=geri |
| `[6]` | `0`–`100` | Sol motor mutlak hızı (işaretsiz) |
| `[7]` | `0x52` | `'R'` — sağ motor işaretçisi |
| `[8]` | `0x46` / `0x42` | Yön: `'F'`=ileri, `'B'`=geri |
| `[9]` | `0`–`100` | Sağ motor mutlak hızı (işaretsiz) |
| `[10]` | — | CK2 (Fletcher-16 üst baytı) |
| `[11]` | — | CK1 (Fletcher-16 alt baytı) |

**Toplam:** 12 bayt

### Yön Kodlaması

```
0x46 = 'F' = İleri (Forward)    ← hız ≥ 0
0x42 = 'B' = Geri  (Backward)   ← hız < 0
```

### Hız Kodlama Adımları

1. Ham hız: `−100 … +100` (signed i8)
2. Motor ters bağlantı uygulanır (bkz. §9)
3. Yön baytı belirlenir: `hız ≥ 0 → 0x46`, `hız < 0 → 0x42`
4. Mutlak değer alınır: `abs(hız)` → `0 … 100` (unsigned u8)
5. Paket oluşturulur

Rust uygulaması:
```rust
let l = if rev_left { left.saturating_neg() } else { left }.clamp(-100, 100);
let dir = |v: i8| if v >= 0 { 0x46u8 } else { 0x42u8 };
// paket[5] = dir(l), paket[6] = l.unsigned_abs()
```

---

## 7. Zamanlama ve Kuyruk Modeli

### 7.1 Zamanlama Parametreleri

| Parametre | Değer | Açıklama |
|---|---|---|
| Periyodik hız gönderimi | **50 ms** (20 Hz) | Joystick hızı, yalnızca motor çalışıyorken |
| Durum sorgulama | **500 ms** | `GetStatus` her yarım saniyede bir |
| Mesaj kuyruğu işleme | **5 ms** (200 Hz) | Kuyruktan bir mesaj al ve gönder |

### 7.2 Hysteresis Filtresi (Paket Spam Önleme)

Periyodik göndericisi, değerler yeterince değişmemişse paketi atlar:

```
|Δsol| > 2  VEYA  |Δsağ| > 2  VEYA  vites_değişti
    → Paket oluştur ve gönder
    aksi hâlde → atla
```

Özel durum: Her iki hız da 0 ve önceki gönderim 0 değilse yine de gönder
(tam durma garantisi).

Rust uygulaması (`crates/app/src/bridge.rs`):
```rust
if diff_l > 2 || diff_r > 2 || gear != prev_gear {
    let pkt = speed_packet(speeds.left, speeds.right, gear, rev_l, rev_r);
    state.connection.lock().unwrap().send(pkt);
}
```

### 7.3 Mesaj Önceliği

| Komut | Öncelik | Davranış |
|---|---|---|
| `SetStart` | **Yüksek** | Kuyruğu atlar, anında gönderilir |
| `SetStop` | **Yüksek** | Kuyruğu atlar, anında gönderilir |
| `SetSpeed` | Normal | Kuyruğa eklenir |
| `SetLight` | Normal | Kuyruğa eklenir |
| `SetBrake` | Normal | Kuyruğa eklenir |
| `GetStatus` | Normal | Kuyruğa eklenir |

> **Not:** Rust uygulamasında `mpsc` kanalı öncelik mekanizması içermez;
> START/STOP anında `ConnectionManager::send()` ile gönderilir.

---

## 8. RX Çerçeveleme (Stream Parsing)

Seri port ve TCP stream'i sürekli veri akışıdır. Paket sınırları `0xAA`
ve `LEN` alanı kullanılarak belirlenir.

### 8.1 Algoritma

```
loop:
    1. Buffer'da 0xAA ara
    2. Bulamazsan → daha fazla veri bekle
    3. Öncesindeki baytları at (gürültü)
    4. Buffer < 3 bayt → daha fazla veri bekle
    5. total = 3 + LEN + 2  (AA + CMD + LEN + DATA + 2 checksum)
    6. Buffer < total → daha fazla veri bekle
    7. pkt = buffer[0..total]
    8. Checksum doğrula: fletcher16(pkt[1..total-2]) == pkt[total-2..total]
    9a. Geçerliyse → paketi döndür
    9b. Geçersizse → ilk baytı at, başa dön (bozuk paket)
```

Rust uygulaması (`crates/transport/src/framing.rs`):
```rust
pub fn try_parse_packet(buf: &mut Vec<u8>) -> Option<Vec<u8>> {
    loop {
        let start = buf.iter().position(|&b| b == 0xAA)?;
        if start > 0 { buf.drain(..start); }
        if buf.len() < 3 { return None; }
        let data_len = buf[2] as usize;
        let total = 3 + data_len + 2;
        if buf.len() < total { return None; }
        let pkt = buf[..total].to_vec();
        let expected = protocol::fletcher16(&pkt[1..total-2]);
        let actual   = [pkt[total-2], pkt[total-1]];
        buf.drain(..total);
        if expected == actual { return Some(pkt); }
        // Checksum hatalı → bir sonraki 0xAA'ya geç
    }
}
```

### 8.2 Buffer Yönetimi

- Serial worker: `Vec<u8>` accumulation buffer, bloklanmayan `try_recv` döngüsü
- TCP worker: `BytesMut` (tokio), `select!` makrosu ile okuma ve yazma eşzamanlı

---

## 9. Motor Ters Bağlantı Kuralı

Sırt sırta (back-to-back) monte edilmiş motorlarda fiziksel dönüş yönü
birbirine terstir. Yazılım bu durumu `reverse_left` / `reverse_right` bayrakları
ile çözer.

### Uygulama Sırası

```
1. Ham joystick hızı (−100 … +100)
2. reverse_left  == true → left_speed  = -left_speed
   reverse_right == true → right_speed = -right_speed
3. Yön baytı belirlenir (adım 2 sonrası işarete bakılır)
4. Mutlak değer alınır
```

> **Kritik:** Tersine çevirme, yön ve mutlak değer hesabından **önce** yapılmalıdır.
> Bu sıra bozulursa doğru yön baytı üretilmez.

### Mutex Kuralı

Sol Ters ve Sağ Ters bayrakları aynı anda etkin olamaz:

```
set_reverse_left(true)  → reverse_right otomatik false olur
set_reverse_right(true) → reverse_left  otomatik false olur
```

Bu kural UI katmanında (Slint state / AppState) uygulanır; protokol katmanı
her iki bayrağı bağımsız kabul eder.

---

## 10. Örnek Byte Dizileri

### Tam Hız İleri, V1

```
Sol: +100  Sağ: +100  Vites: 1
AA 01 07 01 4C 46 64 52 46 64 [CK2] [CK1]
          ^^ ^^ ^^ ^^ ^^ ^^
          |  |  |  |  |  └─ Sağ abs: 100 (0x64)
          |  |  |  |  └─── Sağ yön: 'F' (ileri)
          |  |  |  └────── 'R' işaretçisi
          |  |  └───────── Sol abs: 100 (0x64)
          |  └──────────── Sol yön: 'F' (ileri)
          └─────────────── 'L' işaretçisi
                           (bayt [3] = 01 = vites 1)
```

Checksum: `fletcher16([01, 07, 01, 4C, 46, 64, 52, 46, 64])` = `[94, B3]` (yaklaşık)

### Sol Dönüş, V2

```
Sol: −50  Sağ: +50  Vites: 2
AA 01 07 02 4C 42 32 52 46 32 [CK2] [CK1]
             ^^    ^^    ^^
             |     |     └── Sağ yön: 'F'
             |     └──────── Sol yön: 'B' (geri)
             └────────────── Vites 2
```

### START Komutu

```
AA FF 00 [CK2] [CK1]
```

`fletcher16([FF, 00])` = `[FB, 00]` → `AA FF 00 FB 00`

### STOP Komutu

```
AA 00 00 [CK2] [CK1]
```

`fletcher16([00, 00])` = `[00, 00]` → `AA 00 00 00 00`

### Durum Sorgula

```
AA 10 00 [CK2] [CK1]
```

`fletcher16([10, 00])` → `AA 10 00 10 10`

### Durum Yanıtı — Motor Çalışıyor

```
AA 10 01 01 [CK2] [CK1]
```

### GPS Yanıtı — İstanbul Koordinatları

```
Enlem:  41.0082° N → IEEE 754 LE: 79 E0 24 42
Boylam: 28.9784° E → IEEE 754 LE: D1 F1 E7 41

AA 14 08 79 E0 24 42 D1 F1 E7 41 [CK2] [CK1]
```

---

## 11. Hata Yönetimi

### Checksum Hatası

`try_parse_packet()` checksum uyuşmazlığında paketi sessizce atar ve buffer'da
bir sonraki `0xAA` konumundan devam eder. Üst katmana hata iletilmez; kayıp
paket bir sonraki periyodik sorguda (500ms) telafi edilir.

### Bağlantı Hatası

| Katman | Hata Türü | Davranış |
|---|---|---|
| Serial | `TimedOut` | Yok sayılır; bloklanmayan okuma |
| Serial | Diğer I/O hataları | `RobotEvent::Error` → UI durum çubuğu |
| TCP | `EOF` (Ok(0)) | `RobotEvent::Disconnected` |
| TCP | I/O hatası | `RobotEvent::Error` |

### Temiz Kapanış Sırası

```
1. Periyodik send timer durdur
2. Durum sorgu timer durdur
3. STOP komutu gönder (yüksek öncelikli)
4. 100ms bekle (komutun iletilebilmesi için)
5. Port / socket kapat
6. Worker thread'leri sonlandır (2sn timeout)
```

---

## Protokol Versiyonu

| Versiyon | Değişiklik |
|---|---|
| v1.0 | Temel START/STOP/SPEED |
| v2.0 | LEN alanı eklendi |
| v2.3 | Hız paketine yön bayrakları (`'L'`/`'R'`/`'F'`/`'B'`) ve `LEN=7` |
| Faz 2 | GPS sorgulama (`0x14`) etkinleştirilecek |

---

*Bu belge `core/protocol_controller.py` ve `crates/protocol/src/packet.rs` kaynak kodundan
türetilmiştir ve ikisi arasında birebir uyum sağlayacak şekilde yazılmıştır.*
