//! Kamera görünümü — Slint `components/camera_view.slint` karşılığı.

use iced::widget::{button, column, container, image, pick_list, row, stack, text};
use iced::{Alignment, Color, ContentFit, Element, Fill, Theme};

use crate::message::Message;
use crate::state::App;
use crate::styles;
use crate::theme::{
    Tokens, CONTROL_HEIGHT, FIELD_PADDING_X, FIELD_PADDING_Y, FONT_LG, FONT_SM, RADIUS_MD,
    RADIUS_SM, SPACE_LG, SPACE_MD, SPACE_SM, SPACE_XS,
};
use crate::view::widgets::{connection_warning, icon_button, numeric};

/// Yer tutucu ikon boyutu.
const PLACEHOLDER_ICON: f32 = 48.0;
/// Sağ üst rozet genişliği (Slint: 72px).
const BADGE_WIDTH: f32 = 72.0;
/// Rozet yüksekliği.
const BADGE_HEIGHT: f32 = 22.0;
/// Kontrol çubuğundaki metinli butonların genişliği.
const ACTION_WIDTH: f32 = 96.0;
/// Kare hızının "sağlıklı" sayıldığı alt eşik.
///
/// Bunun altında görüntü gözle fark edilir biçimde takılıyor, o yüzden rozet
/// yeşilden uyarı tonuna geçiyor.
const FPS_HEALTHY_MIN: u32 = 20;

type ButtonStyleFn = fn(&Theme, button::Status) -> button::Style;

/// Overlay rozetindeki metnin rengini token'lardan seçen fonksiyon.
///
/// Eskiden rozet `healthy: bool` alıyordu ve nesne sayısı için sabit `false`
/// geçiliyordu: sayı daima uyarı tonundaydı, yanındaki FPS rozeti ise aynı
/// tonu "kare hızı düştü" anlamında kullanıyordu. Aynı satırda tek renk iki
/// farklı şey söylüyordu. Rengi doğrudan çağıran seçiyor çünkü rozetin bir
/// sağlık göstergesi olup olmadığını yalnızca çağıran bilir.
type TintFn = fn(&Tokens) -> Color;

pub fn view(app: &App) -> Element<'_, Message> {
    let mut layers = stack![container(surface(app)).center(Fill).style(styles::page)];

    if app.camera_running {
        layers = layers.push(badges(app));
    }
    if !app.connected {
        layers = layers.push(connection_warning());
    }
    layers = layers.push(control_bar(app));

    layers.into()
}

/// Canlı görüntü ya da yer tutucu.
fn surface(app: &App) -> Element<'_, Message> {
    match (&app.camera_frame, app.camera_running) {
        (Some(handle), true) => image(handle.clone())
            .content_fit(ContentFit::Contain)
            .width(Fill)
            .height(Fill)
            .into(),
        _ => placeholder(app),
    }
}

/// Boş durum.
///
/// Birincil eylem buraya da konuyor: aksi hâlde "Kamera aktif değil" yazısı
/// ekranın ortasında dururken tek başlatma yolu sol alt köşede kalıyor.
fn placeholder(app: &App) -> Element<'_, Message> {
    let mut content = column![
        text("📷").size(PLACEHOLDER_ICON),
        text("Kamera aktif değil")
            .size(FONT_LG)
            .style(styles::text_tertiary),
    ]
    .align_x(Alignment::Center);

    if !app.camera_error.is_empty() {
        content = content.push(
            text(app.camera_error.as_str())
                .size(FONT_SM)
                .style(styles::text_error),
        );
    }

    if app.camera_starting {
        // Açılış sırasında başlatma düğmesi gösterilmez: ikinci basış aynı
        // kamerayı bir daha açmaya çalışır.
        content = content.push(
            text("Kamera açılıyor…")
                .size(FONT_SM)
                .style(styles::text_secondary),
        );
    } else if app.available_cameras.is_empty() {
        // Boş liste sessiz kalmamalı: eskiden "Kamera aktif değil" yazısının
        // altında hiçbir şey yoktu, kullanıcı neden başlatamadığını görmüyordu.
        content = content.push(
            text("Kamera bulunamadı — cihazı bağlayıp ↻ ile yenileyin")
                .size(FONT_SM)
                .style(styles::text_secondary),
        );
    } else if let Some(choice) = &app.selected_camera {
        content = content.push(
            text(choice.name.as_str())
                .size(FONT_SM)
                .style(styles::text_tertiary),
        );
        content = content.push(
            button(text("▶  Kamerayı Başlat").size(FONT_SM).center())
                .width(180.0)
                .height(CONTROL_HEIGHT)
                .style(styles::success)
                .on_press(Message::CameraStartPressed),
        );
    }

    // Tek spacing: satırlar arası boşluk kolonun son hâlinde geçerli olan
    // değerdir, kurulum sırasında ayrıca vermek ölü koddu.
    content.spacing(SPACE_LG).into()
}

