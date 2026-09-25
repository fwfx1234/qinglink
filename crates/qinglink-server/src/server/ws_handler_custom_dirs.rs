use super::*;

pub(super) fn handle(state: &AppState, req: &WsClientRequest, _session_token: Option<&str>) -> WsServerMessage {
    match req {
        // === Custom Scan Dirs ===
        WsClientRequest::GetCustomDirs => {
            let manager = crate::lock_unpoisoned(state.app_scanner.custom_dir_manager());
            let dirs: Vec<_> = manager.list().iter().map(|d| (*d).clone()).collect();
            WsServerMessage::Response(WsServerResponse::CustomDirs(
                crate::protocol::responses::CustomDirListResponse { dirs },
            ))
        }
        WsClientRequest::AddCustomDir {
            path,
            name,
            enabled,
            max_depth,
            extensions,
            recursive,
        } => {
            let mut manager = crate::lock_unpoisoned(state.app_scanner.custom_dir_manager());
            let req = crate::custom_dir::AddCustomDirRequest {
                path: path.clone(),
                name: name.clone(),
                enabled: *enabled,
                max_depth: *max_depth,
                extensions: extensions.clone(),
                recursive: *recursive,
            };
            match manager.add(req) {
                Ok(dir) => WsServerMessage::Response(WsServerResponse::CustomDir(
                    crate::protocol::responses::CustomDir {
                        id: dir.id,
                        path: dir.path,
                        name: dir.name,
                        enabled: dir.enabled,
                        max_depth: dir.max_depth,
                        extensions: dir.extensions,
                        recursive: dir.recursive,
                        created_at: dir.created_at,
                        last_scanned: dir.last_scanned,
                    },
                )),
                Err(e) => {
                    WsServerMessage::Response(WsServerResponse::error("ADD_FAILED", &e.to_string()))
                }
            }
        }
        WsClientRequest::UpdateCustomDir {
            id,
            path,
            name,
            enabled,
            max_depth,
            extensions,
            recursive,
        } => {
            let mut manager = crate::lock_unpoisoned(state.app_scanner.custom_dir_manager());
            let req = crate::custom_dir::UpdateCustomDirRequest {
                path: path.clone(),
                name: name.clone(),
                enabled: *enabled,
                max_depth: *max_depth,
                extensions: extensions.clone(),
                recursive: *recursive,
            };
            match manager.update(id, req) {
                Ok(dir) => WsServerMessage::Response(WsServerResponse::CustomDir(
                    crate::protocol::responses::CustomDir {
                        id: dir.id,
                        path: dir.path,
                        name: dir.name,
                        enabled: dir.enabled,
                        max_depth: dir.max_depth,
                        extensions: dir.extensions,
                        recursive: dir.recursive,
                        created_at: dir.created_at,
                        last_scanned: dir.last_scanned,
                    },
                )),
                Err(e) => WsServerMessage::Response(WsServerResponse::error(
                    "UPDATE_FAILED",
                    &e.to_string(),
                )),
            }
        }
        WsClientRequest::DeleteCustomDir { id } => {
            let mut manager = crate::lock_unpoisoned(state.app_scanner.custom_dir_manager());
            match manager.remove(id) {
                Ok(()) => WsServerMessage::Response(WsServerResponse::success()),
                Err(e) => WsServerMessage::Response(WsServerResponse::error(
                    "DELETE_FAILED",
                    &e.to_string(),
                )),
            }
        }
        WsClientRequest::ValidateDir { path } => {
            let result = crate::custom_dir::CustomDirManager::validate(path);
            WsServerMessage::Response(WsServerResponse::DirValidation(
                crate::protocol::responses::DirValidationResult {
                    valid: result.valid,
                    exists: result.exists,
                    is_dir: result.is_dir,
                    readable: result.readable,
                    exe_count: result.exe_count,
                    error: result.error,
                },
            ))
        }

        _ => unreachable!("request routed to the wrong handler"),
    }
}


