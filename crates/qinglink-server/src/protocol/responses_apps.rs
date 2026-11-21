// === App responses ===

// `AppInfo` 只有协议层一份定义，本文件只做重导出。
pub use qinglink_protocol::AppInfo;

/// 应用搜索结果响应。
#[derive(Debug, Serialize)]
pub struct SearchAppsResponse {
    /// 匹配到的应用列表。
    pub apps: Vec<AppInfo>,
}

