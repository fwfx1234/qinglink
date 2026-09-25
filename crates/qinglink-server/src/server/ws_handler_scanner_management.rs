use super::*;

pub(super) fn handle(state: &AppState, req: &WsClientRequest, _session_token: Option<&str>) -> WsServerMessage {
    match req {
        // === Scanner App Management ===
        WsClientRequest::AddScannerApp {
            name,
            exe_path,
            category,
            icon_base64,
            source,
            args,
        } => {
            // 扫描目录时不再批量提取图标；加入应用库时按需补一次，避免
            // 手机端列表丢失图标。
            let icon_base64 = icon_base64
                .clone()
                .or_else(|| crate::platform::extract_icon_base64(exe_path));
            let app = crate::service::app_scanner::AppEntry {
                id: format!("user:{}", uuid::Uuid::new_v4()),
                name: name.clone(),
                original_name: name.clone(),
                exe_path: exe_path.clone(),
                icon_base64: icon_base64.clone(),
                category: category.clone(),
                source: source.clone().unwrap_or_else(|| "User".to_string()),
                args: args.clone(),
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
        WsClientRequest::UpdateScannerApp {
            id,
            name,
            exe_path,
            icon_base64,
            args,
        } => {
            let db = crate::lock_unpoisoned(&state.app_db);
            match db.update_app(id, name, exe_path, icon_base64.as_deref(), args.as_deref()) {
                Ok(()) => {
                    drop(db);
                    reload_visible_catalog(state);
                    WsServerMessage::Response(WsServerResponse::ScannerAppUpdated {
                        id: id.clone(),
                    })
                }
                Err(e) => WsServerMessage::Response(WsServerResponse::error("UPDATE_FAILED", &e)),
            }
        }
        WsClientRequest::DeleteScannerApp { id } => {
            let db = crate::lock_unpoisoned(&state.app_db);
            match db.delete_app(id) {
                Ok(()) => {
                    drop(db);
                    reload_visible_catalog(state);
                    WsServerMessage::Response(WsServerResponse::ScannerAppDeleted {
                        id: id.clone(),
                    })
                }
                Err(e) => WsServerMessage::Response(WsServerResponse::error("DELETE_FAILED", &e)),
            }
        }
        WsClientRequest::SetScannerAppHidden { id, hidden } => {
            let db = crate::lock_unpoisoned(&state.app_db);
            match db.set_hidden(id, *hidden) {
                Ok(()) => {
                    drop(db);
                    reload_visible_catalog(state);
                    WsServerMessage::Response(WsServerResponse::ScannerAppUpdated {
                        id: id.clone(),
                    })
                }
                Err(e) => WsServerMessage::Response(WsServerResponse::error("HIDE_FAILED", &e)),
            }
        }
        _ => unreachable!("request routed to the wrong handler"),
    }
}


