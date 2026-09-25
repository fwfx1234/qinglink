use super::*;

pub(super) fn handle(state: &AppState, req: &WsClientRequest, _session_token: Option<&str>) -> WsServerMessage {
    match req {
        // === Mobile launch directory ===
        WsClientRequest::ListLauncherItems {
            kind,
            query,
            page,
            page_size,
        } => {
            let result =
                state
                    .app_scanner
                    .list_summaries(kind, query.as_deref(), *page, *page_size);
            WsServerMessage::Response(WsServerResponse::LauncherItems(result))
        }
        WsClientRequest::GetRecentLauncherItems { kind } => {
            let ids = crate::lock_unpoisoned(&state.app_db)
                .recent_ids(12)
                .unwrap_or_default();
            let items = state.app_scanner.recent_summaries(kind, &ids);
            WsServerMessage::Response(WsServerResponse::RecentLauncherItems(items))
        }
        WsClientRequest::GetLauncherIcon { id } => {
            let icon_base64 = state.app_scanner.icon_base64(id);
            WsServerMessage::Response(WsServerResponse::LauncherIcon(
                crate::protocol::responses::LauncherIconResponse {
                    id: id.clone(),
                    icon_base64,
                },
            ))
        }
        WsClientRequest::RefreshScannerAppIcon { id } => {
            let Some(app) = state
                .app_scanner
                .get_or_scan()
                .into_iter()
                .find(|app| app.id == *id)
            else {
                return WsServerMessage::Response(WsServerResponse::error(
                    "NOT_FOUND",
                    "App not found",
                ));
            };
            // Re-extract from the executable and persist so the new icon
            // survives the in-memory cache and app restarts.
            let icon_base64 = crate::platform::extract_icon_base64(&app.exe_path);
            let save_error = {
                let db = crate::lock_unpoisoned(&state.app_db);
                db.update_app(
                    id,
                    &app.name,
                    &app.exe_path,
                    icon_base64.as_deref(),
                    None,
                )
                .err()
            };
            if let Some(error) = save_error {
                return WsServerMessage::Response(WsServerResponse::error(
                    "ICON_UPDATE_FAILED",
                    &error,
                ));
            }
            reload_visible_catalog(state);
            WsServerMessage::Response(WsServerResponse::LauncherIcon(
                crate::protocol::responses::LauncherIconResponse {
                    id: id.clone(),
                    icon_base64,
                },
            ))
        }
        WsClientRequest::RefreshLauncherCatalog { kind } => {
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

        _ => unreachable!("request routed to the wrong handler"),
    }
}


