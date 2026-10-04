/// 单次鼠标移动允许的最大坐标增量（像素），用于过滤异常跳变。
pub const MAX_MOUSE_MOVE_DELTA: i32 = 500;

// === Auth ===

/// 配对认证请求：客户端提交 PIN 与设备名以换取访问令牌。
#[derive(Debug, Deserialize)]
pub struct AuthPairRequest {
    /// 服务端界面显示的配对 PIN 码。
    pub pin: String,
    /// 发起配对的设备名称，用于在服务端标识来源。
    pub device_name: Option<String>,
}

// === System ===

/// 关机请求。
#[derive(Debug, Default, Deserialize)]
pub struct ShutdownRequest {
    /// 是否强制关机，跳过应用的保存提示。
    #[serde(default)]
    pub force: bool,
    /// 延迟执行的秒数，0 表示立即关机。
    #[serde(default)]
    pub delay_secs: u64,
}

/// 睡眠请求。
#[derive(Debug, Default, Deserialize)]
pub struct SleepRequest {
    /// true 表示休眠（状态写入磁盘），false 表示普通睡眠。
    #[serde(default)]
    pub hibernate: bool,
}

