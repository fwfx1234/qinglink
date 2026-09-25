use super::*;

pub(super) fn handle(
    state: &AppState,
    req: &WsClientRequest,
    _session_token: Option<&str>,
) -> WsServerMessage {
    match req {
        // === Legacy scanner ===
        WsClientRequest::GetScannerApps => {
            let apps = state.app_scanner.get_or_scan();
            let response: Vec<crate::service::app_scanner::ScannerAppEntryResponse> =
                apps.into_iter().map(Into::into).collect();
            WsServerMessage::Response(WsServerResponse::ScannerApps(response))
        }
        WsClientRequest::RefreshScannerApps => {
            let apps = state.app_scanner.scan();
            let response: Vec<crate::service::app_scanner::ScannerAppEntryResponse> =
                apps.into_iter().map(Into::into).collect();
            let cached = state.app_scanner.get_or_scan();
            let db = crate::lock_unpoisoned(&state.app_db);
            let scanned_apps: Vec<_> = cached
                .iter()
                .filter(|app| !app.category.eq_ignore_ascii_case("game"))
                .cloned()
                .collect();
            let scanned_games: Vec<_> = cached
                .iter()
                .filter(|app| app.category.eq_ignore_ascii_case("game"))
                .cloned()
                .collect();
            let _ = db.sync_scanned_kind("app", &scanned_apps);
            let _ = db.sync_scanned_kind("game", &scanned_games);
            let _ = db.mark_scan_completed("app");
            let _ = db.mark_scan_completed("game");
            drop(db);
            reload_visible_catalog(state);
            WsServerMessage::Response(WsServerResponse::ScannerApps(response))
        }
        WsClientRequest::RenameScannerApp {
            id,
            original_name,
            new_name,
        } => {
            let started = std::time::Instant::now();
            tracing::info!(
                id = %id,
                original_name = %original_name,
                new_name = %new_name,
                "[rename] 收到重命名请求"
            );
            // Drop the database guard before reloading the visible catalog.
            // `reload_visible_catalog` acquires the same mutex and otherwise
            // this request deadlocks after a successful database update.
            let result = {
                tracing::info!("[rename] 1/3 等待 app_db 锁并写入数据库");
                let db = crate::lock_unpoisoned(&state.app_db);
                tracing::info!("[rename] 1/3 app_db 锁已获取，调用 rename_app");
                let result = db.rename_app(id, new_name);
                tracing::info!(
                    elapsed_ms = started.elapsed().as_millis() as u64,
                    "[rename] 1/3 数据库阶段结束"
                );
                result
            };
            match result {
                Ok(()) => {
                    tracing::info!("[rename] 2/3 app_scanner.rename 开始");
                    state.app_scanner.rename(original_name, new_name);
                    tracing::info!(
                        elapsed_ms = started.elapsed().as_millis() as u64,
                        "[rename] 2/3 app_scanner.rename 完成"
                    );
                    tracing::info!("[rename] 3/3 reload_visible_catalog 开始");
                    reload_visible_catalog(state);
                    tracing::info!(
                        elapsed_ms = started.elapsed().as_millis() as u64,
                        "[rename] 3/3 reload_visible_catalog 完成，返回成功"
                    );
                    WsServerMessage::Response(WsServerResponse::success())
                }
                Err(error) => {
                    tracing::warn!(
                        elapsed_ms = started.elapsed().as_millis() as u64,
                        %error,
                        "[rename] 数据库写入失败"
                    );
                    WsServerMessage::Response(WsServerResponse::error("RENAME_FAILED", &error))
                }
            }
        }
        WsClientRequest::LaunchScannerApp { id } => {
            if id == "steam:bigpicture" {
                return match crate::platform::launch_app("steam://open/bigpicture", &[]) {
                    Ok(()) => WsServerMessage::Response(WsServerResponse::success()),
                    Err(error) => WsServerMessage::Response(WsServerResponse::error(
                        "LAUNCH_FAILED",
                        &error.to_string(),
                    )),
                };
            }
            if let Some(app_id) = id
                .strip_prefix("steam:")
                .and_then(|value| value.parse().ok())
            {
                return match crate::platform::launch_steam_game(app_id) {
                    Ok(()) => WsServerMessage::Response(WsServerResponse::success()),
                    Err(error) => WsServerMessage::Response(WsServerResponse::error(
                        "LAUNCH_FAILED",
                        &error.to_string(),
                    )),
                };
            }
            let apps = state.app_scanner.get_or_scan();
            if let Some(app) = apps.iter().find(|a| a.id == *id) {
                if !std::path::Path::new(&app.exe_path).is_file() {
                    return WsServerMessage::Response(WsServerResponse::error(
                        "APP_NOT_FOUND",
                        "应用文件已不存在，请刷新应用目录",
                    ));
                }
                match crate::platform::launch_app(&app.exe_path, &app.args) {
                    Ok(()) => {
                        let _ = crate::lock_unpoisoned(&state.app_db).record_launch(&app.id);
                        WsServerMessage::Response(WsServerResponse::success())
                    }
                    Err(e) => WsServerMessage::Response(WsServerResponse::error(
                        "LAUNCH_FAILED",
                        &e.to_string(),
                    )),
                }
            } else {
                WsServerMessage::Response(WsServerResponse::error("NOT_FOUND", "App not found"))
            }
        }
        WsClientRequest::ScanGameDirectory { path } => {
            let started = std::time::Instant::now();
            tracing::info!(path = %path, "[scan] 开始扫描游戏目录");
            match state.app_scanner.scan_game_directory(path) {
                Ok(candidates) => {
                    tracing::info!(
                        path = %path,
                        count = candidates.len(),
                        elapsed_ms = started.elapsed().as_millis() as u64,
                        "[scan] 扫描完成"
                    );
                    WsServerMessage::Response(WsServerResponse::GameScanCandidates(
                        crate::protocol::responses::GameScanCandidatesResponse {
                            path: path.clone(),
                            candidates: candidates
                                .iter()
                                .map(|entry| crate::protocol::responses::LauncherItemSummary {
                                    id: entry.id.clone(),
                                    name: entry.name.clone(),
                                    original_name: entry.original_name.clone(),
                                    exe_path: entry.exe_path.clone(),
                                    category: entry.category.clone(),
                                    source: entry.source.clone(),
                                    args: entry.args.clone(),
                                    // 扫描阶段不提取图标：数百个 exe 逐个 shell
                                    // 取图会让请求远超手机端超时。图标改为加入
                                    // 应用库时按需提取（见 AddScannerApp）。
                                    icon_base64: None,
                                })
                                .collect(),
                        },
                    ))
                }
                Err(error) => {
                    tracing::warn!(path = %path, %error, "[scan] 扫描失败");
                    WsServerMessage::Response(WsServerResponse::error(
                        "GAME_DIR_SCAN_FAILED",
                        &error,
                    ))
                }
            }
        }
        WsClientRequest::AddScannedGames { path, exe_paths } => {
            let scanned = match state.app_scanner.scan_game_directory(path) {
                Ok(entries) => entries,
                Err(error) => {
                    return WsServerMessage::Response(WsServerResponse::error(
                        "GAME_DIR_SCAN_FAILED",
                        &error,
                    ));
                }
            };
            let selected: std::collections::HashSet<String> = exe_paths
                .iter()
                .map(|path| path.trim().replace('/', "\\").to_ascii_lowercase())
                .collect();
            let entries: Vec<_> = scanned
                .into_iter()
                .filter(|entry| {
                    selected.contains(&entry.exe_path.replace('/', "\\").to_ascii_lowercase())
                })
                .map(|mut entry| {
                    entry.id = format!("user-game:{}", uuid::Uuid::new_v4());
                    entry.source = "User".to_string();
                    entry
                })
                .collect();
            if entries.is_empty() {
                return WsServerMessage::Response(WsServerResponse::error(
                    "NO_GAMES_SELECTED",
                    "请选择至少一个有效游戏",
                ));
            }
            let db = crate::lock_unpoisoned(&state.app_db);
            let result = entries.iter().try_for_each(|entry| db.add_app(entry, true));
            drop(db);
            match result {
                Ok(()) => {
                    reload_visible_catalog(state);
                    WsServerMessage::Response(WsServerResponse::success())
                }
                Err(error) => {
                    WsServerMessage::Response(WsServerResponse::error("ADD_GAMES_FAILED", &error))
                }
            }
        }

        _ => unreachable!("request routed to the wrong handler"),
    }
}
