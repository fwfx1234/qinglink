use super::*;

pub(super) fn handle(
    state: &AppState,
    req: &WsClientRequest,
    _session_token: Option<&str>,
) -> WsServerMessage {
    match req {
        // === Legacy scanner ===
        WsClientRequest::GetScannerApps => list_scanner_apps(state),
        WsClientRequest::RefreshScannerApps => refresh_scanner_apps(state),
        WsClientRequest::RenameScannerApp {
            id,
            original_name,
            new_name,
        } => rename_scanner_app(state, id, original_name, new_name),
        WsClientRequest::LaunchScannerApp { id } => launch_scanner_app(state, id),
        WsClientRequest::ScanGameDirectory { path, kind } => scan_game_directory(state, path, kind),
        WsClientRequest::AddScannedGames {
            path,
            exe_paths,
            names,
            kind,
        } => add_scanned_games(state, path, exe_paths, names, kind),

        _ => unreachable!("request routed to the wrong handler"),
    }
}

/// 返回扫描器缓存中的全部条目。
fn list_scanner_apps(state: &AppState) -> WsServerMessage {
    let apps = state.app_scanner.get_or_scan();
    let response: Vec<crate::service::app_scanner::ScannerAppEntryResponse> =
        apps.into_iter().map(Into::into).collect();
    WsServerMessage::Response(WsServerResponse::ScannerApps(response))
}

