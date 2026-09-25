// === Processes ===

#[derive(Debug, Default, Deserialize)]
pub struct ProcessListQuery {
    pub search: Option<String>,
    #[serde(default = "default_page")]
    pub page: usize,
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

#[derive(Debug, Deserialize)]
pub struct LaunchAppRequest {
    pub path: String,
    #[serde(default)]
    pub args: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct SearchAppsQuery {
    pub query: String,
}


