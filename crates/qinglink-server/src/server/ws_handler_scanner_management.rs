use super::*;

pub(super) fn handle(
    state: &AppState,
    req: &WsClientRequest,
    _session_token: Option<&str>,
) -> WsServerMessage {
    match req {
        // === Scanner App Management ===
        WsClientRequest::AddScannerApp {
            name,
            exe_path,
            category,
            icon_base64,
            source,
            args,
        } => handle_add_app(
            state,
            name,
            exe_path,
            category,
            icon_base64.as_deref(),
            source.as_deref(),
            args,
        ),
        WsClientRequest::UpdateScannerApp {
            id,
            name,
            exe_path,
            icon_base64,
            args,
        } => handle_update_app(
            state,
            id,
            name,
            exe_path,
            icon_base64.as_deref(),
            args.as_deref(),
        ),
        WsClientRequest::DeleteScannerApp { id } => handle_delete_app(state, id),
        WsClientRequest::SetScannerAppHidden { id, hidden } => {
            handle_set_hidden(state, id, *hidden)
        }
        // === 常用 / 置顶：手机端的"我的库"存在服务端 ===
        //
        // 这两个标记过去放在手机的 SharedPreferences 里，导致换手机或换浏览器就
        // 丢失、管理端也看不到同一份常用列表。现在由服务端唯一拥有。
        WsClientRequest::SetScannerAppFavorite { id, favorite } => {
            handle_set_favorite(state, id, *favorite)
        }
        WsClientRequest::SetScannerAppPinned { id, pinned } => {
            handle_set_pinned(state, id, *pinned)
        }
        _ => unreachable!("request routed to the wrong handler"),
    }
}

/// 手动新增一个应用条目，必要时解析图标，成功后刷新可见目录。
fn handle_add_app(
    state: &AppState,
    name: &str,
    exe_path: &str,
    category: &str,
    icon_base64: Option<&str>,
    source: Option<&str>,
    args: &[String],
) -> WsServerMessage {
    // Resolve the icon through every available source (shortcut icon,
    // embedded resource, sibling artwork). A single folder scan no
    // longer extracts icons in bulk, so this is the one entry the user
    // just added and the extra work is invisible.
    let (icon_base64, icon_source) = match icon_base64 {
        Some(explicit) => (Some(explicit.to_string()), Some("manual".to_string())),
        None => {
            let resolution = crate::platform::resolve_icon(exe_path, None);
            let source = (resolution.icon_base64.is_some()).then(|| resolution.source.clone());
            (resolution.icon_base64, source)
        }
    };
    let app = crate::service::app_scanner::AppEntry {
        id: format!("user:{}", uuid::Uuid::new_v4()),
        name: name.to_string(),
        original_name: name.to_string(),
        exe_path: exe_path.to_string(),
        icon_base64,
        icon_path: None,
        icon_source,
        category: category.to_string(),
        source: source.unwrap_or("User").to_string(),
        args: args.to_vec(),
        is_favorite: false,
        is_pinned: false,
    };
    let db = crate::lock_unpoisoned(&state.app_db);
    match db.add_app(&app, true) {
        Ok(()) => {
            drop(db);
            reload_visible_catalog(state);
            WsServerMessage::Response(WsServerResponse::ScannerAppUpdated { id: app.id })
        }
        Err(e) => WsServerMessage::Response(WsServerResponse::error("ADD_FAILED", &e)),
    }
}

/// 更新用户添加的条目，成功后刷新可见目录。
fn handle_update_app(
    state: &AppState,
    id: &str,
    name: &str,
    exe_path: &str,
    icon_base64: Option<&str>,
    args: Option<&[String]>,
) -> WsServerMessage {
    let db = crate::lock_unpoisoned(&state.app_db);
    match db.update_app(id, name, exe_path, icon_base64, args) {
        Ok(()) => {
            drop(db);
            reload_visible_catalog(state);
            WsServerMessage::Response(WsServerResponse::ScannerAppUpdated { id: id.to_string() })
        }
        Err(e) => WsServerMessage::Response(WsServerResponse::error("UPDATE_FAILED", &e)),
    }
}

/// 删除一个条目，成功后刷新可见目录。
fn handle_delete_app(state: &AppState, id: &str) -> WsServerMessage {
    let db = crate::lock_unpoisoned(&state.app_db);
    match db.delete_app(id) {
        Ok(()) => {
            drop(db);
            reload_visible_catalog(state);
            WsServerMessage::Response(WsServerResponse::ScannerAppDeleted { id: id.to_string() })
        }
        Err(e) => WsServerMessage::Response(WsServerResponse::error("DELETE_FAILED", &e)),
    }
}

/// 屏蔽或恢复一个条目，成功后刷新可见目录。
fn handle_set_hidden(state: &AppState, id: &str, hidden: bool) -> WsServerMessage {
    let db = crate::lock_unpoisoned(&state.app_db);
    match db.set_hidden(id, hidden) {
        Ok(()) => {
            drop(db);
            reload_visible_catalog(state);
            WsServerMessage::Response(WsServerResponse::ScannerAppUpdated { id: id.to_string() })
        }
        Err(e) => WsServerMessage::Response(WsServerResponse::error("HIDE_FAILED", &e)),
    }
}

/// 加入或移出"常用"；数据库锁在刷新可见目录前已释放。
fn handle_set_favorite(state: &AppState, id: &str, favorite: bool) -> WsServerMessage {
    let result = {
        let db = crate::lock_unpoisoned(&state.app_db);
        db.set_favorite(id, favorite)
    };
    match result {
        Ok(()) => {
            reload_visible_catalog(state);
            WsServerMessage::Response(WsServerResponse::ScannerAppUpdated { id: id.to_string() })
        }
        Err(e) => WsServerMessage::Response(WsServerResponse::error("FAVORITE_FAILED", &e)),
    }
}

/// 置顶或取消置顶；数据库锁在刷新可见目录前已释放。
fn handle_set_pinned(state: &AppState, id: &str, pinned: bool) -> WsServerMessage {
    let result = {
        let db = crate::lock_unpoisoned(&state.app_db);
        db.set_pinned(id, pinned)
    };
    match result {
        Ok(()) => {
            reload_visible_catalog(state);
            WsServerMessage::Response(WsServerResponse::ScannerAppUpdated { id: id.to_string() })
        }
        Err(e) => WsServerMessage::Response(WsServerResponse::error("PIN_FAILED", &e)),
    }
}
