#![cfg(target_os = "windows")]

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

pub struct LifecycleMonitor {
    pub(super) should_stop: Arc<AtomicBool>,
    desktop_registered: Arc<AtomicBool>,
    parent_pid: Arc<AtomicU32>,
}

impl LifecycleMonitor {
    pub fn new(should_stop: Arc<AtomicBool>) -> Self {
        Self {
            should_stop,
            desktop_registered: Arc::new(AtomicBool::new(false)),
            parent_pid: Arc::new(AtomicU32::new(0)),
        }
    }

    pub fn register_parent(&self, pid: u32) {
        self.parent_pid.store(pid, Ordering::Release);
        self.desktop_registered.store(true, Ordering::Release);
        tracing::info!(parent_pid = pid, "已注册父进程 PID");
    }

    pub fn accepts_client_pid(&self, pid: u32) -> bool {
        self.desktop_registered.load(Ordering::Acquire)
            && self.parent_pid.load(Ordering::Acquire) == pid
    }

    pub fn start_monitors(&self) {
        // The service is independent from the interactive desktop process.
        // Parent registration is retained for authorizing an explicit
        // shutdown request, but a missing heartbeat must never stop the SCM
        // service (the desktop may be closed or crash at any time).
        let _ = &self.should_stop;
        tracing::debug!("desktop lease monitor disabled; service lifetime is managed by SCM");
    }
}
