// === Server management responses ===

/// 服务端运行设置。
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServerSettings {
    /// 服务监听端口。
    pub port: u16,
    /// 是否随系统启动自动运行。
    pub auto_start: bool,
    /// 关闭窗口时是否最小化到托盘而非退出。
    pub minimize_to_tray: bool,
}

impl Default for ServerSettings {
    fn default() -> Self {
        Self {
            port: 3721,
            auto_start: false,
            minimize_to_tray: false,
        }
    }
}

