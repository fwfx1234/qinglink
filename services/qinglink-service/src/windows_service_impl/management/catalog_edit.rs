//! 目录写操作：重命名、隐藏、收藏、置顶、删除。
//!
//! 每个改动落库后都必须广播一次目录变更，否则已连接的移动端不会重新拉取列表。

use crate::windows_service_impl::management::lookup::{find_entry, validate_display_name};
use crate::windows_service_impl::management::record::{CommandError, CommandResult, value};
use qinglink_server::AppState;

/// Tell connected mobile clients that the catalog changed.
///
/// Both kinds are announced: a hide or delete changes the visible set of the
/// kind the entry belongs to, and clients re-list rather than diff.
pub(super) fn publish_catalog_change(state: &AppState) {
    qinglink_server::server::reload_visible_catalog(state);
    state.announce_catalog_change("app");
    state.announce_catalog_change("game");
}

/// Rename an entry, or clear the override when the name is empty.
pub(super) fn rename_app(state: &AppState, id: &str, name: &str) -> CommandResult {
    let entry = find_entry(state, id)?;
    // An empty (or whitespace-only) name clears the override and
    // restores the scanned name on the next read, because every listing
    // coalesces NULLIF(display_name, '') back to `name`.
    let name = validate_display_name(name)?;
    {
        let db = state
            .app_db
            .lock()
            .map_err(|_| CommandError::internal("catalog unavailable"))?;
        if db
            .rename_app(&entry.id, &name)
            .map_err(CommandError::internal)?
            == 0
        {
            return Err(CommandError::not_found());
        }
    }
    publish_catalog_change(state);
    let entry = find_entry(state, &entry.id)?;
    value(serde_json::json!({
        "id": entry.id,
        "name": entry.name,
        "renamed": entry.name != entry.original_name,
    }))
}

/// Hide or unhide an entry from the mobile listings.
pub(super) fn set_app_hidden(state: &AppState, id: &str, hidden: bool) -> CommandResult {
    let entry = find_entry(state, id)?;
    {
        let db = state
            .app_db
            .lock()
            .map_err(|_| CommandError::internal("catalog unavailable"))?;
        if db
            .set_hidden(&entry.id, hidden)
            .map_err(CommandError::internal)?
            == 0
        {
            return Err(CommandError::not_found());
        }
    }
    publish_catalog_change(state);
    value(serde_json::json!({"id": entry.id, "hidden": hidden}))
}

/// Toggle the favorite flag of an entry.
pub(super) fn set_app_favorite(state: &AppState, id: &str, favorite: bool) -> CommandResult {
    let entry = find_entry(state, id)?;
    {
        let db = state
            .app_db
            .lock()
            .map_err(|_| CommandError::internal("catalog unavailable"))?;
        if db
            .set_favorite(&entry.id, favorite)
            .map_err(CommandError::internal)?
            == 0
        {
            return Err(CommandError::not_found());
        }
    }
    publish_catalog_change(state);
    value(serde_json::json!({"id": entry.id, "favorite": favorite}))
}

/// Toggle the pinned flag of an entry.
pub(super) fn set_app_pinned(state: &AppState, id: &str, pinned: bool) -> CommandResult {
    let entry = find_entry(state, id)?;
    {
        let db = state
            .app_db
            .lock()
            .map_err(|_| CommandError::internal("catalog unavailable"))?;
        if db
            .set_pinned(&entry.id, pinned)
            .map_err(CommandError::internal)?
            == 0
        {
            return Err(CommandError::not_found());
        }
    }
    publish_catalog_change(state);
    value(serde_json::json!({"id": entry.id, "pinned": pinned}))
}

/// Soft-delete an entry from the catalog.
pub(super) fn delete_app(state: &AppState, id: &str) -> CommandResult {
    let entry = find_entry(state, id)?;
    {
        let db = state
            .app_db
            .lock()
            .map_err(|_| CommandError::internal("catalog unavailable"))?;
        if db.delete_app(&entry.id).map_err(CommandError::internal)? == 0 {
            return Err(CommandError::not_found());
        }
    }
    publish_catalog_change(state);
    value(serde_json::json!({"deleted":true,"id":entry.id}))
}