/// Sağ üst: FPS ve tespit sayısı.
fn badges(app: &App) -> Element<'_, Message> {
    // Kare hızı gerçekten bir sağlık göstergesi: eşiğin altı uyarı.
    let fps_tint: TintFn = if app.camera_fps >= FPS_HEALTHY_MIN {
        |t| t.success
    } else {
        |t| t.warning
    };

    let mut badges =
        row![overlay_badge(format!("{} FPS", app.camera_fps), fps_tint)].spacing(SPACE_SM);

    if app.detection_enabled {
        // Nesne sayısı nötr: "3 nesne" ne iyi ne kötü haber. Çipin kendi metin
        // rengiyle yazılır, yoksa yanındaki FPS uyarısıyla karışır.
        badges = badges.push(overlay_badge(
            format!("{} nesne", app.detection_count),
            |t| t.on_overlay,
        ));
    }

    // Tespit hattı ölmüşse nedeni burada yazılı: düğmenin yanına sığmıyor ve
    // kilitli bir düğme tek başına *neden* kilitli olduğunu söylemiyor.
    let mut notices = column![badges].spacing(SPACE_SM).align_x(Alignment::End);
    if let Some(reason) = &app.detection_error {
        notices = notices.push(overlay_note(reason.as_str()));
    }

    container(notices)
        .align_right(Fill)
        .align_top(Fill)
        .padding(SPACE_MD)
        .into()
}

/// Genişliği içeriğine göre değişen overlay notu.
///
/// `overlay_badge` sabit `BADGE_WIDTH` kullanıyor — sayısal okumalar rakam
/// değişirken zıplamasın diye. Cümle uzunluğundaki bir metin oraya sığmaz.
fn overlay_note<'a>(message: &'a str) -> Element<'a, Message> {
    container(text(message).size(FONT_SM))
        .padding([SPACE_XS, SPACE_SM])
        .style(styles::overlay_chip(RADIUS_SM))
        .into()
}

/// Sürekli değişen sayılar sabit genişlikli fontla yazılır — rakam
/// değiştiğinde rozet içeriği yatayda zıplamaz.
fn overlay_badge<'a>(label: String, tint: TintFn) -> Element<'a, Message> {
    container(
        numeric(label)
            .center()
            .style(move |theme: &Theme| iced::widget::text::Style {
                color: Some(tint(&Tokens::for_theme(theme))),
            }),
    )
    .width(BADGE_WIDTH)
    .height(BADGE_HEIGHT)
    .style(styles::overlay_chip(RADIUS_SM))
    .into()
}

/// Sol alt kontrol çubuğu: kamera seçici · yenile · algıla · başlat/durdur.
///
/// Çubuk doğrudan video üzerinde duruyordu; parlak bir karede butonlar
/// okunmuyordu. Rozetlerle aynı koyu zemine alındı.
fn control_bar(app: &App) -> Element<'_, Message> {
    let mut controls = row![].spacing(SPACE_SM).align_y(Alignment::Center);

    if app.camera_running {
        // Model yüklenemediyse düğme kilitli ve etiketi bunu söylüyor;
        // gerekçe rozetler kolonunda yazılı (bkz. `overlay_note`).
        let unavailable = app.detection_error.is_some();
        let label = match (unavailable, app.detection_enabled) {
            (true, _) => "🔍 Yok",
            (false, true) => "🔍 Kapat",
            (false, false) => "🔍 Algıla",
        };

        controls = controls.push(
            button(text(label).size(FONT_SM).center())
                .width(ACTION_WIDTH)
                .height(CONTROL_HEIGHT)
                .style(styles::toggle(app.detection_enabled))
                .on_press_maybe((!unavailable).then_some(Message::DetectionToggled)),
        );
    } else if app.available_cameras.is_empty() {
        // Boş `pick_list` yerine gerekçe: açılınca boş bir menü gösteren
        // seçici, listenin gerçekten boş olduğunu söylemiyor.
        controls = controls.push(
            container(
                text("Kamera bulunamadı")
                    .size(FONT_SM)
                    .style(styles::text_secondary),
            )
            .height(CONTROL_HEIGHT)
            .center_y(CONTROL_HEIGHT),
        );
        controls = controls.push(icon_button("↻", Some(Message::CamerasRefreshRequested)));
    } else {
        controls = controls.push(
            pick_list(
                app.available_cameras.as_slice(),
                app.selected_camera.as_ref(),
                Message::CameraSelected,
            )
            .placeholder("Kamera seçin")
            .text_size(FONT_SM)
            .padding([FIELD_PADDING_Y, FIELD_PADDING_X])
            .style(styles::field)
            .width(240.0),
        );
        controls = controls.push(icon_button("↻", Some(Message::CamerasRefreshRequested)));
    }

    // Açılış sırasında ne başlat ne durdur: kamera henüz iki durumdan da
    // değil. Düğme kilitli ve ne beklendiğini yazıyor.
    let (label, message, style): (&str, Option<Message>, ButtonStyleFn) = if app.camera_starting {
        ("⏳ Açılıyor", None, styles::success)
    } else if app.camera_running {
        ("■ Durdur", Some(Message::CameraStopPressed), styles::danger)
    } else {
        (
            "▶ Başlat",
            app.selected_camera
                .as_ref()
                .map(|_| Message::CameraStartPressed),
            styles::success,
        )
    };

    controls = controls.push(
        button(text(label).size(FONT_SM).center())
            .width(ACTION_WIDTH)
            .height(CONTROL_HEIGHT)
            .style(style)
            .on_press_maybe(message),
    );

    container(
        container(controls)
            .padding(SPACE_SM)
            .style(styles::overlay_chip(RADIUS_MD)),
    )
    .align_bottom(Fill)
    .padding(SPACE_MD)
    .into()
}
