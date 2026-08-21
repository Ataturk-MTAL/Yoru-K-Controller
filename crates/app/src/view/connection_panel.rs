//! Bağlantı paneli — Slint `components/connection_panel.slint` karşılığı.

use iced::widget::{button, column, pick_list, row, text, text_input};
use iced::{Alignment, Border, Element, Fill, Theme};

use crate::message::Message;
use crate::state::App;
use crate::styles;
use crate::theme::{
    self, Tokens, CONTROL_HEIGHT, FIELD_PADDING_X, FIELD_PADDING_Y, FONT_MD, FONT_SM, SPACE_MD,
    SPACE_SM,
};
use crate::view::widgets::{icon_button, section_title, status_chip};

/// Seri port satırındaki "Port:" etiketinin genişliği (Slint: 34px).
const LABEL_WIDTH: f32 = 34.0;
/// Mod seçici satır yüksekliği — 30 px'lik hedef alanı küçüktü.
const ROW_HEIGHT: f32 = CONTROL_HEIGHT;

/// Seri bağlantının kullanıcıya görünen adı.
///
/// Aynı bağlantı iki ayrı yerde iki ayrı isimle yazılıyordu: burada "Seri Port",
/// durum çubuğunda (`view/status_bar.rs`) İngilizce "Serial". Arayüz Türkçe
/// olduğu için Türkçe biçim kazandı; TCP tarafı protokol adı olduğu için
/// çevrilmiyor ("TCP/IP"). Durum çubuğu da bu adı kullanmalı — iki farklı isim
/// aynı durumu iki ayrı bağlantı gibi gösteriyordu.
const SERIAL_LABEL: &str = "Seri Port";

type ButtonStyleFn = fn(&Theme, button::Status) -> button::Style;
type TextStyleFn = fn(&Theme) -> text::Style;

pub fn view(app: &App) -> Element<'_, Message> {
    column![
        section_title("BAĞLANTI"),
        mode_selector(app),
        endpoint_fields(app),
        connect_button(app),
        status_line(app),
    ]
    .spacing(SPACE_MD)
    .into()
}

/// Seri Port / TCP/IP segment seçici.
///
/// Etiket `SERIAL_LABEL`'dan geliyor: mod düğmesi ile alttaki durum satırı aynı
/// bağlantıdan söz ediyor, ikisinin farklı yazması iki ayrı şey izlenimi verir.
fn mode_selector(app: &App) -> Element<'_, Message> {
    // Bağlantı kurulmuşken (ya da kurulurken) kilitli: aksi hâlde seri porta
    // bağlıyken "TCP/IP"ye basmak yalnızca `is_serial` bayrağını çeviriyor ve
    // hem bu paneldeki durum satırı hem alt durum çubuğu "TCP/IP — Bağlı"
    // yazıyor. Açık olan hâlâ seri port; arayüz hattın gerçeğinden ayrılıyor.
    let locked = app.connected || app.connecting;

    row![
        button(text(SERIAL_LABEL).size(FONT_SM).center())
            .width(Fill)
            .height(ROW_HEIGHT)
            .style(styles::segment(app.is_serial))
            .on_press_maybe((!locked).then_some(Message::SerialModeSelected(true))),
        button(text("TCP/IP").size(FONT_SM).center())
            .width(Fill)
            .height(ROW_HEIGHT)
            .style(styles::segment(!app.is_serial))
            .on_press_maybe((!locked).then_some(Message::SerialModeSelected(false))),
    ]
    .spacing(SPACE_SM)
    .into()
}

/// Seçili moda göre port seçici ya da host/port alanları.
fn endpoint_fields(app: &App) -> Element<'_, Message> {
    if app.is_serial && app.available_ports.is_empty() {
        // Boş `pick_list` açıldığında boş bir menü gösteriyordu: liste gerçekten
        // boş mu, yoksa yenilenmedi mi — ayırt edilemiyordu. ↻ düğmesi kalıyor.
        row![
            text("Seri port bulunamadı — kabloyu takıp ↻ ile yenileyin")
                .size(FONT_SM)
                .style(styles::text_secondary)
                .width(Fill),
            icon_button("↻", Some(Message::PortsRefreshRequested)),
        ]
        .spacing(SPACE_SM)
        .align_y(Alignment::Center)
        .into()
    } else if app.is_serial {
        row![
            text("Port:")
                .size(FONT_SM)
                .width(LABEL_WIDTH)
                .style(styles::text_secondary),
            pick_list(
                app.available_ports.as_slice(),
                app.serial_port.as_ref(),
                Message::PortSelected,
            )
            .placeholder("Port seçin")
            .text_size(FONT_SM)
            .padding([FIELD_PADDING_Y, FIELD_PADDING_X])
            .style(styles::field)
            .width(Fill),
            icon_button("↻", Some(Message::PortsRefreshRequested)),
        ]
        .spacing(SPACE_SM)
        .align_y(Alignment::Center)
        .into()
    } else {
        row![
            text_input("192.168.4.1", &app.tcp_host)
                .on_input(Message::HostChanged)
                .font(theme::mono())
                .padding([FIELD_PADDING_Y, FIELD_PADDING_X])
                .size(FONT_SM)
                .style(validated_input(app.tcp_host_valid()))
                .width(Fill),
            text(":").size(FONT_MD).style(styles::text_secondary),
            text_input("80", &app.tcp_port)
                .on_input(Message::TcpPortChanged)
                .font(theme::mono())
                .padding([FIELD_PADDING_Y, FIELD_PADDING_X])
                .size(FONT_SM)
                .style(validated_input(app.tcp_port_value().is_some()))
                .width(64.0),
        ]
        .spacing(SPACE_SM)
        .align_y(Alignment::Center)
        .into()
    }
}

