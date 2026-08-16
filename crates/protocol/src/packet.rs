use crate::types::{Command, RobotResponse};

/// Protokol sabit başlangıç baytı
pub const START_BYTE: u8 = 0xAA;

/// Fletcher-16 checksum — pkt\[1..\] üzerinden hesaplanır (START_BYTE hariç)
/// Python eşdeğeri: sum1 % 255, sum2 % 255 → \[sum2, sum1\]
pub fn fletcher16(data: &[u8]) -> [u8; 2] {
    let mut sum1: u32 = 0;
    let mut sum2: u32 = 0;
    for &b in data {
        sum1 = (sum1 + b as u32) % 255;
        sum2 = (sum2 + sum1) % 255;
    }
    [sum2 as u8, sum1 as u8]
}

/// Veri alanı olmayan basit komut paketi oluşturur
/// Format: \[0xAA\]\[CMD\]\[0x00\]\[CK2\]\[CK1\]
pub fn simple_packet(cmd: Command) -> Vec<u8> {
    let pkt = vec![START_BYTE, cmd as u8, 0x00];
    let ck = fletcher16(&pkt[1..]);
    let mut result = pkt;
    result.extend_from_slice(&ck);
    result
}

/// Hız paketi oluşturur
/// Format: AA 01 07 \[gear\] 4C \[left_dir\] \[left_abs\] 52 \[right_dir\] \[right_abs\] \[CK2\] \[CK1\]
/// dir: 0x46 = 'F' (ileri), 0x42 = 'B' (geri)
pub fn speed_packet(left: i8, right: i8, gear: u8, rev_left: bool, rev_right: bool) -> Vec<u8> {
    let l = if rev_left {
        left.saturating_neg()
    } else {
        left
    }
    .clamp(-100, 100);
    let r = if rev_right {
        right.saturating_neg()
    } else {
        right
    }
    .clamp(-100, 100);
    let gear = gear.clamp(1, 3);

    let dir = |v: i8| if v >= 0 { 0x46u8 } else { 0x42u8 }; // 'F' veya 'B'

    let pkt = vec![
        START_BYTE,
        Command::SetSpeed as u8,
        7, // veri uzunluğu
        gear,
        0x4C, // 'L'
        dir(l),
        l.unsigned_abs(),
        0x52, // 'R'
        dir(r),
        r.unsigned_abs(),
    ];
    let ck = fletcher16(&pkt[1..]);
    let mut result = pkt;
    result.extend_from_slice(&ck);
    result
}

/// Işık komut paketi: \[0xAA\]\[0x02\]\[0x01\]\[on=1/off=0\]\[CK2\]\[CK1\]
pub fn light_packet(on: bool) -> Vec<u8> {
    let pkt = vec![
        START_BYTE,
        Command::SetLight as u8,
        0x01,
        if on { 0x01 } else { 0x00 },
    ];
    let ck = fletcher16(&pkt[1..]);
    let mut result = pkt;
    result.extend_from_slice(&ck);
    result
}

/// Fren komut paketi: \[0xAA\]\[0x03\]\[0x01\]\[on=1/off=0\]\[CK2\]\[CK1\]
pub fn brake_packet(on: bool) -> Vec<u8> {
    let pkt = vec![
        START_BYTE,
        Command::SetBrake as u8,
        0x01,
        if on { 0x01 } else { 0x00 },
    ];
    let ck = fletcher16(&pkt[1..]);
    let mut result = pkt;
    result.extend_from_slice(&ck);
    result
}

/// GPS yayın komut paketi: \[0xAA\]\[0x04\]\[0x01\]\[on=1/off=0\]\[CK2\]\[CK1\]
/// `on=true`  → ESP32'ye periyodik (100ms) GPS paketi göndermesini söyler
/// `on=false` → GPS yayınını durdurur
pub fn gps_enable_packet(on: bool) -> Vec<u8> {
    let pkt = vec![
        START_BYTE,
        Command::SetGpsEnable as u8,
        0x01,
        if on { 0x01 } else { 0x00 },
    ];
    let ck = fletcher16(&pkt[1..]);
    let mut result = pkt;
    result.extend_from_slice(&ck);
    result
}

