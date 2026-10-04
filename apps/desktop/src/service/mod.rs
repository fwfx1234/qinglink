//! 桌面端唯一的管理传输通道。
//!
//! 桌面进程不持有任何业务数据。管理界面读取或修改的一切都经已授权的本地命名管道
//! 送到持有 SQLite 目录的 `QingLinkService`。正因如此，在浏览器里做的重命名会立即
//! 对这台 PC 的每个移动端客户端生效。

mod client;

pub use client::{ServiceClient, ServiceEvent, ServiceLinkState, ServiceRuntime, ServiceStatus};

use std::time::Duration;

/// Pipe exposed by the service for authorized desktop peers.
pub const PIPE_NAME: &str = r"\\.\pipe\QingLinkControlV2";

/// Per-request budget for one pipe round trip.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(3);

/// How long a caller waits for the worker to answer before giving up.
///
/// Slightly longer than [`REQUEST_TIMEOUT`] so a slow-but-successful service
/// reply wins the race and is reported accurately.
pub const CALL_TIMEOUT: Duration = Duration::from_secs(5);

/// How often the service snapshot is refreshed.
const SNAPSHOT_INTERVAL: Duration = Duration::from_millis(500);

/// A failure that can be attributed to a specific cause.
///
/// The variants exist so the web layer can map them onto stable HTTP error
/// codes without parsing message text.
#[derive(Clone, Debug)]
pub enum ServiceError {
    /// The service is not installed, not running, or the pipe is broken.
    Unavailable(String),
    /// The installed service is too old for the requested command.
    Unsupported(String),
    /// The caller gave up waiting for the worker.
    Timeout,
    /// The service answered with a refusal.
    Rejected { code: String, message: String },
    /// The service refused the desktop identity: the desktop executable changed
    /// after the service was installed, so the pinned trust record no longer
    /// matches. Repaired by reinstalling (repairing) the service.
    Unauthorized(String),
    /// Anything else, including transport-level failures.
    Internal(String),
}

impl std::fmt::Display for ServiceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable(message)
            | Self::Unsupported(message)
            | Self::Unauthorized(message)
            | Self::Internal(message) => formatter.write_str(message),
            Self::Timeout => formatter.write_str("服务响应超时，请稍后重试"),
            Self::Rejected { message, .. } => formatter.write_str(message),
        }
    }
}

impl std::error::Error for ServiceError {}

/// Hint shown whenever the desktop cannot reach the service.
pub const UPGRADE_HINT: &str = "当前服务不支持此管理操作，请在设置中修复（升级）服务";
