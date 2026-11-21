//! 应用图标：读取缓存、重解析候选、列出候选文件。
//!
//! 只有用户显式指定的图标文件才会写入条目的 `icon_path` 偏好；按需读取时解析出的
//! 自动结果只缓存「载荷 + 来源」，因此后续刷新仍按自动链重新解析，不会被误判成手动来源。

use crate::windows_service_impl::management::catalog_edit::publish_catalog_change;
use crate::windows_service_impl::management::lookup::{find_entry, steam_artwork};
use crate::windows_service_impl::management::record::{CommandError, CommandResult, value};
use qinglink_protocol::service_ipc::{
    AppIcon, AppIconCandidate, AppIconCandidates, AppIconUpdate, icon_fits_budget,
};
use qinglink_server::AppState;
use qinglink_server::service::app_db::DbAppEntry;

/// The bitmap chosen for a refresh, plus where it came from and what to store.
struct IconChoice {
    icon_base64: Option<String>,
    source: String,
    resolved_path: Option<String>,
    stored_path: Option<String>,
}

/// Return the stored icon, falling back to a fresh extraction.
///
/// 解析成功的结果会被缓存（载荷 + 来源，不写 `icon_path`），这样同一个条目在
/// 后续请求里拿到的是同一份载荷：每次重读文件资源时偶发的提取失败会让图标
/// "时而有时而没有"。
pub(super) fn get_app_icon(state: &AppState, id: &str) -> CommandResult {
    let entry = find_entry(state, id)?;
    let icon_base64 = match entry.icon_base64.clone() {
        Some(icon) => Some(icon),
        None => qinglink_server::server::icon_service::resolve_and_remember(
            state,
            &entry.id,
            &entry.exe_path,
            entry.icon_path.as_deref(),
            &entry.source,
        ),
    };
    // Reject oversized payloads rather than letting the IPC frame grow
    // without bound; the UI falls back to a placeholder. The extractor
    // already stays inside the budget, so this is only a safety net.
    let icon_base64 = icon_base64.filter(|icon| icon_fits_budget(icon));
    value(AppIcon {
        id: entry.id,
        icon_base64,
    })
}

/// Re-resolve an entry's icon, optionally from a user-chosen file.
pub(super) fn refresh_app_icon(
    state: &AppState,
    id: &str,
    icon_path: Option<&str>,
) -> CommandResult {
    let entry = find_entry(state, id)?;
    // `icon_path` is the user's explicit choice. Omitting it means
    // "重新扫描": ignore and clear any stored preference, then let the
    // scanner's own artwork (Steam covers) win before the executable,
    // which is only a guess.
    let requested = icon_path.map(str::trim).filter(|value| !value.is_empty());
    if let Some(requested) = requested
        && !std::path::Path::new(requested).is_file()
    {
        return Err(CommandError::new(
            "NOT_FOUND",
            format!("图标文件不存在：{requested}"),
        ));
    }
    let choice = resolve_icon_choice(state, &entry, requested);
    let icon_base64 = choice.icon_base64.filter(|icon| icon_fits_budget(icon));
    {
        let db = state
            .app_db
            .lock()
            .map_err(|_| CommandError::internal("catalog unavailable"))?;
        // Only an explicit choice is stored as the entry's preference.
        // Automatic discovery is reproducible, and pinning it would make
        // a later refresh report `manual` and keep a generic icon.
        db.set_icon(
            &entry.id,
            icon_base64.as_deref(),
            Some(&choice.source),
            choice.stored_path.as_deref(),
        )
        .map_err(CommandError::internal)?;
    }
    publish_catalog_change(state);
    value(AppIconUpdate {
        id: entry.id,
        icon_base64,
        source: choice.source,
        icon_path: choice.resolved_path.or(choice.stored_path),
    })
}

/// Resolve the bitmap for a refresh: explicit file first, then Steam art, then
/// the automatic extraction chain.
fn resolve_icon_choice(
    state: &AppState,
    entry: &DbAppEntry,
    requested: Option<&str>,
) -> IconChoice {
    match requested {
        Some(requested) => {
            let resolution =
                qinglink_server::platform::resolve_icon(&entry.exe_path, Some(requested));
            // Only claim "manual" when the user's file actually produced
            // the bitmap; otherwise the automatic chain won.
            let explicit_won = resolution.source == "manual";
            IconChoice {
                icon_base64: resolution.icon_base64,
                source: resolution.source,
                resolved_path: resolution.source_path,
                stored_path: explicit_won.then_some(requested.to_string()),
            }
        }
        None => {
            if entry.source.eq_ignore_ascii_case("steam")
                && let Some(artwork) = steam_artwork(state, &entry.id)
                && let Some(icon) = qinglink_server::platform::image_file_to_icon_base64(&artwork)
            {
                IconChoice {
                    icon_base64: Some(icon),
                    source: "steam".to_string(),
                    resolved_path: Some(artwork),
                    stored_path: None,
                }
            } else {
                let resolution = qinglink_server::platform::resolve_icon(&entry.exe_path, None);
                IconChoice {
                    icon_base64: resolution.icon_base64,
                    source: resolution.source,
                    resolved_path: resolution.source_path,
                    stored_path: None,
                }
            }
        }
    }
}

/// Enumerate the icon files the entry's executable offers.
pub(super) fn list_app_icons(state: &AppState, id: &str) -> CommandResult {
    let entry = find_entry(state, id)?;
    let candidates = qinglink_server::platform::list_icon_candidates(&entry.exe_path)
        .into_iter()
        .map(|candidate| AppIconCandidate {
            path: candidate.path,
            kind: candidate.kind,
            size_bytes: candidate.size_bytes,
        })
        .collect();
    value(AppIconCandidates {
        id: entry.id,
        candidates,
    })
}
