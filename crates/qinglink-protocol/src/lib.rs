//! qinglink 各进程之间共享的协议类型定义。
//!
//! 覆盖健康检查、设备配对、桌面平台能力，以及桌面服务本地管理 IPC 的命令、
//! 应答与事件。这里只约定 JSON 结构，不包含任何传输实现，供桌面端、服务端与
//! 手机端复用。

pub mod health;
pub mod pairing;
pub mod platform;
pub mod service_ipc;

pub use health::HealthResponse;
pub use pairing::{
    PairingConfirm, PairingRequest, PairingRequestResponse, PairingTokenResponse,
    WakeNetworkAdapter,
};
pub use platform::{
    AppInfo, AudioOutputDevice, AudioOutputsResponse, DisplayInfoResponse, DisplayModeResponse,
    ForegroundResponse, LockInputResult, LockInputStatus, MAX_MOUSE_MOVE_DELTA,
    SecureDesktopStatus, UnlockStatus, VolumeStatus, WindowSwitchDirection,
};

/// 桌面服务默认监听的 TCP 端口。
pub const DEFAULT_PORT: u16 = 3721;
/// 局域网设备发现使用的 UDP 端口。
pub const DISCOVERY_PORT: u16 = 3723;
/// 设备发现报文携带的魔数，用于识别 qinglink 广播。
pub const DISCOVERY_MAGIC: &str = "qinglink-discover-v1";
