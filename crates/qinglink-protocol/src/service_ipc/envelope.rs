//! 本地管理 IPC 的信封层：协议版本常量、请求、应答、事件与分页包装。

use serde::{Deserialize, Serialize};

use super::DesktopCommand;

/// 本地管理 IPC 的协议版本号。
pub const SERVICE_IPC_VERSION: u32 = 2;
/// 分页请求允许的最大单页条数。
pub const MAX_PAGE_SIZE: usize = 100;

/// 本地管理协议的一次请求。
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DesktopRequest {
    /// 请求使用的协议版本。
    pub version: u32,
    /// 请求标识，应答会原样回传以便配对。
    pub request_id: String,
    /// 具体的命令及其参数。
    #[serde(flatten)]
    pub command: DesktopCommand,
}

/// 分页结果的通用包装。
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Page<T> {
    /// 当前页的数据项。
    pub items: Vec<T>,
    /// 下一页的起始偏移量，已是末页时为 `None`。
    pub next_offset: Option<usize>,
}

/// Response envelope of the local management protocol.
///
/// `version` and `request_id` default so that a reply from a service predating
/// this envelope (which answered `{ok, code, message}` only) still decodes. The
/// caller can then classify it as a protocol mismatch instead of reporting a
/// transport failure.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ServiceResponse {
    /// 应答使用的协议版本。
    #[serde(default)]
    pub version: u32,
    /// 对应请求的标识。
    #[serde(default)]
    pub request_id: String,
    /// 请求是否成功。
    pub ok: bool,
    /// 结果码，标识成功或失败原因。
    pub code: String,
    /// 结果的文字说明。
    pub message: String,
    /// 各命令自己的返回数据。
    #[serde(default)]
    pub payload: serde_json::Value,
}

/// 服务主动推送的事件。
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ServiceEvent {
    /// 事件使用的协议版本。
    pub version: u32,
    /// 事件名称。
    pub event: String,
    /// 事件携带的数据。
    #[serde(default)]
    pub payload: serde_json::Value,
}
