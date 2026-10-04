//! 服务健康检查（health）的应答类型。

use serde::{Deserialize, Serialize};

/// 服务存活探测的应答体。
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct HealthResponse {
    /// 服务所属 PC 的唯一标识。
    pub pc_id: String,
    /// 用户为该 PC 设置的展示名称。
    pub name: String,
    /// 健康检查应答的协议版本。
    pub version: u8,
    /// 服务实际监听的 TCP 端口。
    pub port: u16,
}
