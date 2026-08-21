//! Anahtarlı canvas önbelleği.
//!
//! iced MVU'da `view` her mesaj turundan sonra koşar ve `canvas::Program`
//! nesneleri o turda yeniden kurulur. Bu yüzden `canvas::Cache` Program'ın
//! içinde yaşayamaz — orada tutulsa her karede yeni bir boş önbellek doğar ve
//! geometri yine sıfırdan tesselate edilirdi. Önbellek `App` state'inde durur,
//! Program'a referansla girer.
//!
//! Geçersiz kılma tek noktada: çizimi belirleyen tüm alanlar bir **anahtara**
//! toplanır, anahtar değişince önbellek düşer. Alternatif — `update` içindeki
//! ilgili her kolda `clear()` çağırmak — yarın eklenen bir mesajda unutulur ve
//! bayat kare çizer; burada unutulacak bir çağrı yok, yalnızca anahtara
//! eklenecek bir alan var.

use std::cell::Cell;

use iced::widget::canvas;

/// Önbellek + onu geçerli kılan anahtar.
///
/// `K` kopyalanabilir ve karşılaştırılabilir olmalı; pratikte çizimi belirleyen
/// alanların düz bir demeti (float'lar `to_bits()` ile, çünkü `f32: !Eq`).
pub struct Keyed<K: Copy + PartialEq> {
    cache: canvas::Cache,
    /// `None` = hiç çizilmedi.
    key: Cell<Option<K>>,
}

impl<K: Copy + PartialEq> Keyed<K> {
    pub fn new() -> Self {
        Self {
            cache: canvas::Cache::new(),
            key: Cell::new(None),
        }
    }

    /// Anahtar değiştiyse önbelleği düşürür, sonra önbelleği ödünç verir.
    ///
    /// `&self` yeterli: `canvas::Cache::clear` içeride `RefCell` kullanıyor,
    /// yani bu `view(&App)` içinden çağrılabilir.
    pub fn get(&self, key: K) -> &canvas::Cache {
        if self.key.get() != Some(key) {
            self.cache.clear();
            self.key.set(Some(key));
        }
        &self.cache
    }
}

impl<K: Copy + PartialEq> Default for Keyed<K> {
    fn default() -> Self {
        Self::new()
    }
}
