// === System responses ===

/// 系统资源与运行状态。
#[derive(Debug, Serialize)]
pub struct SystemStatus {
    /// CPU 占用率（百分比）。
    pub cpu_usage: f32,
    /// 物理内存总量（字节）。
    pub memory_total: u64,
    /// 已使用的物理内存（字节）。
    pub memory_used: u64,
    /// 内存占用率（百分比）。
    pub memory_percent: f32,
    /// 系统已运行时长（秒）。
    pub uptime_seconds: u64,
}

/// 认证成功后返回的会话信息。
#[derive(Debug, Serialize)]
pub struct AuthenticatedResponse {
    /// 后续请求需要携带的访问令牌。
    pub token: String,
    /// 本次会话的唯一标识。
    pub session_id: String,
    /// 令牌过期时间（Unix 时间戳，秒），None 表示不过期。
    pub expires_at: Option<i64>,
    /// 已绑定设备的 MAC 地址。
    pub mac_address: Option<String>,
}


// 这 7 个响应体是**协议层的那一份**，不是复本。
//
// 它们曾在本文件里各定义一遍，与 `qinglink_protocol::platform` 逐字重复；
// 重复的代价已经发生过一次：`ForegroundResponse.focus_text` 在两边带着不同的
// `serde` 属性，同一条逻辑消息按"序列化时用哪份类型"发出两种线格式。
// 现在只保留一处定义，本文件只做重导出。
pub use qinglink_protocol::{
    AudioOutputDevice, AudioOutputsResponse, LockInputResult, LockInputStatus,
    SecureDesktopStatus, UnlockStatus, VolumeStatus,
};
