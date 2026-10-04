//! 局域网设备发现：在后台线程应答移动端的 UDP 广播探测。
//!
//! 应答包含本机 ID、显示名称、协议端口与可选 MAC，供手机端在无配置时找到 PC。

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

/// 返回设备对外提供协议服务的默认端口。
pub fn default_port() -> u16 {
    DEFAULT_PORT
}

/// 监听发现端口并应答广播探测的 UDP 响应器，析构时停止后台线程。
#[derive(Debug)]
pub struct UdpDiscoveryResponder {
    stop: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}

impl UdpDiscoveryResponder {
    /// 绑定发现端口并启动应答线程，返回析构时能停止它的句柄。
    ///
    /// # Errors
    ///
    /// 当发现端口 `0.0.0.0:37777` 无法绑定（端口被占用或权限不足），
    /// 或套接字的读超时无法设置时返回错误。
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
