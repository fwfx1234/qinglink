use std::{
    net::{SocketAddr, UdpSocket},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

use anyhow::Result;
use qinglink_protocol::{DEFAULT_PORT, DISCOVERY_MAGIC, DISCOVERY_PORT};
use uuid::Uuid;

pub fn default_port() -> u16 {
    DEFAULT_PORT
}

pub struct UdpDiscoveryResponder {
    stop: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}

impl UdpDiscoveryResponder {
    pub fn start(pc_id: Uuid, name: String, port: u16, mac: Option<String>) -> Result<Self> {
        let socket = UdpSocket::bind(("0.0.0.0", DISCOVERY_PORT))?;
        socket.set_read_timeout(Some(Duration::from_millis(500)))?;
        let stop = Arc::new(AtomicBool::new(false));
        let stop_thread = Arc::clone(&stop);
        let thread = thread::spawn(move || {
            let mut buffer = [0_u8; 512];
            while !stop_thread.load(Ordering::Relaxed) {
                let Ok((size, peer)) = socket.recv_from(&mut buffer) else {
                    continue;
                };
                if &buffer[..size] != DISCOVERY_MAGIC.as_bytes() {
                    continue;
                }
                let response = serde_json::json!({"id": pc_id, "name": name, "port": port, "version": 2, "mac": mac});
                let _ = socket.send_to(
                    response.to_string().as_bytes(),
                    SocketAddr::new(peer.ip(), peer.port()),
                );
            }
        });
        Ok(Self {
            stop,
            thread: Some(thread),
        })
    }
}

impl Drop for UdpDiscoveryResponder {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
