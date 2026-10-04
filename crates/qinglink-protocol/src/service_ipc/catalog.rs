//! 目录（应用 / 游戏清单）载荷：条目、管理行、策展标记与统计。

use serde::{Deserialize, Serialize};

/// 目录中的一条精简条目。
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CatalogEntry {
    /// 条目标识。
    pub id: String,
    /// 条目显示名称。
    pub name: String,
    /// 条目来源（如扫描所得或用户添加）。
    pub source: String,
}

/// One row of the admin catalog. `id` may be a Windows path, so callers must
/// never place it in a URL path segment.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CatalogApp {
    /// 条目标识，可能是 Windows 路径或扫描得到的键。
    pub id: String,
    /// The name every client sees: the user override when present, otherwise
    /// the scanned name.
    pub name: String,
    /// 扫描得到的原始名称。
    pub original_name: String,
    /// 可执行文件路径。
    pub exe_path: String,
    /// 条目类别（`app` / `game`）。
    pub category: String,
    /// 条目来源。
    pub source: String,
    /// 是否对客户端隐藏。
    pub hidden: bool,
    /// 是否为用户手动添加。
    pub user_added: bool,
    /// 启动参数。
    pub args: Vec<String>,
    /// 是否已解析到图标。
    pub has_icon: bool,
    /// `name != original_name`, precomputed so the UI does not have to guess.
    pub renamed: bool,
    /// Provenance of the stored icon, when one was resolved.
    #[serde(default)]
    pub icon_source: Option<String>,
    /// Unix seconds of the last icon resolution; clients use it to bust caches.
    #[serde(default)]
    pub icon_updated_at: Option<i64>,
    /// 常用（"我的库"）.
    #[serde(default)]
    pub favorite: bool,
    /// 置顶到手机首页.
    #[serde(default)]
    pub pinned: bool,
}

/// Curation flags of one admin row, kept together because they are always
/// written and rendered as a pair.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
pub struct CatalogFlags {
    /// 是否属于常用（"我的库"）。
    #[serde(default)]
    pub favorite: bool,
    /// 是否置顶到手机首页。
    #[serde(default)]
    pub pinned: bool,
}

impl CatalogApp {
    /// Build one admin row from a persisted entry.
    ///
    /// `name` is already the coalesced display value by the time it reaches
    /// here, so `renamed` compares against the scanned name.
    #[allow(clippy::too_many_arguments)]
    pub fn from_parts(
        id: String,
        name: String,
        original_name: String,
        exe_path: String,
        category: String,
        source: String,
        hidden: bool,
        user_added: bool,
        args: Vec<String>,
        has_icon: bool,
        icon_source: Option<String>,
        icon_updated_at: Option<i64>,
        flags: CatalogFlags,
    ) -> Self {
        let renamed = name != original_name;
        Self {
            id,
            name,
            original_name,
            exe_path,
            category,
            source,
            hidden,
            user_added,
            args,
            has_icon,
            renamed,
            icon_source,
            icon_updated_at,
            favorite: flags.favorite,
            pinned: flags.pinned,
        }
    }
}

/// Per-kind counters for the admin UI.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct CatalogKindStats {
    /// 该类别的条目总数。
    pub total: usize,
    /// 其中已隐藏的条目数。
    pub hidden: usize,
    /// 其中用户手动添加的条目数。
    pub user_added: usize,
    /// 其中被重命名过的条目数。
    pub renamed: usize,
    /// How many of `total` are in 常用.
    #[serde(default)]
    pub favorite: usize,
}

/// 应用与游戏两个类别的目录统计。
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct CatalogStats {
    /// 应用类目录的统计。
    pub app: CatalogKindStats,
    /// 游戏类目录的统计。
    pub game: CatalogKindStats,
}
