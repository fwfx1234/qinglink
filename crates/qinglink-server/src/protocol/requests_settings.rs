// === Server settings ===

/// 更新服务端设置的请求，仅提交需要修改的字段。
#[derive(Debug, Default, Deserialize)]
pub struct UpdateSettingsRequest {
    /// 服务监听端口。
    #[serde(default)]
    pub port: Option<u16>,
    /// 是否随系统启动自动运行。
    #[serde(default)]
    pub auto_start: Option<bool>,
    /// 关闭窗口时是否最小化到托盘而非退出。
    #[serde(default)]
    pub minimize_to_tray: Option<bool>,
}

