//! 行解码辅助：`scanner_apps.args` 列编解码与目录行形态 [`DbAppEntry`]。

use crate::service::app_scanner::AppEntry;

/// Serialize launch arguments for the `scanner_apps.args` column.
pub(super) fn encode_args(args: &[String]) -> String {
    serde_json::to_string(args).unwrap_or_else(|_| "[]".to_string())
}

/// Parse the `scanner_apps.args` column, tolerating legacy/empty values.
pub(super) fn decode_args(raw: &str) -> Vec<String> {
    serde_json::from_str(raw).unwrap_or_default()
}

/// 数据库中的一行目录记录：`AppEntry` 加上持久化专用字段后的形态。
#[derive(Debug, Clone)]
pub struct DbAppEntry {
    /// 条目唯一标识。
    pub id: String,
    /// 当前显示名称（已应用重命名）。
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
    /// 是否在列表中隐藏。
    pub is_hidden: bool,
    /// 是否为用户手动添加的条目。
    pub is_user_added: bool,
    /// 启动时附加的命令行参数。
    pub args: Vec<String>,
    /// Preferred icon file recorded for this entry (user choice or scanner artwork).
    pub icon_path: Option<String>,
    /// Provenance of `icon_base64` (`manual` / `steam` / `executable` / …).
    pub icon_source: Option<String>,
    /// Unix seconds of the last icon resolution; used to bust client caches.
    pub icon_updated_at: Option<i64>,
    /// 常用（"我的库"）. Owned by the service so it follows the PC.
    pub is_favorite: bool,
    /// 置顶到手机首页.
    pub is_pinned: bool,
    /// 游戏启动时请求的显示模式。
    pub launch_mode: Option<crate::service::app_scanner::LaunchDisplayMode>,
}

impl From<DbAppEntry> for AppEntry {
    fn from(db: DbAppEntry) -> Self {
        AppEntry {
            id: db.id,
            name: db.name,
            original_name: db.original_name,
            exe_path: db.exe_path,
            icon_base64: db.icon_base64,
            icon_path: db.icon_path,
            icon_source: db.icon_source,
            category: db.category,
            source: db.source,
            args: db.args,
            is_favorite: db.is_favorite,
            is_pinned: db.is_pinned,
            launch_mode: db.launch_mode,
        }
    }
}
