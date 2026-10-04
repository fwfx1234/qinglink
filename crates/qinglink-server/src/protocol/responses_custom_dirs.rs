// === Custom directory responses ===

/// 自定义扫描目录列表响应。
#[derive(Debug, Serialize)]
pub struct CustomDirListResponse {
    /// 已配置的自定义目录列表。
    pub dirs: Vec<crate::custom_dir::CustomDir>,
}

