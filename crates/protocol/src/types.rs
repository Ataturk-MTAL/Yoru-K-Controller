/// Robot'tan gelen yanıt türleri
#[derive(Debug, Clone)]
pub enum RobotResponse {
    Status(bool),
    Speed { gear: u8, left: i8, right: i8 },
    Light(bool),
    Brake(bool),
    Gps { lat: f32, lon: f32 },
    Unknown(Vec<u8>),
}

/// Komut baytları
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    SetStop = 0x00,
    SetSpeed = 0x01,
    SetLight = 0x02,
    SetBrake = 0x03,
    SetGpsEnable = 0x04, // 0x01 = GPS yayınını başlat, 0x00 = durdur
    SetStart = 0xFF,
    GetStatus = 0x10,
    GetSpeed = 0x11,
    GetLight = 0x12,
    GetBrake = 0x13,
    GetGps = 0x14,
}

/// Vites seçimi
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Gear {
    #[default]
    V1 = 1,
    V2 = 2,
    V3 = 3,
}

impl Gear {
    pub fn as_byte(self) -> u8 {
        self as u8
    }
}
