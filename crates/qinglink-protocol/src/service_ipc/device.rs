//! 设备与配对载荷：待确认配对请求与已配对设备摘要。

use serde::{Deserialize, Serialize};

/// 一条等待用户确认的配对请求，其中携带配对 PIN。
///
/// A PIN is intentionally excluded from Debug diagnostics.
///
/// 因此本类型刻意不派生 `Debug`：缺少该实现才能保证 PIN 不会被打进日志。
#[derive(Clone, Deserialize, Serialize)]
#[allow(missing_debug_implementations)]
pub struct PendingPairing {
    /// 配对请求标识。
    pub request_id: String,
    /// 发起配对的设备标识。
    pub device_id: String,
    /// 发起配对的设备名称。
    pub device_name: String,
    /// 需要用户在 PC 端输入的配对 PIN。
    pub pin: String,
    /// 该请求的过期时间（Unix 秒）。
    pub expires_at_unix: i64,
}

/// 已配对设备的摘要信息。
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DeviceSummary {
    /// 设备唯一标识。
    pub id: String,
    /// 设备展示名称。
    pub device_name: String,
}
