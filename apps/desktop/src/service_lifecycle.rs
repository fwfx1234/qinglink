//! 桌面端租约与服务准备。
//!
//! 只要桌面端在运行，它就持有服务上的一份租约，服务因此能区分"用户会话在线"和
//! "机器前没人"。准备过程与租约都不得阻塞启动。

#![cfg(target_os = "windows")]

use anyhow::{Result, ensure};
use qinglink_protocol::service_ipc::SERVICE_IPC_VERSION;
use serde::{Deserialize, Serialize};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;

pub(crate) const PIPE_NAME: &str = r"\\.\pipe\QingLinkControlV2";

/// How often a registered lease is refreshed.
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(2);

#[derive(Debug, Serialize)]
struct PipeRequest {
    version: u32,
    request_id: String,
    command: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    parent_pid: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct PipeResponse {
    /// Protocol version the service answered with, when it echoed one.
    #[serde(default)]
    pub(crate) version: u32,
    pub(crate) ok: bool,
    pub(crate) code: String,
    pub(crate) message: String,
    #[serde(default)]
    pub(crate) request_id: Option<String>,
}

/// Probe the management protocol once, as a diagnostic.
///
/// The lease uses the lifecycle commands; this goes through the management
/// handshake instead, so a success also proves the installed service speaks the
/// protocol the desktop actually needs. Nothing is mutated: `capabilities` is
/// the read-only negotiation the desktop performs on every connect.
pub(crate) fn probe_management_pipe() -> Result<PipeResponse> {
    let request = PipeRequest {
        version: 2,
        request_id: format!(
            "check-service-{}-{}",
            std::process::id(),
            time::OffsetDateTime::now_utc().unix_timestamp_nanos()
        ),
        command: "capabilities",
        parent_pid: None,
    };
    let response: PipeResponse = qinglink_windows::ipc::request(
        PIPE_NAME,
        &request,
        Duration::from_secs(3),
        &AtomicBool::new(false),
    )?;
    // An old service answers with its own protocol version. Reporting that as
    // "reachable" would hide the more useful fact that it needs an upgrade.
    ensure!(
        response.version == SERVICE_IPC_VERSION,
        "服务协议版本不匹配（服务 {} / 桌面端 {}），请升级或修复服务",
        response.version,
        SERVICE_IPC_VERSION
    );
    Ok(response)
}

pub struct ServiceLifecycleManager {
    stop_signal: Arc<AtomicBool>,
    registered: Arc<AtomicBool>,
    worker: Option<std::thread::JoinHandle<()>>,
}

impl ServiceLifecycleManager {
    pub fn new() -> Self {
        Self {
            stop_signal: Arc::new(AtomicBool::new(false)),
            registered: Arc::new(AtomicBool::new(false)),
            worker: None,
        }
    }

    pub fn start(&mut self, _silent: bool) {
        let stop = Arc::clone(&self.stop_signal);
        let registered = Arc::clone(&self.registered);
        self.worker = Some(std::thread::spawn(move || {
            if let Err(error) = Self::prepare_service() {
                // Not fatal: the management interface still runs and reports the
                // problem, which is more useful than refusing to start.
                tracing::warn!(error = %format!("{error:#}"), "服务准备失败");
            }
            let mut failures: u32 = 0;
            // The lease is retried twice a second for the life of the process,
            // so a persistent cause must be reported once: repeating it buries
            // the rest of the log without adding information.
            let mut reported: Option<String> = None;
            let mut environment_reported = false;
            let mut protocol_reported = false;
            while !stop.load(Ordering::Acquire) {
                let command = if registered.load(Ordering::Acquire) {
                    "heartbeat"
                } else {
                    "register_parent"
                };
                match Self::send(command, &stop) {
                    Ok(()) => {
                        registered.store(true, Ordering::Release);
                        failures = 0;
                        reported = None;
                    }
                    Err(error) => {
                        registered.store(false, Ordering::Release);
                        failures = failures.saturating_add(1);
                        // The full chain matters: the outer context is always
                        // "cannot connect to local pipe", while the innermost
                        // cause carries the OS error that identifies whether the
                        // pipe is missing, busy, or denied by the environment.
                        let chain = format!("{error:#}");
                        if !stop.load(Ordering::Acquire) && reported.as_deref() != Some(&chain) {
                            tracing::warn!(error = %chain, command, "服务租约通信失败，将重新注册");
                            reported = Some(chain.clone());
                        }
                        if !environment_reported && is_environment_restriction(&chain) {
                            environment_reported = true;
                            Self::report_environment_restriction();
                        }
                        if !protocol_reported && is_protocol_mismatch(&chain) {
                            protocol_reported = true;
                            Self::report_protocol_mismatch();
                        }
                    }
                }
                let delay = if failures == 0 {
                    HEARTBEAT_INTERVAL
                } else {
                    Duration::from_secs((1 << failures.min(4)).min(15))
                };
                if !Self::sleep_interruptible(delay, &stop) {
                    break;
                }
            }
        }));
    }

