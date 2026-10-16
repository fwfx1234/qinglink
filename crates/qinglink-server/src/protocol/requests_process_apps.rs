// === Processes ===

/// 进程列表查询参数。
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ProcessListQuery {
    /// 按进程名过滤的关键字，为空表示不过滤。
    pub search: Option<String>,
    /// 页码，从 0 开始。
    #[serde(default = "default_page")]
    pub page: usize,
    /// 每页返回的进程数量。
    #[serde(default = "default_page_size")]
    pub page_size: usize,
    /// 进程过滤: "user" 只显示用户进程, "all" 显示所有进程
    #[serde(default = "default_process_filter")]
    pub filter: String,
}

/// 进程图标查询参数。
///
/// 用单独的负载类型承载 `get_process_icon`，是为了不在 `requests_ws.rs`
/// 里再展开一组字段行（该文件受 AGENTS.md §2.3 的行数棘轮约束）。
#[derive(Debug, Clone, Deserialize)]
pub struct ProcessIconQuery {
    /// 目标进程 PID。
    pub pid: u32,
}

/// 更新启动器条目；省略显示模式字段表示保留现有配置。
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateScannerAppQuery {
    /// 条目唯一标识。
    pub id: String,
    /// 新的显示名称。
    pub name: String,
    /// 新的可执行文件路径。
    pub exe_path: String,
    /// 新的 Base64 图标，缺省表示保留原图标。
    pub icon_base64: Option<String>,
    /// 新的启动参数，缺省表示保留原参数。
    #[serde(default)]
    pub args: Option<Vec<String>>,
    /// 新的游戏启动显示模式，缺省表示不修改。
    #[serde(default)]
    pub launch_mode: Option<crate::service::app_scanner::LaunchDisplayMode>,
    /// 为 `true` 时清除已有的游戏启动显示模式。
    #[serde(default)]
    pub clear_launch_mode: bool,
}

/// 图标预览查询参数。
///
/// 手机在 PC 文件浏览器里挑图标文件时按路径预览一个文件；同样用单独的负载类型
/// 承载，避免 `requests_ws.rs` 再展开字段行。
#[derive(Debug, Clone, Deserialize)]
pub struct IconPreviewQuery {
    /// PC 上待预览的图标文件路径。
    pub path: String,
}

fn default_page() -> usize {
    0
}

fn default_page_size() -> usize {
    50
}

fn default_process_filter() -> String {
    "user".to_string()
}

fn default_process_sort() -> String {
    "active".to_string()
}

// === Apps ===

/// 启动应用请求。
#[derive(Debug, Deserialize)]
pub struct LaunchAppRequest {
    /// 要启动的可执行文件路径。
    pub path: String,
    /// 传给该程序的命令行参数。
    #[serde(default)]
    pub args: Vec<String>,
}

