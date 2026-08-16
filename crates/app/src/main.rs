//! Yörü-K Kontrolcü — iced arayüzü.
//!
//! Slint sürümüyle (`crates/app`) aynı protokol/transport/control katmanlarını
//! kullanır; değişen yalnızca sunum katmanıdır.

mod backend;
mod camera;
mod map;
mod message;
mod state;
mod styles;
mod theme;
mod update;
mod view;

use iced::keyboard::{self, key::Named, Key};
use iced::{event, mouse, window, Event, Font, Size, Subscription, Task};

use backend::Backend;
use camera::Camera;
use message::{Dir, Message, Modal, Tab};
use state::App;

/// Başlangıç pencere boyutu (Slint: `preferred-width/height`).
const WINDOW_SIZE: Size = Size::new(1280.0, 960.0);
/// En küçük pencere boyutu (Slint: `min-width/height`).
const MIN_WINDOW_SIZE: Size = Size::new(1024.0, 768.0);

fn main() -> iced::Result {
    #[cfg(debug_assertions)]
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                // Hedef, crate adıyla aynı olmalı: `app-iced` → `app` yeniden
                // adlandırmasından sonra eski hedef hiçbir satırı eşleştirmiyordu.
                .add_directive("app=debug".parse().expect("geçersiz log filtresi")),
        )
        .init();

    // TCP worker `tokio::spawn` kullanıyor; bu guard ana thread'e bir runtime
    // bağlamı verir. Guard `run()` boyunca canlı kalmalıdır.
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("tokio runtime başlatılamadı");
    let _runtime_guard = runtime.enter();

    iced::application(boot, update::update, view::view)
        .title("Yörü-K Kontrolcü")
        .theme(App::theme)
        .subscription(subscription)
        .window(window::Settings {
            size: WINDOW_SIZE,
            min_size: Some(MIN_WINDOW_SIZE),
            // Kapatma isteğini kendimiz karşılıyoruz: çıkmadan önce robota
            // STOP gitmeli, yoksa motor son hızında kalır.
            exit_on_close_request: false,
            ..window::Settings::default()
        })
        .font(include_bytes!("../assets/fonts/Saira-Regular.ttf").as_slice())
        .font(include_bytes!("../assets/fonts/Saira-Medium.ttf").as_slice())
        .font(include_bytes!("../assets/fonts/Saira-SemiBold.ttf").as_slice())
        .font(include_bytes!("../assets/fonts/SpaceMono-Regular.ttf").as_slice())
        .default_font(Font::with_name("Saira"))
        .run()
}

/// Boot — arka plan thread'lerini kurar, port ve kamera listelerini yükler.
fn boot() -> (App, Task<Message>) {
    let backend = Backend::start();
    let camera = Camera::start_pipeline();

    (
        App::new(backend, camera),
        Task::batch([
            Task::perform(backend::list_serial_ports(), Message::PortsLoaded),
            Task::perform(camera::list_cameras(), Message::CamerasLoaded),
        ]),
    )
}

fn subscription(app: &App) -> Subscription<Message> {
    let mut subscriptions = vec![
        backend::events().map(Message::Robot),
        camera::updates().map(Message::Camera),
        hotkeys(),
        window::close_requests().map(|_id| Message::CloseRequested),
    ];

    // Joystick sürüklenirken fare bırakması canvas'a ulaşmayabilir: imleç
    // başka bir widget'ın üstündeyse olayı o yutar ve bilek merkeze dönmez.
    // Sürükleme sürerken bırakmayı global olarak dinliyoruz.
    if app.joystick.dragging {
        subscriptions.push(pointer_release());
    }

    Subscription::batch(subscriptions)
}

/// Sol fare tuşunun bırakılması — hangi widget yutarsa yutsun yakalanır.
fn pointer_release() -> Subscription<Message> {
    event::listen_with(|event, _status, _window| match event {
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
            Some(Message::JoystickReleased)
        }
        _ => None,
    })
}

/// Klavye kısayolları — Slint `FocusScope` `key-pressed/released` karşılığı.
///
/// `listen_with` bir **fn pointer** ister; yakalayan closure kabul edilmez.
fn hotkeys() -> Subscription<Message> {
    event::listen_with(|event, status, _window| {
        // Odaktaki bir metin alanı tuşu yuttuysa robot komutu üretilmez —
        // IP adresi yazarken robotun hareket etmesini engeller.
        if status == event::Status::Captured {
            return None;
        }

        match event {
            Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) => on_key_down(key),
            Event::Keyboard(keyboard::Event::KeyReleased { key, .. }) => on_key_up(key),
            _ => None,
        }
    })
}

fn on_key_down(key: Key) -> Option<Message> {
    if let Some(dir) = direction(&key) {
        return Some(Message::DirPressed(dir));
    }

    match key {
        Key::Named(Named::Space) => Some(Message::EmergencyStop),
        Key::Named(Named::Escape) => Some(Message::ModalClosed),
        Key::Character(c) => match c.as_str() {
            "1" => Some(Message::GearSelected(1)),
            "2" => Some(Message::GearSelected(2)),
            "3" => Some(Message::GearSelected(3)),
            "l" | "L" => Some(Message::LightToggled),
            "b" | "B" => Some(Message::BrakeToggled),
            "c" | "C" => Some(Message::TabSelected(Tab::Camera)),
            "m" | "M" => Some(Message::TabSelected(Tab::Map)),
            "t" | "T" => Some(Message::ThemeToggled),
            "?" => Some(Message::ModalOpened(Modal::Shortcuts)),
            _ => None,
        },
        _ => None,
    }
}

fn on_key_up(key: Key) -> Option<Message> {
    direction(&key).map(Message::DirReleased)
}

/// WASD ve ok tuşları aynı yöne düşer.
fn direction(key: &Key) -> Option<Dir> {
    match key {
        Key::Named(Named::ArrowUp) => Some(Dir::Forward),
        Key::Named(Named::ArrowDown) => Some(Dir::Backward),
        Key::Named(Named::ArrowLeft) => Some(Dir::Left),
        Key::Named(Named::ArrowRight) => Some(Dir::Right),
        Key::Character(c) => match c.as_str() {
            "w" | "W" => Some(Dir::Forward),
            "s" | "S" => Some(Dir::Backward),
            "a" | "A" => Some(Dir::Left),
            "d" | "D" => Some(Dir::Right),
            _ => None,
        },
        _ => None,
    }
}
