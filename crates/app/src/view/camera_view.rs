//! Kamera görünümü — Slint `components/camera_view.slint` karşılığı.

use iced::widget::{button, column, container, image, pick_list, row, stack, text};
use iced::{Alignment, Background, Border, ContentFit, Element, Fill, Theme};

use crate::message::Message;
use crate::state::App;
use crate::styles;
use crate::theme::{
    Tokens, CONTROL_HEIGHT, FIELD_PADDING_X, FIELD_PADDING_Y, FONT_LG, FONT_SM, RADIUS_MD,
    RADIUS_SM, SPACE_LG, SPACE_MD, SPACE_SM,
};
use crate::view::widgets::{icon_button, numeric};

/// Yer tutucu ikon boyutu.
const PLACEHOLDER_ICON: f32 = 48.0;
/// Sağ üst rozet genişliği (Slint: 72px).
const BADGE_WIDTH: f32 = 72.0;
/// Rozet yüksekliği.
const BADGE_HEIGHT: f32 = 22.0;
/// Kenar boşluğu — overlay'ler pencereye 8px mesafede durur.
const EDGE: f32 = 8.0;
/// Kontrol çubuğundaki metinli butonların genişliği.
const ACTION_WIDTH: f32 = 96.0;

type ButtonStyleFn = fn(&Theme, button::Status) -> button::Style;

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
    .spacing(SPACE_MD)
    .align_x(Alignment::Center);

    if !app.camera_error.is_empty() {
        content = content.push(
            text(app.camera_error.as_str())
                .size(FONT_SM)
                .style(styles::text_error),
        );
    }

    if let Some(choice) = &app.selected_camera {
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

    content.spacing(SPACE_LG).into()
}

/// Sağ üst: FPS ve tespit sayısı.
fn badges(app: &App) -> Element<'_, Message> {
    let mut badges = row![overlay_badge(
        format!("{} FPS", app.camera_fps),
        app.camera_fps >= 20,
    )]
    .spacing(SPACE_SM);

    if app.detection_enabled {
        badges = badges.push(overlay_badge(
            format!("{} nesne", app.detection_count),
            false,
        ));
    }

    container(badges)
        .align_right(Fill)
        .align_top(Fill)
        .padding(EDGE)
        .into()
}

/// Sürekli değişen sayılar sabit genişlikli fontla yazılır — rakam
/// değiştiğinde rozet içeriği yatayda zıplamaz.
fn overlay_badge<'a>(label: String, healthy: bool) -> Element<'a, Message> {
    container(numeric(label).center().style(move |theme: &Theme| {
        let t = Tokens::for_theme(theme);
        iced::widget::text::Style {
            color: Some(if healthy { t.success } else { t.warning }),
        }
    }))
    .width(BADGE_WIDTH)
    .height(BADGE_HEIGHT)
    .style(scrim_chip(RADIUS_SM))
    .into()
}

/// Sol alt kontrol çubuğu: kamera seçici · yenile · algıla · başlat/durdur.
///
/// Çubuk doğrudan video üzerinde duruyordu; parlak bir karede butonlar
/// okunmuyordu. Rozetlerle aynı koyu zemine alındı.
fn control_bar(app: &App) -> Element<'_, Message> {
    let mut controls = row![].spacing(SPACE_SM).align_y(Alignment::Center);

    if app.camera_running {
        controls = controls.push(
            button(
                text(if app.detection_enabled {
                    "🔍 Kapat"
                } else {
                    "🔍 Algıla"
                })
                .size(FONT_SM)
                .center(),
            )
            .width(ACTION_WIDTH)
            .height(CONTROL_HEIGHT)
            .style(styles::toggle(app.detection_enabled))
            .on_press(Message::DetectionToggled),
        );
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
            .width(240.0),
        );
        controls = controls.push(icon_button("↻", Some(Message::CamerasRefreshRequested)));
    }

    let (label, message, style): (&str, Message, ButtonStyleFn) = if app.camera_running {
        ("■ Durdur", Message::CameraStopPressed, styles::danger)
    } else {
        ("▶ Başlat", Message::CameraStartPressed, styles::success)
    };

    let enabled = app.camera_running || app.selected_camera.is_some();
    controls = controls.push(
        button(text(label).size(FONT_SM).center())
            .width(ACTION_WIDTH)
            .height(CONTROL_HEIGHT)
            .style(style)
            .on_press_maybe(enabled.then_some(message)),
    );

    container(
        container(controls)
            .padding(SPACE_SM)
            .style(scrim_chip(RADIUS_MD)),
    )
    .align_bottom(Fill)
    .padding(EDGE)
    .into()
}

/// Sol üst bağlantı uyarısı.
fn connection_warning<'a>() -> Element<'a, Message> {
    container(
        container(
            text("Yörü-K bağlı değil")
                .size(FONT_SM)
                .style(styles::text_error),
        )
        .padding([4.0, 8.0])
        .style(styles::error_badge),
    )
    .padding(EDGE)
    .into()
}

/// Video üzerindeki overlay'lerin ortak koyu zemini.
fn scrim_chip(radius: f32) -> impl Fn(&Theme) -> container::Style {
    move |theme: &Theme| container::Style {
        background: Some(Background::Color(Tokens::for_theme(theme).scrim)),
        border: Border {
            radius: radius.into(),
            ..Border::default()
        },
        ..container::Style::default()
    }
}
