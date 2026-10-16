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
        WsClientRequest::UpdateScannerApp(query) => handle_update_app(state, query),
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
        // A client-supplied bitmap (phone picking one from its gallery) is
        // normalised to the same canonical PNG every other source produces.
        Some(explicit) => match normalized_icon(explicit) {
            Some(normalized) => (Some(normalized), Some("manual".to_string())),
            None => return invalid_icon(),
        },
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
        launch_mode: None,
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

/// 更新条目，成功后刷新可见目录。
fn handle_update_app(
    state: &AppState,
    query: &crate::protocol::requests::UpdateScannerAppQuery,
) -> WsServerMessage {
    if let Some(mode) = query.launch_mode.as_ref()
        && !launch_mode_is_supported(mode)
    {
        return WsServerMessage::Response(WsServerResponse::error(
            "INVALID_DISPLAY_MODE",
            "主显示器当前不支持该分辨率或刷新率，请重新选择",
        ));
    }
    // 客户端上传的图标先规范化，再把来源记成 `manual`；只更新其他字段时两者都为
    // `None`，SQL 的 COALESCE 会保留原值。
    let (icon_payload, icon_source) = match query.icon_base64.as_deref() {
        Some(explicit) => match normalized_icon(explicit) {
            Some(normalized) => (Some(normalized), Some("manual")),
            None => return invalid_icon(),
        },
        None => (None, None),
    };
    let db = crate::lock_unpoisoned(&state.app_db);
    if query.launch_mode.is_some() || query.clear_launch_mode {
        let is_game = db
            .get_app(&query.id)
            .ok()
            .flatten()
            .is_some_and(|entry| entry.category.eq_ignore_ascii_case("game"));
        if !is_game {
            return WsServerMessage::Response(WsServerResponse::error(
                "INVALID_DISPLAY_MODE",
                "启动显示模式只能配置到游戏条目",
            ));
        }
    }
    let result = db
        .update_app(
            &query.id,
            &query.name,
            &query.exe_path,
            icon_payload.as_deref(),
            icon_source,
            query.args.as_deref(),
        )
        .and_then(|()| {
            if query.clear_launch_mode {
                db.set_launch_mode(&query.id, None)
            } else if let Some(mode) = query.launch_mode.as_ref() {
                db.set_launch_mode(&query.id, Some(mode))
            } else {
                Ok(())
            }
        });
    match result {
        Ok(()) => {
            drop(db);
            reload_visible_catalog(state);
            WsServerMessage::Response(WsServerResponse::ScannerAppUpdated {
                id: query.id.clone(),
            })
        }
        Err(e) => WsServerMessage::Response(WsServerResponse::error("UPDATE_FAILED", &e)),
    }
}

fn launch_mode_is_supported(mode: &crate::service::app_scanner::LaunchDisplayMode) -> bool {
    if mode.width == 0 || mode.height == 0 || mode.refresh_millihz == 0 {
        return false;
    }
    crate::platform::get_displays()
        .ok()
        .and_then(|displays| displays.into_iter().find(|display| display.primary))
        .is_some_and(|display| {
            display.modes.iter().any(|candidate| {
                candidate.width == mode.width
                    && candidate.height == mode.height
                    && candidate.refresh_millihz == mode.refresh_millihz
            })
        })
}

/// 把客户端上传的位图规范化为目录存储的 PNG 载荷；无法解码时返回 `None`。
fn normalized_icon(payload: &str) -> Option<String> {
    crate::platform::base64_image_to_icon_base64(payload)
}

/// 客户端上传的图标无法解码时的统一应答。
fn invalid_icon() -> WsServerMessage {
    WsServerMessage::Response(WsServerResponse::error(
        "INVALID_ICON",
        "图标图片无法解析，请换一张 png/jpg 图片",
    ))
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
