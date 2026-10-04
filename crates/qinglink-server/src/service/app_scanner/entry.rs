//! 启动器条目类型：目录条目、目录扫描候选与 WebSocket 响应结构。

use serde::{Deserialize, Serialize};

/// 启动器目录中的一个条目（软件、游戏或手动添加项）。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppEntry {
    /// 条目唯一标识，带来源前缀（如 `startmenu:` / `steam:` / `candidate:`）。
    pub id: String,
    /// 当前显示名称，可能是用户重命名后的结果。
    pub name: String,
    /// 扫描器给出的原始名称，重命名时作为稳定键。
    pub original_name: String,
    /// 可执行文件或快捷方式路径。
    pub exe_path: String,
    /// 缓存的图标数据（base64 编码的 PNG）。
    pub icon_base64: Option<String>,
    /// Preferred icon file: an explicit user choice, or artwork the scanner
    /// recorded for this entry. Consulted before any automatic discovery.
    #[serde(default)]
    pub icon_path: Option<String>,
    /// Provenance of `icon_base64` (`manual` / `steam` / `executable` / …).
    #[serde(default)]
    pub icon_source: Option<String>,
    /// 分类：`Application` 或 `Game`。
    pub category: String,
    /// 来源：`StartMenu` / `Steam` / `Custom` / `User` 等。
    pub source: String,
    /// Extra command-line arguments appended when launching this entry.
    #[serde(default)]
    pub args: Vec<String>,
    /// 常用（"我的库"）标记。The service owns it so it follows the PC rather
    /// than the phone that happened to add it.
    #[serde(default)]
    pub is_favorite: bool,
    /// 置顶到手机首页.
    #[serde(default)]
    pub is_pinned: bool,
}

/// One executable found by a user-initiated folder scan.
///
/// Scan hints stay out of [`AppEntry`] because they describe the *scan*, not
/// the catalog row: they only exist to let the client pre-select the handful of
/// programs a folder actually launches.
#[derive(Clone, Debug)]
pub struct ScanCandidate {
    /// 由本次扫描构造出的目录条目。
    pub entry: AppEntry,
    /// 可执行文件大小（字节），用于推荐度排序。
    pub size_bytes: u64,
    /// Directory depth below the scan root (0 = the root itself).
    pub depth: u32,
    /// True for the one executable per directory that looks like its main program.
    pub recommended: bool,
}

/// WebSocket response format (matches the Flutter client ScannerAppEntry model).
#[derive(Clone, Debug, Serialize)]
pub struct ScannerAppEntryResponse {
    /// 条目唯一标识。
    pub id: String,
    /// 显示名称。
    pub name: String,
    /// 扫描器给出的原始名称。
    pub original_name: String,
    /// 可执行文件或快捷方式路径。
    pub exe_path: String,
    /// 缓存的图标数据（base64 编码的 PNG）。
    pub icon_base64: Option<String>,
    /// 分类：`Application` 或 `Game`。
    pub category: String,
    /// 来源：`StartMenu` / `Steam` / `Custom` / `User` 等。
    pub source: String,
    /// Extra command-line arguments saved for this scanner entry.
    #[serde(default)]
    pub args: Vec<String>,
    /// Provenance of `icon_base64`, so clients can explain an odd icon.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_source: Option<String>,
    /// 是否置顶到手机首页。
    pub pinned: bool,
    /// 是否属于常用（“我的库”）。
    pub quick: bool,
}

impl From<AppEntry> for ScannerAppEntryResponse {
    fn from(entry: AppEntry) -> Self {
        Self {
            id: entry.id,
            name: entry.name,
            original_name: entry.original_name,
            exe_path: entry.exe_path,
            icon_base64: entry.icon_base64,
            category: entry.category,
            source: entry.source,
            args: entry.args,
            icon_source: entry.icon_source,
            pinned: entry.is_pinned,
            quick: entry.is_favorite,
        }
    }
}
