use super::*;

pub(super) fn handle(
    state: &AppState,
    req: &WsClientRequest,
    _session_token: Option<&str>,
) -> WsServerMessage {
    match req {
        // === Mobile launch directory ===
        WsClientRequest::ListLauncherItems {
            kind,
            query,
            page,
            page_size,
        } => handle_catalog_page(state, kind, query.as_deref(), *page, *page_size),
        WsClientRequest::GetRecentLauncherItems { kind } => handle_recent_items(state, kind),
        WsClientRequest::GetLauncherIcon { id } => handle_launcher_icon(state, id),
        // === Per-entry icon management ===
        WsClientRequest::RefreshScannerAppIcon { id, icon_path }
        | WsClientRequest::SetScannerAppIcon { id, icon_path } => {
            handle_icon_update(state, id, icon_path.as_deref())
        }
        WsClientRequest::ListScannerAppIcons { id } => handle_icon_candidates(state, id),
        // === Blocked entries ===
        //
        // The phone's library shows curated entries; blocked ones stay on the
        // PC and are listed here so the user can restore them.
        WsClientRequest::ListBlockedScannerApps => handle_blocked_entries(state),
        WsClientRequest::RefreshLauncherCatalog { kind } => handle_catalog_scan(state, kind),

        _ => unreachable!("request routed to the wrong handler"),
    }
}

/// 分页返回启动器条目摘要。
fn handle_catalog_page(
    state: &AppState,
    kind: &str,
    query: Option<&str>,
    page: usize,
    page_size: usize,
) -> WsServerMessage {
    let result = state
        .app_scanner
        .list_summaries(kind, query, page, page_size);
    WsServerMessage::Response(WsServerResponse::LauncherItems(result))
}

/// 返回最近启动过的条目摘要（最多 12 条）。
fn handle_recent_items(state: &AppState, kind: &str) -> WsServerMessage {
    let ids = crate::lock_unpoisoned(&state.app_db)
        .recent_ids(12)
        .unwrap_or_default();
    let items = state.app_scanner.recent_summaries(kind, &ids);
    WsServerMessage::Response(WsServerResponse::RecentLauncherItems(items))
}

/// 返回单个条目的图标；内存目录未命中时回退为直接查库解析。
fn handle_launcher_icon(state: &AppState, id: &str) -> WsServerMessage {
    // The in-memory catalog only holds visible entries, so blocked ones
    // (listed in the 已屏蔽 view) fall back to a direct lookup.
    let icon_base64 = state.app_scanner.icon_base64(id).or_else(|| {
        let entry = crate::lock_unpoisoned(&state.app_db)
            .get_all_apps()
            .ok()?
            .into_iter()
            .find(|entry| entry.id == *id)?;
        crate::platform::resolve_icon(&entry.exe_path, entry.icon_path.as_deref()).icon_base64
    });
    WsServerMessage::Response(WsServerResponse::LauncherIcon(
        crate::protocol::responses::LauncherIconResponse {
            id: id.to_string(),
            icon_base64,
        },
    ))
}

/// 重新解析并持久化单个条目的图标（刷新与显式设置共用）。
fn handle_icon_update(state: &AppState, id: &str, icon_path: Option<&str>) -> WsServerMessage {
    match crate::server::icon_service::resolve_and_store(state, id, icon_path) {
        Ok(update) => WsServerMessage::Response(WsServerResponse::IconUpdate(update)),
        Err(error) => {
            WsServerMessage::Response(WsServerResponse::error("ICON_UPDATE_FAILED", &error))
        }
    }
}

/// 列出该条目可选的图标文件候选。
fn handle_icon_candidates(state: &AppState, id: &str) -> WsServerMessage {
    match crate::server::icon_service::candidates(state, id) {
        Ok((id, candidates)) => WsServerMessage::Response(WsServerResponse::IconCandidates(
            crate::protocol::responses::IconCandidatesResponse { id, candidates },
        )),
        Err(error) => WsServerMessage::Response(WsServerResponse::error("NOT_FOUND", &error)),
    }
}

/// 列出被屏蔽的条目，按名称排序后整表返回。
fn handle_blocked_entries(state: &AppState) -> WsServerMessage {
    let entries = match crate::lock_unpoisoned(&state.app_db).get_all_apps() {
        Ok(entries) => entries,
        Err(error) => {
            return WsServerMessage::Response(WsServerResponse::error("LIST_FAILED", &error));
        }
    };
    let mut items: Vec<crate::protocol::responses::LauncherItemSummary> = entries
        .into_iter()
        .filter(|entry| entry.is_hidden)
        .map(blocked_summary)
        .collect();
    items.sort_by(|left, right| {
        left.name
            .to_lowercase()
            .cmp(&right.name.to_lowercase())
            .then_with(|| left.id.cmp(&right.id))
    });
    let total = items.len();
    WsServerMessage::Response(WsServerResponse::BlockedLauncherItems(
        crate::protocol::responses::BlockedLauncherItemsResponse { items, total },
    ))
}

/// 手动触发一次目录扫描；游戏运行中或线程创建失败时拒绝。
fn handle_catalog_scan(state: &AppState, kind: &str) -> WsServerMessage {
    if scanning_is_paused(state) {
        return WsServerMessage::Response(WsServerResponse::error(
            "GAME_ACTIVE",
            "检测到游戏运行，扫描已延后",
        ));
    }
    let kind = if kind.eq_ignore_ascii_case("game") {
        "game".to_string()
    } else {
        "app".to_string()
    };
    let state = state.clone();
    match std::thread::Builder::new()
        .name(format!("qinglink-scan-{kind}"))
        .spawn(move || {
            let _scan_permit = crate::lock_unpoisoned(&state.scan_gate);
            if let Err(error) = crate::server::scan_catalog_kind(&state, &kind) {
                tracing::error!(%error, kind, "[远程控制] 手动扫描失败");
            }
        }) {
        Ok(_) => WsServerMessage::Response(WsServerResponse::success()),
        Err(error) => WsServerMessage::Response(WsServerResponse::error(
            "SCAN_START_FAILED",
            &error.to_string(),
        )),
    }
}

/// Present one blocked catalog row in the same shape as a normal list entry.
fn blocked_summary(
    entry: crate::service::app_db::DbAppEntry,
) -> crate::protocol::responses::LauncherItemSummary {
    crate::protocol::responses::LauncherItemSummary {
        id: entry.id,
        name: entry.name,
        original_name: entry.original_name,
        exe_path: entry.exe_path,
        category: entry.category,
        source: entry.source,
        args: entry.args,
        icon_source: entry.icon_source,
        size_bytes: None,
        recommended: None,
        favorite: entry.is_favorite,
        pinned: entry.is_pinned,
        icon_base64: None,
    }
}
