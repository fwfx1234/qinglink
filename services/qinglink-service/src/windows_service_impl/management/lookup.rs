//! 目录条目查找、路径规范化与入参校验。
//!
//! 校验器本体在 `qinglink_server::service::app_catalog`：同一份目录有两个写入口
//! （手机端 WS 与桌面管理 IPC），校验器必须两边共用，而依赖方向只能是
//! `service → server`，所以它只能放在 server crate。本模块只保留"查找"这个
//! 依赖 `CommandError` 的部分，并对共用校验器做一次错误类型转换。

use crate::windows_service_impl::management::record::CommandError;
use qinglink_protocol::service_ipc::{CatalogApp, CatalogFlags};
use qinglink_server::AppState;
use qinglink_server::service::app_db::DbAppEntry;

pub(super) use qinglink_server::service::app_catalog::{
    app_id_for_path, default_app_name, normalize_path, sanitize_args,
    validate_app_path as validate_app_path_inner,
    validate_display_name as validate_display_name_inner,
};

/// 校验显示名；错误类型转换为管理命令的 `CommandError`。
pub(super) fn validate_display_name(name: &str) -> std::result::Result<String, CommandError> {
    validate_display_name_inner(name).map_err(CommandError::from)
}

/// 校验可执行文件路径；错误类型转换为管理命令的 `CommandError`。
pub(super) fn validate_app_path(path: &str) -> std::result::Result<String, CommandError> {
    validate_app_path_inner(path).map_err(CommandError::from)
}

/// Resolve an admin row by its catalog id.
///
/// The UI round-trips whatever `id` the list gave it, but a stale id whose
/// executable moved is still resolvable by path, so accept both.
pub(super) fn find_entry(
    state: &AppState,
    id: &str,
) -> std::result::Result<DbAppEntry, CommandError> {
    let wanted = id.trim();
    if wanted.is_empty() {
        return Err(CommandError::new("BAD_REQUEST", "缺少条目 id"));
    }
    let normalized = normalize_path(wanted);
    state
        .admin_catalog()
        .map_err(CommandError::internal)?
        .into_iter()
        .find(|entry| entry.id == wanted || normalize_path(&entry.exe_path) == normalized)
        .ok_or_else(CommandError::not_found)
}

pub(super) fn normalize_query(query: Option<String>) -> Option<String> {
    query
        .map(|query| query.trim().to_lowercase())
        .filter(|query| !query.is_empty())
}

pub(super) fn matches_query(entry: &DbAppEntry, query: Option<&str>) -> bool {
    let Some(query) = query else {
        return true;
    };
    entry.name.to_lowercase().contains(query)
        || entry.original_name.to_lowercase().contains(query)
        || entry.category.to_lowercase().contains(query)
        || entry.exe_path.to_lowercase().contains(query)
}

/// Cover art Steam already downloaded for an installed game.
pub(super) fn steam_artwork(state: &AppState, id: &str) -> Option<String> {
    let app_id: u32 = id.strip_prefix("steam:")?.parse().ok()?;
    let database = state.app_db.lock().ok()?;
    database
        .get_steam_games()
        .ok()?
        .into_iter()
        .find(|game| game.app_id == app_id)?
        .icon_path
}

/// Assemble the row shape the admin UI lists, from the persisted entry.
pub(super) fn catalog_row(entry: DbAppEntry) -> CatalogApp {
    let has_icon = entry.icon_base64.is_some();
    CatalogApp::from_parts(
        entry.id,
        entry.name,
        entry.original_name,
        entry.exe_path,
        entry.category,
        entry.source,
        entry.is_hidden,
        entry.is_user_added,
        entry.args,
        has_icon,
        entry.icon_source,
        entry.icon_updated_at,
        CatalogFlags {
            favorite: entry.is_favorite,
            pinned: entry.is_pinned,
        },
    )
}
