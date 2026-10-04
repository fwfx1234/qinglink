//! 服务运行快照：状态、设备计数与目录扫描进度。

use serde::{Deserialize, Serialize};

use super::PendingPairing;

/// 服务运行状态快照。
#[derive(Clone, Default, Deserialize, Serialize)]
pub struct ServiceSnapshot {
    /// 服务是否处于暂停状态。
    pub paused: bool,
    /// 网络监听是否已就绪。
    pub network_ready: bool,
    /// 服务对外地址，尚未就绪时为 `None`。
    pub address: Option<String>,
    /// 最近一次错误信息，无错误时为 `None`。
    pub last_error: Option<String>,
    /// 当前待确认的配对请求，没有时为 `None`。
    pub pending_pairing: Option<PendingPairing>,
    /// Number of active device tokens.
    ///
    /// Optional so that a service predating this field still decodes; `None`
    /// means "unknown", which suppresses change detection instead of reporting a
    /// spurious drop to zero.
    #[serde(default)]
    pub device_count: Option<usize>,
    /// Completion stamp of the last catalog scan per kind, in Unix seconds.
    ///
    /// Also optional for the same reason. The desktop polls this snapshot, so
    /// this is how "the background scan finished" reaches connected browsers
    /// without a service-to-desktop event channel.
    #[serde(default)]
    pub scan: Option<CatalogScanState>,
}

/// 手写的 `Debug`：`pending_pairing` 只输出非敏感字段，PIN 永不出现在日志中。
impl std::fmt::Debug for ServiceSnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServiceSnapshot")
            .field("paused", &self.paused)
            .field("network_ready", &self.network_ready)
            .field("address", &self.address)
            .field("last_error", &self.last_error)
            .field(
                "pending_pairing",
                &self.pending_pairing.as_ref().map(|pending| {
                    (
                        &pending.request_id,
                        &pending.device_id,
                        &pending.device_name,
                        pending.expires_at_unix,
                    )
                }),
            )
            .field("device_count", &self.device_count)
            .field("scan", &self.scan)
            .finish()
    }
}

/// Last completed scan timestamp per catalog kind.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct CatalogScanState {
    /// 应用目录最近一次扫描完成的时间（Unix 秒）。
    pub app: Option<i64>,
    /// 游戏目录最近一次扫描完成的时间（Unix 秒）。
    pub game: Option<i64>,
}
