//! Motor kontrol paneli — Slint `components/motor_control.slint` karşılığı.

use iced::widget::{button, column, row, text, Space};
use iced::{Alignment, Element, Fill, Theme};

use crate::message::Message;
use crate::state::{App, INTERVALS_MS};
use crate::styles;
use crate::theme::{CONTROL_HEIGHT, FONT_MD, FONT_SM, ICON_BUTTON, SPACE_MD, SPACE_SM};
use crate::view::widgets::{icon_button, numeric, section_title};

/// Adım satırlarındaki değer kutusunun genişliği (vites ve gönderim aralığı).
///
/// İki satırda da AYNI olmak zorunda: satırlar sağa dayalı (`Space` + `Fill`),
/// yani `+` butonları hizalı ama değer kutusu farklı genişlikteyse `−` butonu
/// satırlar arasında kayıyor. Eskiden 36 ve 56 yazılıydı — tam 20 px'lik bu
/// fark gözle görülüyordu. Ölçü `ICON_BUTTON` üzerinden türetiliyor: en uzun
/// değer ("50 ms", sabit genişlikli fontta 5 karakter ≈ 36 px) sığsın diye
/// bir `SPACE_MD` pay bırakılıyor.
const VALUE_WIDTH: f32 = ICON_BUTTON + SPACE_MD;

pub fn view(app: &App) -> Element<'_, Message> {
    column![
        section_title("MOTOR KONTROL"),
        motor_button(app),
        gear_row(app),
        interval_row(app),
        peripheral_row(app),
        reverse_row(app),
    ]
    .spacing(SPACE_MD)
    .into()
}

/// Slint `TMotorButton` — bağlantı yokken pasif.
fn motor_button(app: &App) -> Element<'_, Message> {
    let label = if app.motor_running {
        "■  Motor DURDUR"
    } else {
        "▶  Motor BAŞLAT"
    };

    let message = app.connected.then_some({
        if app.motor_running {
            Message::StopPressed
        } else {
            Message::StartPressed
        }
    });

    // Birincil eylem de standart kontrol yüksekliğinde: ayrı bir "motor butonu
    // yüksekliği" sabiti tutmak, tek doğruluk kaynağını ikiye bölüyordu.
    let control = button(text(label).size(FONT_MD).center())
        .width(Fill)
        .height(CONTROL_HEIGHT)
        .style(styles::motor(app.motor_running))
        .on_press_maybe(message);

    // Uygulamanın birincil eylemi bu; pasifken *neden* pasif olduğu yazılmazsa
    // kullanıcı soluk butona bakıp takılıyor.
    if app.connected {
        control.into()
    } else {
        column![
            control,
            text("Motoru çalıştırmak için önce robota bağlan")
                .size(FONT_SM)
                .style(styles::text_tertiary),
        ]
        .spacing(SPACE_SM)
        .into()
    }
}

/// Etiket + `−` / değer / `+` satırı — vites ve gönderim aralığının ortak gövdesi.
///
/// İki satır ayrı ayrı yazıldığında üç şey kaymıştı: değer puntosu (14 vs 12),
/// font ailesi ve değer kutusunun genişliği. Üçü de burada tek yerde: değer
/// daima `numeric` (sabit genişlikli, `FONT_SM`) ve kutu daima `VALUE_WIDTH`.
/// Kutuya ayrıca `CONTROL_HEIGHT` veriliyor — böylece metnin dikey ekseni
/// yanındaki butonların kutusuyla birebir aynı, punto ne olursa olsun.
///
/// Metin rengi dışarıdan geliyor: değerin pasif olup olmadığını (aralık motor
/// çalışırken kilitli) yalnızca çağıran biliyor.
fn stepper_row<'a>(
    label: &'a str,
    value: String,
    style: fn(&Theme) -> text::Style,
    previous: Option<Message>,
    next: Option<Message>,
) -> Element<'a, Message> {
    row![
        text(label).size(FONT_SM).style(styles::text_secondary),
        Space::new().width(Fill),
        icon_button("−", previous),
        numeric(value)
            .width(VALUE_WIDTH)
            .height(CONTROL_HEIGHT)
            .center()
            .style(style),
        icon_button("+", next),
    ]
    .spacing(SPACE_SM)
    .align_y(Alignment::Center)
    .into()
}

