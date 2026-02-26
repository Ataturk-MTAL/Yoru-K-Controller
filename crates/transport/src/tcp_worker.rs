use std::sync::mpsc;
use std::time::Duration;

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::mpsc as tokio_mpsc,
};

use crate::framing::{try_parse_packet, packet_to_event, RobotEvent};

/// TCP bağlantı timeout süresi
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

/// TCP worker başlatır (tokio async)
/// Döndürür: paket gönderme kanalı (tokio mpsc)
pub fn spawn(
    host: String,
    port: u16,
    event_tx: mpsc::Sender<RobotEvent>,
) -> tokio_mpsc::Sender<Vec<u8>> {
    let (pkt_tx, mut pkt_rx) = tokio_mpsc::channel::<Vec<u8>>(64);

    tokio::spawn(async move {
        let addr = format!("{host}:{port}");
        let connect_result = tokio::time::timeout(
            CONNECT_TIMEOUT,
            TcpStream::connect(&addr),
        ).await;

        match connect_result {
            Ok(Ok(stream)) => {
                // Nagle algoritmasını devre dışı bırak — küçük paketler hemen gönderilsin
                let _ = stream.set_nodelay(true);
                let _ = event_tx.send(RobotEvent::Connected(addr.clone()));
                let (mut rd, mut wr) = stream.into_split();
                let mut rx_buf: Vec<u8> = Vec::new();
                let mut tmp = [0u8; 512];

                loop {
                    tokio::select! {
                        // TX: gönderilecek paket var mı?
                        result = pkt_rx.recv() => {
                            match result {
                                Some(pkt) => {
                                    if let Err(e) = wr.write_all(&pkt).await {
                                        let _ = event_tx.send(RobotEvent::Error(
                                            format!("TCP yazma hatası: {e}")
                                        ));
                                        let _ = event_tx.send(RobotEvent::Disconnected);
                                        return;
                                    }
                                    // Küçük paketlerin hemen gönderilmesi için flush
                                    if let Err(e) = wr.flush().await {
                                        let _ = event_tx.send(RobotEvent::Error(
                                            format!("TCP flush hatası: {e}")
                                        ));
                                        let _ = event_tx.send(RobotEvent::Disconnected);
                                        return;
                                    }
                                }
                                None => return, // kanal kapandı
                            }
                        }

                        // RX: gelen veri
                        result = rd.read(&mut tmp) => {
                            match result {
                                Ok(0) => {
                                    let _ = event_tx.send(RobotEvent::Disconnected);
                                    return;
                                }
                                Ok(n) => {
                                    rx_buf.extend_from_slice(&tmp[..n]);
                                    while let Some(raw) = try_parse_packet(&mut rx_buf) {
                                        if let Some(evt) = packet_to_event(raw) {
                                            let _ = event_tx.send(evt);
                                        }
                                    }
                                }
                                Err(e) => {
                                    let msg = friendly_tcp_error(&e.to_string());
                                    let _ = event_tx.send(RobotEvent::Error(msg));
                                    let _ = event_tx.send(RobotEvent::Disconnected);
                                    return;
                                }
                            }
                        }
                    }
                }
            }
            Ok(Err(e)) => {
                let msg = friendly_tcp_error(&e.to_string());
                let _ = event_tx.send(RobotEvent::Error(msg));
                let _ = event_tx.send(RobotEvent::Disconnected);
            }
            Err(_timeout) => {
                let _ = event_tx.send(RobotEvent::Error(
                    "Bağlantı zaman aşımına uğradı (5s). ESP32 açık ve WiFi'ye bağlı mı?".into()
                ));
                let _ = event_tx.send(RobotEvent::Disconnected);
            }
        }
    });

    pkt_tx
}

/// TCP hata mesajlarını kullanıcı dostu hale getirir
fn friendly_tcp_error(raw: &str) -> String {
    if raw.contains("Connection refused") {
        "Bağlantı reddedildi. ESP32 açık ve WiFi'ye bağlı mı?".into()
    } else if raw.contains("Network unreachable") || raw.contains("No route") {
        "Ağa erişilemiyor. ESP32 WiFi ağına bağlı olduğunuzdan emin olun.".into()
    } else if raw.contains("timed out") {
        "Bağlantı zaman aşımına uğradı.".into()
    } else if raw.contains("reset") || raw.contains("broken pipe") {
        "Bağlantı kesildi.".into()
    } else {
        format!("TCP hatası: {raw}")
    }
}
