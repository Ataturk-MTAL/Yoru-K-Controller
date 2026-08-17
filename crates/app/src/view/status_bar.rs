//! Alt durum çubuğu — Slint `app.slint` içindeki status bar bloğunun karşılığı.

use iced::widget::{container, row, text};
use iced::{Alignment, Color, Element, Fill, Theme};

use crate::message::Message;
use crate::state::App;
use crate::styles;
use crate::theme::{Tokens, FONT_XS, SPACE_MD, SPACE_SM, SPACE_XL, STATUS_BAR_HEIGHT};
use crate::view::widgets::{dot, numeric, status_chip, v_separator};

/// Ayırıcı çizgi yüksekliği.
const DIVIDER_HEIGHT: f32 = 14.0;

type TextStyleFn = fn(&Theme) -> iced::widget::text::Style;

pub fn view(app: &App) -> Element<'_, Message> {
    let tokens = if app.is_dark {
        Tokens::dark()
    } else {
        Tokens::light()
    };

    let mut content = row![
        connection_section(app, &tokens),
        v_separator(DIVIDER_HEIGHT),
        motor_section(app, &tokens),
    ]
    .spacing(SPACE_XL)
    .align_y(Alignment::Center);

    if app.light_on {
        content = content.push(indicator(
            tokens.warning,
            "Işık",
            styles::text_warning as TextStyleFn,
        ));
    }
    if app.brake_on {
        content = content.push(indicator(
            tokens.error,
            "Fren",
            styles::text_error as TextStyleFn,
        ));
    }
    if app.gps_active {
        content = content.push(gps_section(app, &tokens));
    }

    // Durum metni kalan alanı doldurup sağa yaslanır. Ayrı bir esnek boşluk
    // kullanılınca satır taşıyor ve metnin sonu pencere kenarında kırpılıyordu.
    //
    // Ton `text_tertiary` değil `text_secondary`: buraya hata dizgeleri de
    // düşüyor ("port açılamadı" gibi) ve `on_surface_muted` bu uygulamada
    // pasif/placeholder metnin tonu — canlı bir bildirimi oraya koymak hatayı
    // sıradan bilgiden ayırt edilemez yapıyordu. Önem derecesine göre GERÇEK
    // ayrım (hata kırmızı, uyarı sarı) App'te bir seviye alanı gerektiriyor;
    // o alan Faz 3'te geliyor, burada yalnızca okunurluk düzeltiliyor.
    content = content.push(
        text(app.status_text.as_str())
            .size(FONT_XS)
            .width(Fill)
            .align_x(Alignment::End)
            .style(styles::text_secondary),
    );

    container(content)
        .height(STATUS_BAR_HEIGHT)
        .width(Fill)
        .padding([0.0, SPACE_XL])
        .style(styles::bar)
        .into()
}

/// Bağlantı göstergesi.
///
/// Aktarım adı kullanıcıya "Seri Port" olarak geçiyor — `connection_panel`
/// durum satırındaki adla birebir aynı. Burada eskiden "Serial" yazıyordu;
/// aynı bağlantı iki yerde iki isimle görünüyordu.
fn connection_section<'a>(app: &'a App, tokens: &Tokens) -> Element<'a, Message> {
    let color = if app.connected {
        tokens.sensor_online
    } else {
        tokens.sensor_offline
    };

    let label = if app.connected {
        let kind = if app.is_serial { "Seri Port" } else { "TCP/IP" };
        format!("{kind} — Bağlı")
    } else {
        "Bağlantı yok".to_string()
    };

    // Bağlantısızlık pasif bir durum değil, okunması gereken bir bilgi:
    // soluk `text_tertiary` yerine `text_secondary`. Şiddeti nokta rengi
    // (`sensor_offline`) taşıyor.
    let style = if app.connected {
        styles::text_success as TextStyleFn
    } else {
        styles::text_secondary as TextStyleFn
    };

    status_chip(color, label, FONT_XS, style)
}

fn motor_section<'a>(app: &'a App, tokens: &Tokens) -> Element<'a, Message> {
    let color = if app.motor_running {
        tokens.motor_running
    } else {
        tokens.motor_stopped
    };

    let (label, style) = if app.motor_running {
        ("Motor Çalışıyor", styles::text_success as TextStyleFn)
    } else {
        ("Motor Durdu", styles::text_secondary as TextStyleFn)
    };

    let mut section = row![status_chip(color, label.to_string(), FONT_XS, style)]
        .spacing(SPACE_SM)
        .align_y(Alignment::Center);

    if app.motor_running {
        section = section.push(
            text!("V{}", app.gear)
                .size(FONT_XS)
                .style(styles::text_warning),
        );
    }

    section.into()
}

/// Işık / fren göstergesi.
fn indicator<'a>(color: Color, label: &'a str, style: TextStyleFn) -> Element<'a, Message> {
    status_chip(color, label.to_string(), FONT_XS, style)
}

/// GPS koordinatı — dört ondalık basamağa yuvarlanır (Slint ile aynı).
///
/// Tek `status_chip` kullanmayan gösterge: koordinat sabit genişlikli fontla
/// yazılmak zorunda (aşağıdaki nota bakın), `status_chip` ise gövde fontunu
/// kullanıyor. Nokta çapı yine de çipin çapıyla aynı token'dan (`SPACE_MD`)
/// geliyor; yan yana duran iki noktanın farklı çapta olması nokta boyutunu
/// bilgi taşıyormuş gibi gösteriyordu.
fn gps_section<'a>(app: &'a App, tokens: &Tokens) -> Element<'a, Message> {
    let has_fix = app.gps_point_count > 0;

    let color = if has_fix {
        tokens.sensor_online
    } else {
        tokens.sensor_warning
    };

    let label = if has_fix {
        format!("{:.4}°  {:.4}°", app.gps_lat, app.gps_lon)
    } else {
        "GPS bekliyor...".to_string()
    };

    // Koordinat sürekli değişiyor — sabit genişlikli font olmadan rakamlar
    // her güncellemede yatayda oynuyor.
    let readout = if has_fix {
        numeric(label).size(FONT_XS).style(styles::text_success)
    } else {
        text(label).size(FONT_XS).style(styles::text_warning)
    };

    row![dot(color, SPACE_MD), readout]
        .spacing(SPACE_SM)
        .align_y(Alignment::Center)
        .into()
}