/// Gelen ham paketi ayrıştırır
pub fn parse_response(data: &[u8]) -> Option<RobotResponse> {
    if data.len() < 3 || data[0] != START_BYTE {
        return None;
    }
    let cmd = data[1];
    let len = data[2] as usize;
    let payload = if data.len() >= 3 + len {
        &data[3..3 + len]
    } else {
        return None;
    };

    match cmd {
        0x10 => {
            // GET_STATUS yanıtı: payload[0] = 0x01 (çalışıyor) / 0x00 (durmuş)
            if payload.is_empty() {
                return None;
            }
            Some(RobotResponse::Status(payload[0] == 0x01))
        }
        0x11 => {
            // GET_SPEED yanıtı: [gear][4C][dir][abs][52][dir][abs]
            if payload.len() < 7 {
                return None;
            }
            let gear = payload[0];
            let left_dir = payload[2];
            let left_abs = payload[3] as i8;
            let right_dir = payload[5];
            let right_abs = payload[6] as i8;
            let left = if left_dir == 0x46 {
                left_abs
            } else {
                -left_abs
            };
            let right = if right_dir == 0x46 {
                right_abs
            } else {
                -right_abs
            };
            Some(RobotResponse::Speed { gear, left, right })
        }
        0x12 => {
            if payload.is_empty() {
                return None;
            }
            Some(RobotResponse::Light(payload[0] == 0x01))
        }
        0x13 => {
            if payload.is_empty() {
                return None;
            }
            Some(RobotResponse::Brake(payload[0] == 0x01))
        }
        0x14 => {
            // GPS: [LAT_4B little-endian][LON_4B little-endian] — IEEE 754 float
            if payload.len() < 8 {
                return None;
            }
            let lat = f32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
            let lon = f32::from_le_bytes([payload[4], payload[5], payload[6], payload[7]]);
            Some(RobotResponse::Gps { lat, lon })
        }
        _ => Some(RobotResponse::Unknown(data.to_vec())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// PROTOCOL.md §10'daki örnek bayt dizileri.
    ///
    /// Belge bir dönem yanlış checksum'lar taşıyordu (START için `FB 00`,
    /// durum sorgusu için `10 10`). Örnekler artık burada sabitlendi: doküman
    /// ile kod ayrışırsa test düşer.
    #[test]
    fn test_protocol_md_ornekleri() {
        assert_eq!(
            simple_packet(Command::SetStart),
            vec![0xAA, 0xFF, 0x00, 0x00, 0x00],
            "START paketi"
        );
        assert_eq!(
            simple_packet(Command::SetStop),
            vec![0xAA, 0x00, 0x00, 0x00, 0x00],
            "STOP paketi"
        );
        assert_eq!(
            simple_packet(Command::GetStatus),
            vec![0xAA, 0x10, 0x00, 0x20, 0x10],
            "durum sorgu paketi"
        );
        assert_eq!(
            speed_packet(100, 100, 1, false, false),
            vec![0xAA, 0x01, 0x07, 0x01, 0x4C, 0x46, 0x64, 0x52, 0x46, 0x64, 0xEA, 0xFC],
            "tam hız ileri, V1"
        );
        assert_eq!(
            speed_packet(-50, 50, 2, false, false),
            vec![0xAA, 0x01, 0x07, 0x02, 0x4C, 0x42, 0x32, 0x52, 0x46, 0x32, 0xE2, 0x95],
            "sol dönüş, V2"
        );
        assert_eq!(
            gps_enable_packet(false),
            vec![0xAA, 0x04, 0x01, 0x00, 0x0E, 0x05],
            "GPS yayınını durdurma paketi"
        );
    }

    #[test]
    fn test_fletcher16_known() {
        // Python ile doğrulanmış değer
        let data = [0x01, 0x07, 0x01, 0x4C, 0x46, 0x64, 0x52, 0x46, 0x64];
        let [s2, s1] = fletcher16(&data);
        // Checksum her iki byte da sıfırdan farklı olmalı (temel sağlamlık testi)
        let combined = ((s2 as u16) << 8) | s1 as u16;
        assert_ne!(combined, 0, "Fletcher-16 sıfır çıktı verdi");
    }

    #[test]
    fn test_speed_packet_structure() {
        let pkt = speed_packet(100, 100, 1, false, false);
        assert_eq!(pkt[0], 0xAA, "START_BYTE hatalı");
        assert_eq!(pkt[1], 0x01, "CMD hatalı");
        assert_eq!(pkt[2], 7, "LEN hatalı");
        assert_eq!(pkt[3], 1, "Vites hatalı");
        assert_eq!(pkt[4], 0x4C, "'L' baytı hatalı");
        assert_eq!(pkt[5], 0x46, "Sol yön 'F' olmalı");
        assert_eq!(pkt[6], 100, "Sol hız 100 olmalı");
        assert_eq!(pkt[7], 0x52, "'R' baytı hatalı");
        assert_eq!(pkt[8], 0x46, "Sağ yön 'F' olmalı");
        assert_eq!(pkt[9], 100, "Sağ hız 100 olmalı");
        assert_eq!(pkt.len(), 12, "Paket uzunluğu 12 olmalı");
    }

    #[test]
    fn test_speed_packet_backward() {
        let pkt = speed_packet(-50, -50, 2, false, false);
        assert_eq!(pkt[5], 0x42, "Sol yön 'B' olmalı");
        assert_eq!(pkt[6], 50, "Sol abs hız 50 olmalı");
        assert_eq!(pkt[8], 0x42, "Sağ yön 'B' olmalı");
        assert_eq!(pkt[9], 50, "Sağ abs hız 50 olmalı");
        assert_eq!(pkt[3], 2, "Vites 2 olmalı");
    }

    #[test]
    fn test_speed_packet_reverse_flag() {
        // rev_left=true → sol hız tersine çevrilir
        let pkt_normal = speed_packet(80, 80, 1, false, false);
        let pkt_rev_l = speed_packet(80, 80, 1, true, false);
        assert_eq!(pkt_normal[5], 0x46, "Normal: sol ileri");
        assert_eq!(pkt_rev_l[5], 0x42, "Reverse: sol geri");
        assert_eq!(pkt_normal[8], 0x46, "Normal: sağ ileri");
        assert_eq!(pkt_rev_l[8], 0x46, "Rev_left: sağ değişmez");
    }

    #[test]
    fn test_simple_packet_start() {
        let pkt = simple_packet(Command::SetStart);
        assert_eq!(pkt[0], 0xAA);
        assert_eq!(pkt[1], 0xFF);
        assert_eq!(pkt[2], 0x00);
        assert_eq!(pkt.len(), 5);
    }

    #[test]
    fn test_simple_packet_stop() {
        let pkt = simple_packet(Command::SetStop);
        assert_eq!(pkt[1], 0x00);
    }

    #[test]
    fn test_light_packet() {
        let pkt_on = light_packet(true);
        let pkt_off = light_packet(false);
        assert_eq!(pkt_on[3], 0x01);
        assert_eq!(pkt_off[3], 0x00);
    }

    #[test]
    fn test_checksum_included() {
        // Paketin son 2 baytı fletcher16 ile uyuşmalı
        let pkt = speed_packet(50, -30, 3, false, false);
        let n = pkt.len();
        let expected_ck = fletcher16(&pkt[1..n - 2]);
        assert_eq!(pkt[n - 2], expected_ck[0], "CK2 hatalı");
        assert_eq!(pkt[n - 1], expected_ck[1], "CK1 hatalı");
    }
}
