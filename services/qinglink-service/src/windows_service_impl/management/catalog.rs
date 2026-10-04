//! 目录读取：记录列表、条目详情与统计。
//!
//! 这里只读，不改动任何持久化状态；所有写操作走 `catalog_edit` / `apps` /
//! `icons`。

use crate::windows_service_impl::management::lookup::{
    catalog_row, find_entry, matches_query, normalize_query,
};
use crate::windows_service_impl::management::page::{
    bounded_page, bounded_page_with_total, check_page,
};
use crate::windows_service_impl::management::record::{CommandError, CommandResult, value};
use qinglink_protocol::service_ipc::{CatalogApp, CatalogEntry, CatalogKindStats, CatalogStats};
use qinglink_server::AppState;
use qinglink_server::service::app_db::DbAppEntry;
use qinglink_server::service::app_scanner::AppScanner;

/// The list filter carried by `DesktopCommand::ListApps`.
pub(super) struct ListFilter {
    pub(super) kind: String,
    pub(super) offset: usize,
    pub(super) limit: usize,
    pub(super) query: Option<String>,
    pub(super) include_hidden: bool,
    pub(super) favorite_only: bool,
}

/// Entries visible to the mobile clients, without hidden rows.
pub(super) fn get_catalog(state: &AppState, offset: usize, limit: usize) -> CommandResult {
    check_page(offset, limit)?;
    let apps = state
        .app_db
        .lock()
        .map_err(|_| CommandError::internal("catalog unavailable"))?
        .get_visible_apps()
        .map_err(CommandError::internal)?;
    let more = offset.saturating_add(limit) < apps.len();
    let items = apps
        .into_iter()
        .skip(offset)
        .take(limit)
        .map(|app| CatalogEntry {
            id: app.id,
            name: app.name.chars().take(256).collect(),
            source: app.source.chars().take(128).collect(),
        })
        .collect();
    bounded_page(items, offset, more)
}

/// The admin listing, which keeps hidden rows so the UI can unhide them.
pub(super) fn list_apps(state: &AppState, filter: ListFilter) -> CommandResult {
    let ListFilter {
        kind,
        offset,
        limit,
        query,
        include_hidden,
        favorite_only,
    } = filter;
    check_page(offset, limit)?;
    let kind = qinglink_server::server::normalize_catalog_kind(&kind);
    let query = normalize_query(query);
    let entries = state.admin_catalog().map_err(CommandError::internal)?;
    let matching: Vec<DbAppEntry> = entries
        .into_iter()
        .filter(|entry| AppScanner::catalog_kind(&entry.category) == kind)
        .filter(|entry| include_hidden || !entry.is_hidden)
        .filter(|entry| !favorite_only || entry.is_favorite)
        .filter(|entry| matches_query(entry, query.as_deref()))
        .collect();
    let total = matching.len();
    let items: Vec<CatalogApp> = matching
        .into_iter()
        .skip(offset)
        .take(limit)
        .map(catalog_row)
        .collect();
    let more = offset.saturating_add(items.len()) < total;
    bounded_page_with_total(items, offset, more, Some(total), Some(kind))
}

/// One entry by id, or the path-equivalent row when the id went stale.
pub(super) fn get_app(state: &AppState, id: &str) -> CommandResult {
    let entry = find_entry(state, id)?;
    value(catalog_row(entry))
}

/// Per-kind counters shown on the admin dashboard.
pub(super) fn get_catalog_stats(state: &AppState) -> CommandResult {
    let entries = state.admin_catalog().map_err(CommandError::internal)?;
    value(CatalogStats {
        app: kind_stats(&entries, "app"),
        game: kind_stats(&entries, "game"),
    })
}

/// Counters for one catalog kind.
fn kind_stats(entries: &[DbAppEntry], kind: &str) -> CatalogKindStats {
    let mut stats = CatalogKindStats::default();
    for entry in entries {
        if AppScanner::catalog_kind(&entry.category) != kind {
            continue;
        }
        stats.total += 1;
        if entry.is_hidden {
            stats.hidden += 1;
        }
        if entry.is_user_added {
            stats.user_added += 1;
        }
        if entry.name != entry.original_name {
            stats.renamed += 1;
        }
        if entry.is_favorite {
            stats.favorite += 1;
        }
    }
    stats
}
