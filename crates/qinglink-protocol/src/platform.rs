//! 桌面平台能力相关的协议类型（应用、音量、锁屏、显示、窗口等）。

use serde::{Deserialize, Serialize};

/// 单次鼠标移动允许的最大位移（像素），超出部分会被裁剪。
pub const MAX_MOUSE_MOVE_DELTA: i32 = 500;

/// 窗口切换的方向。
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WindowSwitchDirection {
    /// 切换到下一个窗口。
    Next,
    /// 切换到上一个窗口。
    Previous,
}

/// 一个可启动的应用。
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppInfo {
    /// 应用显示名称。
    pub name: String,
    /// 可执行文件或快捷方式路径。
    pub path: String,
}
/// 一个音频输出设备。
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AudioOutputDevice {
    /// 设备唯一标识。
    pub id: String,
    /// 设备显示名称。
    pub name: String,
    /// 是否为系统默认输出设备。
    pub is_default: bool,
}
/// 音频输出设备列表的应答。
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AudioOutputsResponse {
    /// 系统当前的输出设备列表。
    pub devices: Vec<AudioOutputDevice>,
}
/// 系统音量状态。
///
/// `Copy` 是刻意的：它只有两个标量字段，服务端多处把它当作值类型传递。
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct VolumeStatus {
    /// 音量百分比（0–100）。
    pub percent: u8,
    /// 是否已静音。
    pub muted: bool,
}
/// 锁屏 / 解锁状态。
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct UnlockStatus {
    /// 当前是否处于锁定状态。
    pub locked: bool,
    /// 锁屏类型的标识。
    pub mode: String,
    /// 是否已启用自动登录。
    pub auto_login_enabled: bool,
    /// 供界面展示的状态说明。
    pub message: String,
    /// 是否检测到 UAC 提权界面。
    #[serde(default)]
    pub uac_detected: bool,
    /// 当前锁屏界面所属的表面（如登录界面、安全桌面）。
    #[serde(default)]
    pub surface: String,
}
/// 输入锁定能力的可用状态。
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct LockInputStatus {
    /// 本机是否支持锁定输入。
    pub available: bool,
    /// 负责锁定输入的服务是否正在运行。
    pub service_running: bool,
    /// 供界面展示的状态说明。
    pub message: String,
}
/// 一次锁定输入操作的结果。
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct LockInputResult {
    /// 结果码，标识成功或失败原因。
    pub code: String,
    /// 结果的文字说明。
    pub message: String,
}
/// 安全桌面（Secure Desktop）状态。
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SecureDesktopStatus {
    /// 当前桌面名称。
    pub desktop: String,
    /// 是否处于锁定状态。
    pub locked: bool,
    /// 输入通道是否已就绪。
    pub input_ready: bool,
    /// 是否支持发送 SAS（Ctrl+Alt+Del）。
    pub sas_supported: bool,
    /// 当前 Windows 会话 ID。
    pub session_id: u32,
    /// 代理进程的状态描述。
    pub agent_state: String,
    /// 是否检测到 UAC 提权界面。
    #[serde(default)]
    pub uac_detected: bool,
    /// 当前进程令牌的完整性级别。
    #[serde(default)]
    pub token_integrity_level: u32,
    /// 最近一次失败的错误信息，无错误时为空字符串。
    #[serde(default)]
    pub last_error: String,
}
/// 当前前台窗口的信息。
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ForegroundResponse {
    /// 前台窗口所属进程的 ID。
    pub pid: u32,
    /// 窗口标题。
    pub title: String,
    /// 前台进程的可执行文件路径，无法获取时为 `None`。
    pub path: Option<String>,
    /// 前台进程名。
    pub process_name: String,
    /// 窗口句柄数值。
    pub hwnd: usize,
    /// 焦点输入框里已有的文本，供手机端回填输入框；没有可回填的输入框时为 `None`。
    ///
    /// `skip_serializing_if` 不是可选优化：服务端此前在自己的复本里就有它，
    /// 而协议层没有，导致同一逻辑消息按"序列化时用哪份类型"发出两种线格式
    /// （有字段 vs 有 `null`）。现在只有这一份定义，行为固定为"无文本时省略字段"；
    /// `default` 保留在反序列化侧，让不发送该字段的旧服务端仍可被解析。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focus_text: Option<String>,
}
/// 物理鼠标按键的按下状态。
///
/// 服务端按固定间隔读取一次，仅在变化时通过 `input_state` 事件下发；手机端据此显示
/// "PC 侧鼠标正处于按下状态"，并在拖拽开始前避免重复下发按下。
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct MouseButtonsState {
    /// 左键是否按下。
    pub left: bool,
    /// 右键是否按下。
    pub right: bool,
    /// 中键是否按下。
    pub middle: bool,
}
/// 一种显示模式（分辨率与刷新率）。
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DisplayModeResponse {
    /// 水平像素数。
    pub width: u32,
    /// 垂直像素数。
    pub height: u32,
    /// 刷新率，单位为毫赫兹（mHz）。
    pub refresh_millihz: u32,
}
/// 一台显示器的信息。
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DisplayInfoResponse {
    /// 显示器唯一标识。
    pub id: String,
    /// 显示器名称。
    pub name: String,
    /// 是否为主显示器。
    pub primary: bool,
    /// 当前生效的显示模式。
    pub current_mode: DisplayModeResponse,
    /// 常用显示模式；当前模式不在其中时也会附在列表末尾。
    pub modes: Vec<DisplayModeResponse>,
}
