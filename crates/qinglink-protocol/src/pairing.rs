//! 设备配对（pairing）流程的请求与应答类型。

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 一台可用于网络唤醒（Wake-on-LAN）的网卡。
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct WakeNetworkAdapter {
    /// 网卡的 MAC 地址。
    pub mac_address: String,
    /// 网卡的 IPv4 地址。
    pub ipv4_address: String,
    /// 子网前缀长度（CIDR 前缀位数）。
    pub prefix_length: u8,
    /// 该网段的广播地址，唤醒魔术包发往此处。
    pub broadcast_address: String,
}

/// 客户端发起配对时提交的自身信息。
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct PairingRequest {
    /// 发起配对的设备唯一标识。
    pub device_id: String,
    /// 发起配对的设备展示名称。
    pub device_name: String,
}

/// 配对请求被受理后返回的待确认凭据。
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct PairingRequestResponse {
    /// 本次配对请求的标识，确认时需要回传。
    pub request_id: Uuid,
    /// 该请求的过期时间（Unix 秒）。
    pub expires_at_unix: i64,
}

/// 客户端提交 PIN 以确认配对。
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct PairingConfirm {
    /// 待确认的配对请求标识。
    pub request_id: Uuid,
    /// PC 端显示、由用户输入的配对 PIN。
    pub pin: String,
    /// 发起配对的设备展示名称。
    pub device_name: String,
    /// 发起配对的设备唯一标识。
    pub device_id: String,
}

/// 配对成功后下发的访问令牌及 PC 信息。
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct PairingTokenResponse {
    /// 后续请求使用的访问令牌。
    pub token: String,
    /// 被配对 PC 的唯一标识。
    pub pc_id: String,
    /// 被配对 PC 的展示名称。
    pub pc_name: String,
    /// PC 的 MAC 地址，用于局域网唤醒；未知时为 `None`。
    pub mac_address: Option<String>,
    /// 可用于唤醒的网卡列表；未采集到该信息时为 `None`。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wake_networks: Option<Vec<WakeNetworkAdapter>>,
}
