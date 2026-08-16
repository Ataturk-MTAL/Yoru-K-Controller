use protocol::{START_BYTE, fletcher16, parse_response, RobotResponse};

/// Transport katmanından gelen olaylar
#[derive(Debug, Clone)]
pub enum RobotEvent {
    Connected(String),
    Disconnected,
    Error(String),
    Packet(RobotResponse),
}

/// Buffer'dan geçerli paket bul ve çıkar
/// Format: \[0xAA\]\[CMD\]\[LEN\]\[DATA...\]\[CK2\]\[CK1\]
/// total_len = 3 (header) + LEN (data) + 2 (checksum)
pub fn try_parse_packet(buf: &mut Vec<u8>) -> Option<Vec<u8>> {
    loop {
        // 0xAA başlangıç baytını bul
        let start_pos = buf.iter().position(|&b| b == START_BYTE)?;
        if start_pos > 0 {
            buf.drain(..start_pos);
        }

        // Minimum 3 bayt: AA + CMD + LEN
        if buf.len() < 3 {
            return None;
        }

        let data_len = buf[2] as usize;
        let total = 3 + data_len + 2; // header + data + 2 checksum byte

        if buf.len() < total {
            return None;
        }

        let pkt = buf[..total].to_vec();
        let expected = fletcher16(&pkt[1..total - 2]);
        let actual = [pkt[total - 2], pkt[total - 1]];

        if expected == actual {
            buf.drain(..total);
            return Some(pkt);
        }

        // Checksum tutmadı: bu 0xAA gerçek bir paket başlangıcı olmayabilir —
        // gürültü baytı olabilir, o hâlde LEN alanı da çöp okunmuştur. `total`
        // kadar atmak, hemen ardından gelen GEÇERLİ bir paketi de yutar.
        // Yalnızca ilk bayt atılır, bir sonraki 0xAA'dan devam edilir.
        buf.drain(..1);
    }
}

/// Ham paket baytlarını RobotEvent'e çevirir
pub fn packet_to_event(raw: Vec<u8>) -> Option<RobotEvent> {
    parse_response(&raw).map(RobotEvent::Packet)
}

#[cfg(test)]
mod tests {
    use super::*;
    use protocol::{simple_packet, speed_packet, Command};

    #[test]
    fn test_parse_simple_packet() {
        let original = simple_packet(Command::SetStart);
        let mut buf = original.clone();
        let parsed = try_parse_packet(&mut buf);
        assert!(parsed.is_some(), "Paket ayrıştırılamadı");
        assert_eq!(parsed.unwrap(), original);
        assert!(buf.is_empty(), "Buffer temizlenmedi");
    }

    #[test]
    fn test_parse_speed_packet() {
        let original = speed_packet(75, -40, 2, false, false);
        let mut buf = original.clone();
        let parsed = try_parse_packet(&mut buf);
        assert_eq!(parsed.unwrap(), original);
    }

    #[test]
    fn test_skip_garbage() {
        let mut buf = vec![0x11, 0x22, 0x33]; // çöp baytlar
        let pkt = simple_packet(Command::SetStop);
        buf.extend_from_slice(&pkt);
        let parsed = try_parse_packet(&mut buf);
        assert_eq!(parsed.unwrap(), pkt);
    }

    #[test]
    fn test_partial_packet() {
        let pkt = simple_packet(Command::SetStart);
        let mut buf = pkt[..3].to_vec(); // yarım paket
        let parsed = try_parse_packet(&mut buf);
        assert!(parsed.is_none(), "Yarım paket None dönmeli");
    }

    #[test]
    fn test_sahte_baslangic_gecerli_paketi_yutmaz() {
        // Gürültü olarak gelen bir 0xAA, LEN alanını da çöp okutur: burada
        // sahte çerçeve LEN=5 diyor, yani 10 bayt kaplıyormuş gibi görünüyor
        // ve hemen ardındaki gerçek paketin üstüne biniyor.
        //
        // Checksum tutmayınca tüm sahte çerçeve atılırsa gerçek paket de
        // silinir. Doğru davranış tek bayt atıp bir sonraki 0xAA'dan devam.
        let real = simple_packet(Command::SetStart);
        let mut buf = vec![START_BYTE, 0x99, 0x05];
        buf.extend_from_slice(&real);
        buf.extend_from_slice(&[0x00, 0x00]); // sahte çerçeveyi tamamlayan gürültü

        let parsed = try_parse_packet(&mut buf);
        assert_eq!(
            parsed.as_deref(),
            Some(real.as_slice()),
            "sahte 0xAA sonrasındaki geçerli paket yutuldu"
        );
    }

    #[test]
    fn test_bad_checksum() {
        let mut pkt = simple_packet(Command::SetStart);
        let n = pkt.len();
        pkt[n - 1] ^= 0xFF; // checksum boz
        let mut buf = pkt;
        let parsed = try_parse_packet(&mut buf);
        assert!(parsed.is_none(), "Hatalı checksum None dönmeli");
    }
}
