//! Windows kaynak gömme.
//!
//! Slint sürümündeki `build.rs` hem `.slint` derlemesini hem ikon gömmeyi
//! yapıyordu. iced'de derlenecek bir UI dosyası yok; geriye yalnızca Windows
//! çalıştırılabilirine ikon gömme kısmı kalıyor.

fn main() {
    #[cfg(windows)]
    {
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon("assets/icon.ico");
        resource
            .compile()
            .expect("Windows kaynak dosyası gömülemedi");
    }
}
