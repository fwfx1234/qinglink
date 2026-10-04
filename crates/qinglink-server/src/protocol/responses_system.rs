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

/// 系统音量状态。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct VolumeStatus {
    /// 当前音量百分比（0-100）。
    pub percent: u8,
    /// 是否已静音。
    pub muted: bool,
}

/// 单个音频输出设备。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AudioOutputDevice {
    /// 设备唯一标识。
    pub id: String,
    /// 设备显示名称。
    pub name: String,
    /// 是否为系统默认输出设备。
    pub is_default: bool,
}

/// 音频输出设备列表响应。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AudioOutputsResponse {
    /// 可用的输出设备列表。
    pub devices: Vec<AudioOutputDevice>,
}

/// 锁屏与解锁状态。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UnlockStatus {
    /// 当前是否处于锁定状态。
    pub locked: bool,
    /// 当前锁定模式的标识。
    pub mode: String,
    /// 是否已启用自动登录。
    pub auto_login_enabled: bool,
    /// 状态相关的提示信息。
    pub message: String,
    /// 是否检测到 UAC 提权界面。
    #[serde(default)]
    pub uac_detected: bool,
    /// 当前所处的桌面或界面标识。
    #[serde(default)]
    pub surface: String,
}

/// 锁屏输入通道的可用状态。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LockInputStatus {
    /// 输入通道当前是否可用。
    pub available: bool,
    /// 后台服务是否正在运行。
    pub service_running: bool,
    /// 状态相关的提示信息。
    pub message: String,
}

/// 锁屏输入操作的结果。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LockInputResult {
    /// 结果码。
    pub code: String,
    /// 结果的描述信息。
    pub message: String,
}

/// 安全桌面（Winlogon 桌面）的运行状态。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SecureDesktopStatus {
    /// 当前安全桌面的名称。
    pub desktop: String,
    /// 安全桌面是否处于锁定状态。
    pub locked: bool,
    /// 输入注入通道是否已就绪。
    pub input_ready: bool,
    /// 是否支持模拟 SAS（Ctrl+Alt+Del）。
    pub sas_supported: bool,
    /// 目标 Windows 会话 ID。
    pub session_id: u32,
    /// 安全桌面代理的运行状态。
    pub agent_state: String,
    /// 是否检测到 UAC 提权界面。
    #[serde(default)]
    pub uac_detected: bool,
    /// 当前令牌的完整性级别。
    #[serde(default)]
    pub token_integrity_level: u32,
    /// 最近一次错误信息，无错误时为空。
    #[serde(default)]
    pub last_error: String,
}