/// 重新扫描本机应用，按类别写入数据库后刷新可见目录。
fn refresh_scanner_apps(state: &AppState) -> WsServerMessage {
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

/// 重命名条目：先写数据库，再改扫描器缓存，最后刷新可见目录。
fn rename_scanner_app(
    state: &AppState,
    id: &str,
    original_name: &str,
    new_name: &str,
) -> WsServerMessage {
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

/// 启动条目：Steam 快捷方式、Steam 游戏与普通可执行文件分别处理。
fn launch_scanner_app(state: &AppState, id: &str) -> WsServerMessage {
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
        let app = state
            .app_scanner
            .get_or_scan()
            .into_iter()
            .find(|app| app.id == id);
        let result = match app.as_ref() {
            Some(app) if app.launch_mode.is_some() => state.launch_display.launch(state, app),
            _ => crate::platform::launch_steam_game(app_id),
        };
        return match result {
            Ok(()) => WsServerMessage::Response(WsServerResponse::success()),
            Err(error) => WsServerMessage::Response(WsServerResponse::error(
                "LAUNCH_FAILED",
                &error.to_string(),
            )),
        };
    }
    let apps = state.app_scanner.get_or_scan();
    if let Some(app) = apps.iter().find(|a| a.id == id) {
        if !std::path::Path::new(&app.exe_path).is_file() {
            return WsServerMessage::Response(WsServerResponse::error(
                "APP_NOT_FOUND",
                "应用文件已不存在，请刷新应用目录",
            ));
        }
        match state.launch_display.launch(state, app) {
            Ok(()) => {
                let _ = crate::lock_unpoisoned(&state.app_db).record_launch(&app.id);
                WsServerMessage::Response(WsServerResponse::success())
            }
            Err(e) => {
                WsServerMessage::Response(WsServerResponse::error("LAUNCH_FAILED", &e.to_string()))
            }
        }
    } else {
        WsServerMessage::Response(WsServerResponse::error("NOT_FOUND", "App not found"))
    }
}

/// 扫描指定目录并把候选可执行文件返回给客户端。
fn scan_game_directory(state: &AppState, path: &str, kind: &str) -> WsServerMessage {
    let started = std::time::Instant::now();
    let kind = if kind.eq_ignore_ascii_case("game") {
        "game"
    } else {
        "app"
    };
    tracing::info!(path = %path, kind, "[scan] 开始扫描目录");
    match state.app_scanner.scan_executable_directory(path, kind) {
        Ok(candidates) => {
            tracing::info!(
                path = %path,
                count = candidates.len(),
                recommended = candidates.iter().filter(|item| item.recommended).count(),
                elapsed_ms = started.elapsed().as_millis() as u64,
                "[scan] 扫描完成"
            );
            WsServerMessage::Response(WsServerResponse::GameScanCandidates(
                crate::protocol::responses::GameScanCandidatesResponse {
                    path: path.to_string(),
                    candidates: candidates.iter().map(scan_candidate_summary).collect(),
                },
            ))
        }
        Err(error) => {
            tracing::warn!(path = %path, %error, "[scan] 扫描失败");
            WsServerMessage::Response(WsServerResponse::error("GAME_DIR_SCAN_FAILED", &error))
        }
    }
}

/// 把单个扫描候选转换为下发给客户端的条目摘要。
fn scan_candidate_summary(
    candidate: &crate::service::app_scanner::ScanCandidate,
) -> crate::protocol::responses::LauncherItemSummary {
    let entry = &candidate.entry;
    crate::protocol::responses::LauncherItemSummary {
        id: entry.id.clone(),
        name: entry.name.clone(),
        original_name: entry.original_name.clone(),
        exe_path: entry.exe_path.clone(),
        category: entry.category.clone(),
        source: entry.source.clone(),
        args: entry.args.clone(),
        icon_source: None,
        size_bytes: Some(candidate.size_bytes),
        recommended: Some(candidate.recommended),
        favorite: false,
        pinned: false,
        launch_mode: None,
        // 扫描阶段不提取图标：数百个 exe 逐个 shell
        // 取图会让请求远超手机端超时。图标改为按需
        // 解析（见 icon_service）。
        icon_base64: None,
    }
}

/// 保存用户在扫描结果中勾选的程序。
fn add_scanned_games(
    state: &AppState,
    path: &str,
    exe_paths: &[String],
    names: &[String],
    kind: &str,
) -> WsServerMessage {
    let category = if kind.eq_ignore_ascii_case("app") {
        "Application"
    } else {
        "Game"
    };
    let root = fold_directory(path);
    let entries = selected_app_entries(category, &root, exe_paths, names);
    if entries.is_empty() {
        return WsServerMessage::Response(WsServerResponse::error(
            "NO_ITEMS_SELECTED",
            "请选择至少一个有效程序",
        ));
    }
    let db = crate::lock_unpoisoned(&state.app_db);
    let result = entries.iter().try_for_each(|entry| db.add_app(entry, true));
    drop(db);
    match result {
        Ok(()) => {
            reload_visible_catalog(state);
            let response = entries.into_iter().map(Into::into).collect();
            WsServerMessage::Response(WsServerResponse::ScannerApps(response))
        }
        Err(error) => {
            WsServerMessage::Response(WsServerResponse::error("ADD_GAMES_FAILED", &error))
        }
    }
}

/// 从客户端回传的候选中筛选出确实位于已扫描目录内的条目。
fn selected_app_entries(
    category: &str,
    root: &str,
    exe_paths: &[String],
    names: &[String],
) -> Vec<crate::service::app_scanner::AppEntry> {
    let display_names: std::collections::HashMap<String, String> = exe_paths
        .iter()
        .zip(names.iter())
        .map(|(path, name)| (fold_path(path), name.trim().to_string()))
        .filter(|(_, name)| !name.is_empty())
        .collect();

    // The client sends back the candidates it reviewed, so the selection
    // is trusted only inside the folder the user actually scanned — and
    // the folder is not scanned a second time. The old implementation
    // rescanned and re-matched, which doubled the request time and could
    // silently drop a selection when the second scan hit its budget.
    exe_paths
        .iter()
        .map(|raw| raw.trim())
        .filter(|raw| !raw.is_empty())
        .filter(|raw| std::path::Path::new(raw).is_file())
        // Must stay inside the folder the user actually scanned. The
        // separator matters: without it `D:\Games2` would count as being
        // inside `D:\Games`.
        .filter(|raw| root.is_empty() || is_inside(root, raw))
        .map(|exe_path| user_app_entry(exe_path, category, &display_names))
        .collect()
}

/// 依据扫描候选与客户端提交的显示名构造一条用户来源的条目。
fn user_app_entry(
    exe_path: &str,
    category: &str,
    display_names: &std::collections::HashMap<String, String>,
) -> crate::service::app_scanner::AppEntry {
    let fallback = std::path::Path::new(exe_path)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("未命名程序")
        .to_string();
    let name = display_names
        .get(&fold_path(exe_path))
        .cloned()
        .unwrap_or(fallback);
    crate::service::app_scanner::AppEntry {
        id: format!("user:{}", uuid::Uuid::new_v4()),
        name: name.clone(),
        original_name: name,
        exe_path: exe_path.to_string(),
        icon_base64: None,
        icon_path: None,
        icon_source: None,
        category: category.to_string(),
        source: "User".to_string(),
        args: Vec::new(),
        is_favorite: false,
        is_pinned: false,
        launch_mode: None,
    }
}

/// Normalize a directory path for the "is this file inside the scanned folder"
/// check: lower case, backslashes, no trailing separator.
fn fold_directory(path: &str) -> String {
    path.trim()
        .trim_matches('"')
        .replace('/', "\\")
        .trim_end_matches('\\')
        .to_ascii_lowercase()
}

fn fold_path(path: &str) -> String {
    path.trim()
        .trim_matches('"')
        .replace('/', "\\")
        .to_ascii_lowercase()
}

/// Whether `path` lives inside the already-folded `root` directory.
fn is_inside(root: &str, path: &str) -> bool {
    let folded = fold_path(path);
    folded
        .strip_prefix(root)
        .is_some_and(|rest| rest.starts_with('\\'))
}
