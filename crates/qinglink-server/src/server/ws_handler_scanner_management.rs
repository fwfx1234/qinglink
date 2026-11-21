use super::*;
use crate::service::app_catalog::{self, CatalogError};

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
        _ => super::unrouted_request(),
    }
}

/// 把校验器的失败原因渲染成协议的错误信封。
fn catalog_error(error: CatalogError) -> WsServerMessage {
    WsServerMessage::Response(WsServerResponse::error(error.code, &error.message))
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
    // 与桌面管理 IPC 走同一组校验器。缺了这一步，手机端就是唯一能把任意
    // `exe_path`（含空串、含不存在的路径）写进目录的入口——而目录里出现重复
    // 或空路径之后，任何"同一 exe 唯一"的约束都无法再建立。
    let exe_path = match app_catalog::validate_app_path(exe_path) {
        Ok(path) => path,
        Err(error) => return catalog_error(error),
    };
    let name = if name.trim().is_empty() {
        app_catalog::default_app_name(&exe_path)
    } else {
        match app_catalog::validate_display_name(name) {
            Ok(name) => name,
            Err(error) => return catalog_error(error),
        }
    };
    if name.is_empty() {
        return catalog_error(CatalogError::bad_request(
            "无法从路径推断名称，请填写显示名",
        ));
    }
    // 稳定 id 让"同一 exe 加两次"变成显式冲突而不是第二行。这一步必须挡在
    // 落库之前：`add_app` 用的是 `INSERT OR REPLACE`，真的命中已存在的主键时
    // 会删行重插，把用户的收藏 / 置顶 / 隐藏 / 自定义名静默清空。
    let id = app_catalog::app_id_for_path(&exe_path);
    match state.admin_catalog() {
        Ok(entries) if entries.iter().any(|entry| entry.id == id) => {
            return catalog_error(CatalogError::conflict("该可执行文件已在目录中"));
        }
        Ok(_) => {}
        Err(error) => return catalog_error(CatalogError::internal(error)),
    }
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
            let resolution = crate::platform::resolve_icon(&exe_path, None);
            let source = (resolution.icon_base64.is_some()).then(|| resolution.source.clone());
            (resolution.icon_base64, source)
        }
    };
    let app = crate::service::app_scanner::AppEntry {
        id: id.clone(),
        name: name.clone(),
        original_name: name,
        exe_path,
        icon_base64,
        icon_path: None,
        icon_source,
        category: crate::server::normalize_catalog_kind(category).to_string(),
        source: source.unwrap_or("User").to_string(),
        args: app_catalog::sanitize_args(args),
        is_favorite: false,
        is_pinned: false,
        launch_mode: None,
    };
    let db = crate::lock_unpoisoned(&state.app_db);
    match db.add_app(&app, true) {
        Ok(()) => {
            drop(db);
            reload_visible_catalog(state);
            WsServerMessage::Response(WsServerResponse::ScannerAppUpdated { id })
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
    // 空名称合法，表示"恢复扫描得到的名字"（列表读取时会把空串折回原始名）。
    let name = match app_catalog::validate_display_name(&query.name) {
        Ok(name) => name,
        Err(error) => return catalog_error(error),
    };
    // 修改既有条目只校验路径形状、不要求文件仍在：条目可能比磁盘上的程序活得
    // 更久（先卸载、后重命名或删除是常见顺序）。
    let exe_path = match app_catalog::validate_app_path_input(&query.exe_path) {
        Ok(path) => path,
        Err(error) => return catalog_error(error),
    };
    let args = query.args.as_deref().map(app_catalog::sanitize_args);
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
    let updated = db
        .update_app(
            &query.id,
            &name,
            &exe_path,
            icon_payload.as_deref(),
            icon_source,
            args.as_deref(),
        )
        .and_then(|updated| {
            // 0 行说明 id 根本不存在；继续写启动模式只会让"改了但没生效"
            // 以成功的形态回到手机端。
            if updated == 0 {
                return Ok(0);
            }
            if query.clear_launch_mode {
                db.set_launch_mode(&query.id, None).map(|()| updated)
            } else if let Some(mode) = query.launch_mode.as_ref() {
                db.set_launch_mode(&query.id, Some(mode)).map(|()| updated)
            } else {
                Ok(updated)
            }
        });
    match updated {
        Ok(0) => {
            drop(db);
            catalog_error(CatalogError::not_found("目录中不存在该条目"))
        }
        Ok(_) => {
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
    let id = match app_catalog::validate_entry_id(id) {
        Ok(id) => id,
        Err(error) => return catalog_error(error),
    };
    let deleted = {
        let db = crate::lock_unpoisoned(&state.app_db);
        db.delete_app(&id)
    };
    match deleted {
        Ok(0) => catalog_error(CatalogError::not_found("目录中不存在该条目")),
        Ok(_) => {
            reload_visible_catalog(state);
            WsServerMessage::Response(WsServerResponse::ScannerAppDeleted { id })
        }
        Err(e) => WsServerMessage::Response(WsServerResponse::error("DELETE_FAILED", &e)),
    }
}

/// 屏蔽或恢复一个条目，成功后刷新可见目录。
fn handle_set_hidden(state: &AppState, id: &str, hidden: bool) -> WsServerMessage {
    let id = match app_catalog::validate_entry_id(id) {
        Ok(id) => id,
        Err(error) => return catalog_error(error),
    };
    let updated = {
        let db = crate::lock_unpoisoned(&state.app_db);
        db.set_hidden(&id, hidden)
    };
    match updated {
        Ok(0) => catalog_error(CatalogError::not_found("目录中不存在该条目")),
        Ok(_) => {
            reload_visible_catalog(state);
            WsServerMessage::Response(WsServerResponse::ScannerAppUpdated { id })
        }
        Err(e) => WsServerMessage::Response(WsServerResponse::error("HIDE_FAILED", &e)),
    }
}

/// 加入或移出"常用"；数据库锁在刷新可见目录前已释放。
fn handle_set_favorite(state: &AppState, id: &str, favorite: bool) -> WsServerMessage {
    let id = match app_catalog::validate_entry_id(id) {
        Ok(id) => id,
        Err(error) => return catalog_error(error),
    };
    let updated = {
        let db = crate::lock_unpoisoned(&state.app_db);
        db.set_favorite(&id, favorite)
    };
    match updated {
        Ok(0) => catalog_error(CatalogError::not_found("目录中不存在该条目")),
        Ok(_) => {
            reload_visible_catalog(state);
            WsServerMessage::Response(WsServerResponse::ScannerAppUpdated { id })
        }
        Err(e) => WsServerMessage::Response(WsServerResponse::error("FAVORITE_FAILED", &e)),
    }
}

/// 置顶或取消置顶；数据库锁在刷新可见目录前已释放。
fn handle_set_pinned(state: &AppState, id: &str, pinned: bool) -> WsServerMessage {
    let id = match app_catalog::validate_entry_id(id) {
        Ok(id) => id,
        Err(error) => return catalog_error(error),
    };
    let updated = {
        let db = crate::lock_unpoisoned(&state.app_db);
        db.set_pinned(&id, pinned)
    };
    match updated {
        Ok(0) => catalog_error(CatalogError::not_found("目录中不存在该条目")),
        Ok(_) => {
            reload_visible_catalog(state);
            WsServerMessage::Response(WsServerResponse::ScannerAppUpdated { id })
        }
        Err(e) => WsServerMessage::Response(WsServerResponse::error("PIN_FAILED", &e)),
    }
}
