// === Process responses ===

/// 单个进程的详细信息。
#[derive(Debug, Clone, Serialize)]
pub struct ProcessInfo {
    /// 进程 PID。
    pub pid: u32,
    /// 进程名。
    pub name: String,
    /// 进程占用的物理内存（字节）。
    pub memory_bytes: u64,
    /// 进程 CPU 占用率（百分比）。
    pub cpu_usage: f32,
    /// 进程可执行文件的完整路径。
    pub path: Option<String>,
    /// 进程启动时间（Unix 时间戳，秒）。
    pub started_at: u64,
    /// 分组：app（有可见窗口）/ background（后台）/ system（Windows 进程）。
    #[serde(default)]
    pub group: String,
}

/// 进程列表分页响应。
#[derive(Debug, Serialize)]
pub struct ProcessListResponse {
    /// 当前页的进程列表。
    pub processes: Vec<ProcessInfo>,
    /// 符合条件的进程总数。
    pub total: usize,
    /// 当前页码，从 0 开始。
    pub page: usize,
    /// 每页的进程数量。
    pub page_size: usize,
    /// 是否还有下一页。
    pub has_more: bool,
}

/// Small launch-directory record used by mobile clients. Icons stay out of the
/// list payload, but the path is included so clients can show item details.
#[derive(Debug, Clone, Serialize)]
pub struct LauncherItemSummary {
    /// 条目唯一标识。
    pub id: String,
    /// 条目的显示名称。
    pub name: String,
    /// 可执行文件的原始文件名。
    pub original_name: String,
    /// 可执行文件完整路径。
    pub exe_path: String,
    /// 条目所属的分类。
    pub category: String,
    /// 条目的来源，例如目录扫描或手动添加。
    pub source: String,
    /// Extra launch arguments configured for this entry.
    #[serde(default)]
    pub args: Vec<String>,
    /// Provenance of the entry's stored icon, so clients can explain an icon
    /// that does not match the program.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_source: Option<String>,
    /// Folder-scan hint: executable size in bytes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
    /// Folder-scan hint: this executable looks like its directory's main program.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recommended: Option<bool>,
    /// 常用（"我的库"）标记，由服务端拥有，因此换手机也一致。
    #[serde(default)]
    pub favorite: bool,
    /// 置顶到手机首页.
    #[serde(default)]
    pub pinned: bool,
    /// Configured game display mode applied before launch.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub launch_mode: Option<crate::service::app_scanner::LaunchDisplayMode>,
    /// Only populated for explicit folder-scan candidates. Keeping this out
    /// of normal catalog responses avoids inflating every library request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_base64: Option<String>,
}

/// One pickable icon source for a catalog entry, offered to the clients so a
/// wrong icon can be fixed without guessing a file path.
#[derive(Debug, Clone, Serialize)]
pub struct IconCandidateSummary {
    /// 候选图标对应的文件路径。
    pub path: String,
    /// `executable` | `shortcut` | `shortcut_icon` | `image`
    pub kind: String,
    /// 图标文件的大小（字节）。
    pub size_bytes: u64,
}

/// 某个条目的可选图标来源列表。
#[derive(Debug, Serialize)]
pub struct IconCandidatesResponse {
    /// 条目唯一标识。
    pub id: String,
    /// 可选的图标来源列表。
    pub candidates: Vec<IconCandidateSummary>,
}

/// 图标更新结果。
#[derive(Debug, Serialize)]
pub struct IconUpdateResponse {
    /// 条目唯一标识。
    pub id: String,
    /// 更新后的图标 Base64 数据，无图标时省略。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_base64: Option<String>,
    /// 最终采用的图标来源标识。
    pub source: String,
    /// 图标来源文件的路径。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_path: Option<String>,
}

/// 单个图标文件的预览结果。
///
/// 手机在 PC 文件浏览器里挑图标时按路径预览：只回图片本身，不落库、不改动条目。
#[derive(Debug, Serialize)]
pub struct IconPreviewResponse {
    /// 被预览文件的路径（回显请求）。
    pub path: String,
    /// 预览图 PNG 数据（Base64）；文件不可读或不是可解析的图标时省略。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_base64: Option<String>,
}

/// 单个进程的图标查询结果。
///
/// 解析不到图标不是错误：`source` 为 `none` 且不带载荷，客户端保留自己的
/// 占位图标即可。
#[derive(Debug, Serialize)]
pub struct ProcessIconResponse {
    /// 目标进程 PID（回显请求）。
    pub pid: u32,
    /// 图标来源标识：`executable` | `image` | `none`。
    pub source: String,
    /// 图标来源文件的路径（程序本身或随附素材），无图标时省略。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_path: Option<String>,
    /// 图标 PNG 数据（Base64），无图标时省略。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_base64: Option<String>,
}

/// Blocked (hidden) catalog entries, listed so a user can unblock them.
#[derive(Debug, Serialize)]
pub struct BlockedLauncherItemsResponse {
    /// 被屏蔽的条目列表。
    pub items: Vec<LauncherItemSummary>,
    /// 被屏蔽的条目总数。
    pub total: usize,
}

/// 启动器条目的分页响应。
#[derive(Debug, Serialize)]
pub struct LauncherItemPage {
    /// 当前页的条目列表。
    pub items: Vec<LauncherItemSummary>,
    /// 符合条件的条目总数。
    pub total: usize,
    /// 当前页码，从 0 开始。
    pub page: usize,
    /// 每页的条目数量。
    pub page_size: usize,
    /// 是否还有下一页。
    pub has_more: bool,
}

/// 启动器条目的图标响应。
#[derive(Debug, Serialize)]
pub struct LauncherIconResponse {
    /// 条目唯一标识。
    pub id: String,
    /// 图标的 Base64 数据，无图标时为 None。
    pub icon_base64: Option<String>,
}

/// 游戏目录扫描的候选条目响应。
#[derive(Debug, Serialize)]
pub struct GameScanCandidatesResponse {
    /// 被扫描的目录路径。
    pub path: String,
    /// 扫描得到的候选条目。
    pub candidates: Vec<LauncherItemSummary>,
}

/// 前台窗口信息响应。
#[derive(Debug, Serialize)]
pub struct ForegroundResponse {
    /// 前台窗口所属进程的 PID。
    pub pid: u32,
    /// 前台窗口标题。
    pub title: String,
    /// 前台窗口所属进程的可执行文件路径。
    pub path: Option<String>,
    /// 前台窗口所属进程的进程名。
    pub process_name: String,
    /// 前台窗口句柄（HWND）的数值形式。
    pub hwnd: usize,
}

/// 显示器的显示模式。
#[derive(Debug, Clone, Serialize)]
pub struct DisplayModeResponse {
    /// 水平分辨率（像素）。
    pub width: u32,
    /// 垂直分辨率（像素）。
    pub height: u32,
    /// 刷新率，单位为毫赫兹（mHz）。
    pub refresh_millihz: u32,
}

/// 单个显示器的信息。
#[derive(Debug, Clone, Serialize)]
pub struct DisplayInfoResponse {
    /// 显示器唯一标识。
    pub id: String,
    /// 显示器名称。
    pub name: String,
    /// 是否为主显示器。
    pub primary: bool,
    /// 当前生效的显示模式。
    pub current_mode: DisplayModeResponse,
    /// 该显示器支持的全部显示模式。
    pub modes: Vec<DisplayModeResponse>,
}

/// 显示器列表响应。
#[derive(Debug, Clone, Serialize)]
pub struct DisplaysResponse {
    /// 系统中的显示器列表。
    pub displays: Vec<DisplayInfoResponse>,
}
