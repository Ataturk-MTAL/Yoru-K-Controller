use std::sync::mpsc;

use crate::framing::RobotEvent;
use crate::{serial_worker, tcp_worker};

/// Bağlantı modu
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionMode {
    Serial,
    Tcp,
}

/// Aktif bağlantının gönderme kanalı
enum ActiveSender {
    None,
    Serial(mpsc::Sender<Vec<u8>>),
    Tcp(tokio::sync::mpsc::Sender<Vec<u8>>),
}

/// Birleşik bağlantı yöneticisi (Serial + TCP)
pub struct ConnectionManager {
    mode:       ConnectionMode,
    sender:     ActiveSender,
    event_tx:   mpsc::Sender<RobotEvent>,
    connected:  bool,
}

impl ConnectionManager {
    pub fn new(event_tx: mpsc::Sender<RobotEvent>) -> Self {
        Self {
            mode:      ConnectionMode::Serial,
            sender:    ActiveSender::None,
            event_tx,
            connected: false,
        }
    }

    /// Seri port bağlantısı kur
    pub fn connect_serial(&mut self, port: &str, baud: u32) {
        self.disconnect_inner();
        let (tx, _handle) = serial_worker::spawn(port, baud, self.event_tx.clone());
        self.sender    = ActiveSender::Serial(tx);
        self.mode      = ConnectionMode::Serial;
        self.connected = true;
    }

    /// TCP bağlantısı kur (ESP32 AP modu)
    /// Not: `connected` flag'i hemen true olmaz — async bağlantı kurulduktan sonra
    /// `RobotEvent::Connected` gelir, UI o zaman güncellenir.
    /// Ancak `sender` hemen kullanılabilir — paketler kuyruğa alınır.
    pub fn connect_tcp(&mut self, host: &str, port: u16) {
        self.disconnect_inner();
        let tx = tcp_worker::spawn(host.to_string(), port, self.event_tx.clone());
        self.sender    = ActiveSender::Tcp(tx);
        self.mode      = ConnectionMode::Tcp;
        // connected, RobotEvent::Connected geldiğinde set edilecek
    }

    /// Bağlantıyı kes
    pub fn disconnect(&mut self) {
        self.disconnect_inner();
        let _ = self.event_tx.send(RobotEvent::Disconnected);
    }

    /// Paket gönder (mevcut aktif transport üzerinden)
    pub fn send(&self, packet: Vec<u8>) {
        match &self.sender {
            ActiveSender::Serial(tx) => { let _ = tx.send(packet); }
            ActiveSender::Tcp(tx)   => { let _ = tx.try_send(packet); }
            ActiveSender::None      => {}
        }
    }

    pub fn is_connected(&self) -> bool { self.connected }
    pub fn mode(&self) -> ConnectionMode { self.mode }

    /// Mevcut seri portları listele
    pub fn list_serial_ports() -> Vec<String> {
        serialport::available_ports()
            .unwrap_or_default()
            .into_iter()
            .map(|p| p.port_name)
            .collect()
    }

    fn disconnect_inner(&mut self) {
        self.sender    = ActiveSender::None;
        self.connected = false;
    }
}