    /// Explain a refused pipe once, in the terms the user can act on.
    ///
    /// The service being installed and running while the pipe is refused is the
    /// signature of an execution environment that forbids the connection (a
    /// restricted sandbox, or a process below the integrity level the pipe
    /// grants). Reporting only the transport error in that case sends people
    /// looking for a broken service that is in fact healthy.
    fn report_environment_restriction() {
        let state = super::input_install::inspect();
        if state.service_installed && state.service_running {
            tracing::warn!(
                "QingLinkService 正在运行，但当前执行环境拒绝访问其命名管道。\
                 服务相关功能（设备、应用目录、配对）在此会话中不可用；\
                 管理界面与日志仍可正常使用。请在普通用户会话或不受限的终端中启动桌面端。"
            );
        } else {
            tracing::warn!(
                installed = state.service_installed,
                running = state.service_running,
                detail = %state.detail,
                "无法连接 QingLinkService，服务相关功能不可用"
            );
        }
    }

    /// Explain a protocol mismatch once, in the terms the user can act on.
    ///
    /// A running service that answers without echoing the correlated envelope is
    /// the signature of a service binary older than the desktop's management
    /// protocol. The interface reports "服务未连接" for it, so the log has to
    /// carry the real reason.
    fn report_protocol_mismatch() {
        let state = super::input_install::inspect();
        tracing::warn!(
            installed = state.service_installed,
            running = state.service_running,
            binary_matches = state.service_binary_matches,
            "QingLinkService 正在运行，但服务协议与桌面端不匹配，管理通道不可用；\
             请在设置中点击「修复服务（升级）」重新安装当前版本的服务"
        );
    }

    fn prepare_service() -> Result<()> {
        let state = super::input_install::inspect();
        ensure!(
            state.service_installed,
            "Windows 服务尚未安装，请在设置中安装服务"
        );
        ensure!(
            state.service_binary_matches,
            "服务版本不匹配，请在设置中修复服务"
        );
        if !state.service_running {
            // Login startup must never raise UAC. The service ACL grants the
            // interactive user SERVICE_START, and IPC remains available while
            // the network is paused or cannot bind.
            super::input_install::start_service_direct()?;
        }
        Ok(())
    }

    fn sleep_interruptible(total: Duration, stop: &AtomicBool) -> bool {
        let mut remaining = total;
        while !remaining.is_zero() && !stop.load(Ordering::Acquire) {
            let step = remaining.min(Duration::from_millis(50));
            std::thread::sleep(step);
            remaining = remaining.saturating_sub(step);
        }
        !stop.load(Ordering::Acquire)
    }

    fn send(command: &'static str, stop: &AtomicBool) -> Result<()> {
        let request = PipeRequest {
            version: 2,
            request_id: format!(
                "desktop-{}-{}",
                std::process::id(),
                time::OffsetDateTime::now_utc().unix_timestamp_nanos()
            ),
            command,
            parent_pid: (command == "register_parent").then(std::process::id),
        };
        let response: PipeResponse =
            qinglink_windows::ipc::request(PIPE_NAME, &request, Duration::from_secs(3), stop)?;
        // Version first: a service predating the correlated envelope answers
        // without `request_id` (and with its own protocol version), and that is
        // an upgrade problem rather than a corrupted reply.
        ensure!(
            response.version == SERVICE_IPC_VERSION,
            "服务协议版本不匹配（服务 {} / 桌面端 {}），请在设置中点击「修复服务（升级）」",
            response.version,
            SERVICE_IPC_VERSION
        );
        ensure!(
            response.request_id.as_deref() == Some(request.request_id.as_str()),
            "服务响应 request_id 不匹配"
        );
        ensure!(
            response.ok,
            "服务拒绝请求 ({}): {}",
            response.code,
            response.message
        );
        Ok(())
    }

    /// Explicit quit asks the service to stop; restart only releases the desktop lease.
    pub fn shutdown(mut self, stop_service: bool) -> Result<()> {
        self.stop_signal.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            worker
                .join()
                .map_err(|_| anyhow::anyhow!("服务租约线程异常退出"))?;
        }
        if stop_service
            && self.registered.load(Ordering::Acquire)
            && let Err(error) = Self::send("shutdown", &AtomicBool::new(false))
        {
            tracing::warn!(%error, "停止服务请求失败；服务可能仍在运行");
        }
        Ok(())
    }
}

/// Whether a transport error chain points at the environment rather than the
/// service.
///
/// These are the failures a connection attempt can produce when the pipe itself
/// is unreachable for a reason the service cannot fix: access denied by policy,
/// no such file (the endpoint was never created in this session's namespace), or
/// the retry budget running out. Distinguishing them from a refusal matters
/// because "install the service" is the wrong advice for all three.
pub(crate) fn is_environment_restriction(chain: &str) -> bool {
    [
        "cannot connect to local pipe",
        "拒绝访问",
        "Access is denied",
    ]
    .iter()
    .any(|marker| chain.contains(marker))
}

/// Whether a transport/refusal error chain points at an outdated service.
///
/// The service being installed and running while the request is refused this way
/// is the signature of a binary that predates the desktop's protocol, which the
/// admin interface cannot repair by itself but can name.
pub(crate) fn is_protocol_mismatch(chain: &str) -> bool {
    ["服务协议版本不匹配", "request_id 不匹配", "UNKNOWN_COMMAND"]
        .iter()
        .any(|marker| chain.contains(marker))
}

impl Drop for ServiceLifecycleManager {
    fn drop(&mut self) {
        self.stop_signal.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take()
            && worker.join().is_err()
        {
            tracing::error!("服务租约线程异常退出");
        }
    }
}
