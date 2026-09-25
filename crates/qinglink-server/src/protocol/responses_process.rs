// === Process responses ===

#[derive(Debug, Clone, Serialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub memory_bytes: u64,
    pub cpu_usage: f32,
    pub path: Option<String>,
    pub started_at: u64,
    /// 分组：app（有可见窗口）/ background（后台）/ system（Windows 进程）。
    #[serde(default)]
    pub group: String,
}

#[derive(Debug, Serialize)]
pub struct ProcessListResponse {
    pub processes: Vec<ProcessInfo>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
    pub has_more: bool,
}

/// Small launch-directory record used by mobile clients. Icons stay out of the
/// list payload, but the path is included so clients can show item details.
#[derive(Debug, Clone, Serialize)]
pub struct LauncherItemSummary {
    pub id: String,
    pub name: String,
    pub original_name: String,
    pub exe_path: String,
    pub category: String,
    pub source: String,
    /// Extra launch arguments configured for this entry.
    #[serde(default)]
    pub args: Vec<String>,
    /// Only populated for explicit folder-scan candidates. Keeping this out
    /// of normal catalog responses avoids inflating every library request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_base64: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LauncherItemPage {
    pub items: Vec<LauncherItemSummary>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
    pub has_more: bool,
}

#[derive(Debug, Serialize)]
pub struct LauncherIconResponse {
    pub id: String,
    pub icon_base64: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GameScanCandidatesResponse {
    pub path: String,
    pub candidates: Vec<LauncherItemSummary>,
}

#[derive(Debug, Serialize)]
pub struct ForegroundResponse {
    pub pid: u32,
    pub title: String,
    pub path: Option<String>,
    pub process_name: String,
    pub hwnd: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct DisplayModeResponse {
    pub width: u32,
    pub height: u32,
    pub refresh_millihz: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct DisplayInfoResponse {
    pub id: String,
    pub name: String,
    pub primary: bool,
    pub current_mode: DisplayModeResponse,
    pub modes: Vec<DisplayModeResponse>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DisplaysResponse {
    pub displays: Vec<DisplayInfoResponse>,
}
