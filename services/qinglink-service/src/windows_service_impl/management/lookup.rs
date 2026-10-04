//! 目录条目查找、路径规范化与入参校验。
//!
//! 每次管理端写入都要经过这里的校验器，界面往返的每个 id 都由 [`find_entry`] 解析，
//! 因此本模块是请求数据被允许变成目录数据的唯一去处。

use crate::windows_service_impl::management::record::CommandError;
use qinglink_protocol::service_ipc::{CatalogApp, CatalogFlags};
use qinglink_server::AppState;
use qinglink_server::service::app_db::DbAppEntry;
use sha2::{Digest, Sha256};

/// Upper bound for a user-supplied display name, in characters.
pub(super) const MAX_DISPLAY_NAME: usize = 128;

/// Upper bound for a user-supplied executable path, in characters.
pub(super) const MAX_APP_PATH: usize = 512;

/// Upper bound for the number of launch arguments of a user-added entry.
pub(super) const MAX_APP_ARGS: usize = 64;

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

pub(super) fn normalize_path(path: &str) -> String {
    path.trim()
        .trim_matches('"')
        .replace('/', "\\")
        .to_lowercase()
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

/// Validate a rename request.
///
/// Returns the trimmed name; an empty string is a valid request meaning
/// "restore the scanned name".
pub(super) fn validate_display_name(name: &str) -> std::result::Result<String, CommandError> {
    let trimmed = name.trim();
    if trimmed.chars().any(char::is_control) {
        return Err(CommandError::new("BAD_REQUEST", "名称不能包含控制字符"));
    }
    if trimmed.chars().count() > MAX_DISPLAY_NAME {
        return Err(CommandError::new(
            "BAD_REQUEST",
            format!("名称不能超过 {MAX_DISPLAY_NAME} 个字符"),
        ));
    }
    Ok(trimmed.to_string())
}

/// Validate a user-supplied executable path and return it trimmed.
///
/// The service is the only writer of the catalog, so this is also the only place
/// a path from the admin interface is trusted.
pub(super) fn validate_app_path(path: &str) -> std::result::Result<String, CommandError> {
    let trimmed = path.trim().trim_matches('"').trim();
    if trimmed.is_empty() {
        return Err(CommandError::new("BAD_REQUEST", "缺少可执行文件路径"));
    }
    if trimmed.chars().count() > MAX_APP_PATH || trimmed.chars().any(char::is_control) {
        return Err(CommandError::new("BAD_REQUEST", "可执行文件路径不合法"));
    }
    if !std::path::Path::new(trimmed).is_file() {
        return Err(CommandError::new("NOT_FOUND", "可执行文件不存在"));
    }
    Ok(trimmed.to_string())
}

/// Stable catalog id for a user-supplied path.
///
/// Derived from the normalized path so the same executable always maps to the
/// same row, which turns a repeated add into a conflict instead of a duplicate.
pub(super) fn app_id_for_path(path: &str) -> String {
    let digest = format!("{:x}", Sha256::digest(normalize_path(path).as_bytes()));
    format!("user:{}", &digest[..16])
}

/// Display name inferred from the executable when the caller left it empty.
pub(super) fn default_app_name(path: &str) -> String {
    std::path::Path::new(path)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(|stem| stem.trim().chars().take(MAX_DISPLAY_NAME).collect())
        .unwrap_or_default()
}

/// Strip control characters from user-supplied launch arguments.
pub(super) fn sanitize_args(args: &[String]) -> Vec<String> {
    args.iter()
        .map(|argument| {
            argument
                .chars()
                .filter(|character| !character.is_control())
                .collect::<String>()
        })
        .filter(|argument| !argument.is_empty())
        .take(MAX_APP_ARGS)
        .collect()
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