/// Geçerlilik geri bildirimi veren metin kutusu stili.
///
/// Gövde `styles::input`'tan geliyor; burada yalnızca geçersiz girdide kenarlık
/// `error`'a çevriliyor. Geçerliyken hiçbir şey eklenmiyor: sürekli yeşil bir
/// "doğru" kenarlığı gürültü, hatanın kendisi ise fark edilmez oluyor — üstelik
/// odak göstergesi de (`primary` kenarlık) o rengin altında kalıyordu.
///
/// Kenarlık kalınlığı `styles::input`'un 1 px'i olarak bırakılıyor. Host alanı
/// eskiden 1.5 px'lik ayrı bir sarmalayıcı `container` içindeydi; text_input
/// kendi kenarlığını da çizdiği için sonuç çift çerçeveydi ve uygulamadaki tek
/// 1.5 px'lik kenarlıktı.
///
/// Tek yardımcı iki alana da bakıyor: host ile port aynı satırdaki iki kardeş,
/// port alanı eskiden geçersizken (0 ya da 65535 üstü) hiç uyarı vermiyordu.
fn validated_input(is_valid: bool) -> impl Fn(&Theme, text_input::Status) -> text_input::Style {
    move |theme: &Theme, status: text_input::Status| {
        let base = styles::input(theme, status);
        if is_valid {
            return base;
        }

        text_input::Style {
            border: Border {
                color: Tokens::for_theme(theme).error,
                ..base.border
            },
            ..base
        }
    }
}

/// BAĞLAN / BAĞLANTIYI KES.
fn connect_button(app: &App) -> Element<'_, Message> {
    let (label, message, style): (&str, Option<Message>, ButtonStyleFn) = if app.connected {
        (
            "BAĞLANTIYI KES",
            Some(Message::DisconnectPressed),
            styles::secondary,
        )
    } else if app.connecting {
        // Kilitli: ikinci basış aynı hedefe ikinci bir bağlantı denemesi
        // başlatır. Seri port açılışı ve TCP el sıkışması gözle görülür sürüyor,
        // etiket olmadan BAĞLAN hiçbir şey yapmamış gibi görünüyordu.
        ("BAĞLANIYOR…", None, styles::accent)
    } else {
        (
            "BAĞLAN",
            app.can_connect().then_some(Message::ConnectPressed),
            styles::accent,
        )
    };

    button(text(label).size(FONT_SM).center())
        .width(Fill)
        .height(CONTROL_HEIGHT)
        .style(style)
        .on_press_maybe(message)
        .into()
}

/// Nokta + "Seri Port — Bağlı" satırı.
fn status_line(app: &App) -> Element<'_, Message> {
    let tokens = if app.is_dark {
        Tokens::dark()
    } else {
        Tokens::light()
    };

    let kind = if app.is_serial {
        SERIAL_LABEL
    } else {
        "TCP/IP"
    };

    let (color, label, style): (_, _, TextStyleFn) = if app.connected {
        (
            tokens.sensor_online,
            format!("{kind} — Bağlı"),
            styles::text_success,
        )
    } else if app.connecting {
        // Üçüncü bir durum: ne bağlı ne kopuk. Kırmızı nokta yanlış bilgi
        // verirdi (hata yok), yeşil de (bağlantı yok) — bekleme tonu.
        (
            tokens.sensor_warning,
            format!("{kind} — Bağlanıyor…"),
            styles::text_warning,
        )
    } else {
        (
            tokens.motor_stopped,
            "Bağlantı yok".to_string(),
            styles::text_tertiary,
        )
    };

    // Nokta çapı ve noktayla etiket arası boşluk `status_chip` içinde sabit:
    // buradaki 10 px, durum çubuğundaki 8 px'lik noktalarla yan yana düştüğünde
    // çap bilgi taşıyormuş gibi görünüyordu.
    status_chip(color, label, FONT_SM, style)
}