/// Vites seçici — Slint'teki `SpinBox { minimum: 1; maximum: 3 }` karşılığı.
///
/// Değer nötr tonda: vites bir uyarı değil, sıradan bir ayar. Eskiden
/// `text_warning` (turuncu/amber) kullanılıyordu ve yanındaki gerçek uyarı
/// göstergeleriyle aynı dili konuşuyordu.
fn gear_row(app: &App) -> Element<'_, Message> {
    stepper_row(
        "Vites:",
        format!("V{}", app.gear),
        styles::text_primary,
        (app.gear > 1).then(|| Message::GearSelected(app.gear - 1)),
        (app.gear < 3).then(|| Message::GearSelected(app.gear + 1)),
    )
}

/// Gönderim aralığı — motor çalışırken değiştirilemez (Slint ile aynı kural).
///
/// Açılır liste yerine adım düğmeleri: hem vites satırıyla aynı dili konuşuyor
/// hem de `scrollable` içinde açılır liste bırakmıyor (bkz. `view::sidebar`).
fn interval_row(app: &App) -> Element<'_, Message> {
    let index = INTERVALS_MS
        .iter()
        .position(|value| *value == app.send_interval_ms)
        .unwrap_or(0);

    let editable = !app.motor_running;
    let previous =
        (editable && index > 0).then(|| Message::IntervalSelected(INTERVALS_MS[index - 1]));
    let next = (editable && index + 1 < INTERVALS_MS.len())
        .then(|| Message::IntervalSelected(INTERVALS_MS[index + 1]));

    stepper_row(
        "Gönd. Aralığı:",
        format!("{} ms", app.send_interval_ms),
        if editable {
            styles::text_primary
        } else {
            styles::text_tertiary
        },
        previous,
        next,
    )
}

/// Aç/kapa butonu — Işık, Fren, Sol/Sağ Ters dördü aynı kalıptan.
///
/// Yükseklik açıkça veriliyor: `button` yalnızca dolgusu kadar büyür, bu dört
/// buton `height` almadığı için ~25.6 px kalıyordu ve aynı panelde 36 px'lik
/// kontrollerin yanında yamalı duruyordu.
/// `message` `None` ise buton pasif çizilir.
fn toggle_button<'a>(label: &'a str, on: bool, message: Option<Message>) -> Element<'a, Message> {
    button(text(label).size(FONT_SM).center())
        .width(Fill)
        .height(CONTROL_HEIGHT)
        .style(styles::toggle(on))
        .on_press_maybe(message)
        .into()
}

/// Işık ve fren.
///
/// İkisi de robota paket gönderiyor (`light_packet` / `brake_packet`), yani
/// bağlantı yokken basıldığında paket hiçbir yere gitmiyor ama arayüz durumu
/// "açık"a çeviriyordu: sonra bağlanınca UI ışığı yanıyor sanıyor, robotta
/// kapalı. Bu yüzden `connected` kapısı — motor düğmesindeki kuralın aynısı.
///
/// Sol/Sağ Ters bunun dışında kalıyor: onlar paket göndermiyor, yalnızca yerel
/// hız işaretini çeviriyor (`Backend::reverse_left/right` atomikleri), yani
/// bağlantı öncesi ayarlanabilir olmaları doğru.
fn peripheral_row(app: &App) -> Element<'_, Message> {
    row![
        toggle_button(
            if app.light_on {
                "💡 IŞIK ✓"
            } else {
                "💡 IŞIK"
            },
            app.light_on,
            app.connected.then_some(Message::LightToggled),
        ),
        toggle_button(
            if app.brake_on {
                "🛑 FREN ✓"
            } else {
                "🛑 FREN"
            },
            app.brake_on,
            app.connected.then_some(Message::BrakeToggled),
        ),
    ]
    .spacing(SPACE_SM)
    .into()
}

/// Motor ters bağlantı (sırt sırta montaj) — iki bayrak karşılıklı dışlar.
fn reverse_row(app: &App) -> Element<'_, Message> {
    row![
        toggle_button(
            if app.reverse_left {
                "↺ Sol Ters ✓"
            } else {
                "↺ Sol Ters"
            },
            app.reverse_left,
            Some(Message::ReverseLeftToggled),
        ),
        toggle_button(
            if app.reverse_right {
                "↻ Sağ Ters ✓"
            } else {
                "↻ Sağ Ters"
            },
            app.reverse_right,
            Some(Message::ReverseRightToggled),
        ),
    ]
    .spacing(SPACE_SM)
    .into()
}
