// === Processes ===

/// 进程列表查询参数。
#[derive(Debug, Default, Deserialize)]
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

