use std::{
    io::{self, Read, Write},
    sync::mpsc,
    thread,
    time::Duration,
};

use crate::framing::{try_parse_packet, packet_to_event, RobotEvent};

/// Seri port worker başlatır
/// Döndürür: (paket gönderme kanalı, thread handle)
pub fn spawn(
    port_name: &str,
    baud_rate: u32,
    event_tx: mpsc::Sender<RobotEvent>,
) -> (mpsc::Sender<Vec<u8>>, thread::JoinHandle<()>) {
    let (pkt_tx, pkt_rx) = mpsc::channel::<Vec<u8>>();
    let port_name = port_name.to_string();

    let handle = thread::Builder::new()
        .name("serial-worker".into())
        .spawn(move || {
            match serialport::new(&port_name, baud_rate)
                .timeout(Duration::from_millis(10))
                .open()
            {
                Ok(mut port) => {
                    let _ = event_tx.send(RobotEvent::Connected(port_name.clone()));
                    let mut rx_buf: Vec<u8> = Vec::new();

                    loop {
                        // TX: bekleyen paketleri gönder
                        // pkt_rx kapatılınca (Sender drop) Disconnected → thread çıkar
                        let mut sender_closed = false;
                        loop {
                            match pkt_rx.try_recv() {
                                Ok(pkt) => {
                                    if let Err(e) = port.write_all(&pkt).and_then(|_| port.flush()) {
                                        let _ = event_tx.send(RobotEvent::Error(format!("Yazma hatası: {e}")));
                                        return;
                                    }
                                }
                                Err(mpsc::TryRecvError::Empty) => break,
                                Err(mpsc::TryRecvError::Disconnected) => {
                                    sender_closed = true;
                                    break;
                                }
                            }
                        }
                        if sender_closed {
                            // Bağlantı kesildi — port drop edilir, seri port serbest kalır
                            return;
                        }

                        // RX: gelen veriyi oku
                        let mut tmp = [0u8; 256];
                        match port.read(&mut tmp) {
                            Ok(0) => {}
                            Ok(n) => {
                                rx_buf.extend_from_slice(&tmp[..n]);
                                while let Some(raw) = try_parse_packet(&mut rx_buf) {
                                    if let Some(evt) = packet_to_event(raw) {
                                        let _ = event_tx.send(evt);
                                    }
                                }
                            }
                            Err(e) if e.kind() == io::ErrorKind::TimedOut => {
                                // Normal — 10ms timeout beklendi
                            }
                            Err(e) => {
                                let msg = friendly_serial_error(&e.to_string());
                                let _ = event_tx.send(RobotEvent::Error(msg));
                                let _ = event_tx.send(RobotEvent::Disconnected);
                                return;
                            }
                        }
                    }
                }
                Err(e) => {
                    let msg = friendly_serial_error(&e.to_string());
                    let _ = event_tx.send(RobotEvent::Error(msg));
                }
            }
        })
        .expect("serial-worker thread başlatılamadı");

    (pkt_tx, handle)
}

/// Seri port hata mesajlarını kullanıcı dostu hale getirir
fn friendly_serial_error(raw: &str) -> String {
    if raw.contains("Permission denied") {
        "Port erişim izni reddedildi. Portu başka uygulama kullanıyor olabilir.".into()
    } else if raw.contains("No such file") || raw.contains("not found") {
        "Port bulunamadı. Cihazın bağlı olduğundan emin olun.".into()
    } else if raw.contains("Device disconnected") || raw.contains("Broken pipe") {
        "Cihaz bağlantısı kesildi.".into()
    } else {
        format!("Seri port hatası: {raw}")
    }
}
