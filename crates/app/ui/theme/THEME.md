# AppTheme — Slint Dark/Light Tema Paketi

Slint'in resmi `Palette` global'i ile senkron çalışan, endüstriyel/IoT projelerine uygun kapsamlı tema sistemi.

## Dosya Yapısı

```
theme/
├── theme.slint        # AppTheme global — tüm renk, spacing, radius, font 
├── components.slint   # Hazır bileşenler — TButton, TCard, TBadge, TInput, TMotorButton...
└── demo.slint         # Showcase uygulaması
```

## Hızlı Başlangıç

### 1. Dosyaları projeye kopyala

```
my-project/
├── ui/
│   ├── theme/
│   │   ├── theme.slint
│   │   └── components.slint
│   └── app.slint
├── src/
│   └── main.rs
└── Cargo.toml
```

### 2. Import et

```slint
// app.slint
import { AppTheme } from "theme/theme.slint";
import { TCard, TButton, TBadge, TMotorButton, TSensorValue, ThemeToggle,
         ButtonVariant, BadgeVariant, StatusLevel } from "theme/components.slint";

export component App inherits Window {
    background: AppTheme.bg;

    VerticalLayout {
        padding: AppTheme.space-xl;
        spacing: AppTheme.space-lg;

        // Sağ üstte tema geçişi
        HorizontalLayout {
            alignment: space-between;
            Text {
                text: "SCADA Panel";
                color: AppTheme.text-primary;
                font-size: AppTheme.font-xl;
                font-weight: 700;
            }
            ThemeToggle {}
        }

        // Motor kontrol
        TMotorButton {
            motor-name: "Pompa 1";
            running: false;
            clicked => { self.running = !self.running; }
        }

        // Sensör kartları
        HorizontalLayout {
            spacing: AppTheme.space-md;

            TSensorValue {
                label: "Sıcaklık";
                value: "45.2";
                unit: "°C";
                status: StatusLevel.online;
            }
        }
    }
}
```

### 3. Rust tarafından tema kontrolü

```rust
use slint::ComponentHandle;

fn main() {
    let app = App::new().unwrap();

    // Programatik olarak dark mode aç
    app.global::<slint_generatedApp::Palette>()
       .set_color_scheme(slint_generatedApp::ColorScheme::Dark);

    app.run().unwrap();
}
```

## Renk Sistemi

### Yüzeyler (Surfaces)

| Token | Kullanım |
|-------|----------|
| `bg` | Pencere arka planı |
| `bg-secondary` | İkincil arka plan |
| `surface` | Kart / panel yüzeyi |
| `surface-raised` | Elevated kart |
| `surface-overlay` | Dropdown / popup |
| `surface-sunken` | Input alanı iç kısım |

### Metin

| Token | Kullanım |
|-------|----------|
| `text-primary` | Ana metin |
| `text-secondary` | Açıklama metni |
| `text-tertiary` | Hint / placeholder |
| `text-disabled` | Devre dışı metin |

### Semantic

| Token | Kullanım |
|-------|----------|
| `success` / `success-subtle` / `success-text` | Başarı, aktif, çalışıyor |
| `warning` / `warning-subtle` / `warning-text` | Uyarı, dikkat |
| `error` / `error-subtle` / `error-text` | Hata, arıza, tehlike |
| `info` / `info-subtle` / `info-text` | Bilgi, referans |

### Endüstriyel / IoT

| Token | Kullanım |
|-------|----------|
| `motor-running` | Motor çalışıyor (yeşil) |
| `motor-stopped` | Motor durdu (gri) |
| `motor-fault` | Motor arıza (kırmızı) |
| `sensor-online/offline/warning` | Sensör durumu |
| `value-normal/high/critical` | Ölçüm değer aralığı |

## Bileşenler

| Bileşen | Açıklama |
|---------|----------|
| `ThemeToggle` | Dark/Light geçiş anahtarı |
| `TCard` | Kenarlıklı veya gölgeli kart |
| `TButton` | primary / secondary / danger / ghost varyantları |
| `TBadge` | success / warning / error / info / neutral etiket |
| `TInput` | Placeholder ve hata destekli text input |
| `TStatusDot` | online / warning / offline / idle durumu |
| `TSeparator` | Yatay ayırıcı çizgi |
| `TMotorButton` | Motor kontrol butonu (running/fault/disabled) |
| `TSensorValue` | Sensör değer kartı (label + value + unit + status) |

## Spacing & Radius Tokenleri

```
space-xs: 2px    radius-xs: 2px
space-sm: 4px    radius-sm: 4px
space-md: 8px    radius-md: 6px
space-lg: 12px   radius-lg: 8px
space-xl: 16px   radius-xl: 12px
space-2xl: 24px  radius-2xl: 16px
space-3xl: 32px  radius-full: 999px
```

## Animasyon Süreleri

```
anim-fast:   100ms   (hover, press)
anim-normal: 180ms   (tema geçişi, renk değişimi)
anim-slow:   300ms   (layout geçişi, expand/collapse)
```
