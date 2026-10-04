// === App responses ===

/// 已安装应用的基本信息。
#[derive(Debug, Serialize)]
pub struct AppInfo {
    /// 应用显示名称。
    pub name: String,
    /// 应用可执行文件路径。
    pub path: String,
}

/// 应用搜索结果响应。
#[derive(Debug, Serialize)]
pub struct SearchAppsResponse {
    /// 匹配到的应用列表。
    pub apps: Vec<AppInfo>,
}

