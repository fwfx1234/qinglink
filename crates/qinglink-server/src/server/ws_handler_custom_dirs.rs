use super::*;

pub(super) fn handle(
    state: &AppState,
    req: &WsClientRequest,
    _session_token: Option<&str>,
) -> WsServerMessage {
    match req {
        // === Custom Scan Dirs ===
        WsClientRequest::GetCustomDirs => list_custom_dirs(state),
        WsClientRequest::AddCustomDir {
            path,
            name,
            enabled,
            max_depth,
            extensions,
            recursive,
        } => add_custom_dir(
            state,
            crate::custom_dir::AddCustomDirRequest {
                path: path.clone(),
                name: name.clone(),
                enabled: *enabled,
                max_depth: *max_depth,
                extensions: extensions.clone(),
                recursive: *recursive,
            },
        ),
        WsClientRequest::UpdateCustomDir {
            id,
            path,
            name,
            enabled,
            max_depth,
            extensions,
            recursive,
        } => update_custom_dir(
            state,
            id,
            crate::custom_dir::UpdateCustomDirRequest {
                path: path.clone(),
                name: name.clone(),
                enabled: *enabled,
                max_depth: *max_depth,
                extensions: extensions.clone(),
                recursive: *recursive,
            },
        ),
        WsClientRequest::DeleteCustomDir { id } => delete_custom_dir(state, id),
        WsClientRequest::ValidateDir { path } => validate_dir(path),

        _ => super::unrouted_request(),
    }
}

/// 列出已配置的自定义扫描目录。
fn list_custom_dirs(state: &AppState) -> WsServerMessage {
    let manager = crate::lock_unpoisoned(state.app_scanner.custom_dir_manager());
    let dirs: Vec<_> = manager.list().iter().map(|d| (*d).clone()).collect();
    WsServerMessage::Response(WsServerResponse::CustomDirs(
        crate::protocol::responses::CustomDirListResponse { dirs },
    ))
}

/// 新增一个自定义扫描目录并回传落库后的记录。
fn add_custom_dir(
    state: &AppState,
    req: crate::custom_dir::AddCustomDirRequest,
) -> WsServerMessage {
    let mut manager = crate::lock_unpoisoned(state.app_scanner.custom_dir_manager());
    match manager.add(req) {
        Ok(dir) => WsServerMessage::Response(WsServerResponse::CustomDir(custom_dir_response(dir))),
        Err(e) => WsServerMessage::Response(WsServerResponse::error("ADD_FAILED", &e.to_string())),
    }
}

/// 更新自定义扫描目录中被提交的字段并回传落库后的记录。
fn update_custom_dir(
    state: &AppState,
    id: &str,
    req: crate::custom_dir::UpdateCustomDirRequest,
) -> WsServerMessage {
    let mut manager = crate::lock_unpoisoned(state.app_scanner.custom_dir_manager());
    match manager.update(id, req) {
        Ok(dir) => WsServerMessage::Response(WsServerResponse::CustomDir(custom_dir_response(dir))),
        Err(e) => {
            WsServerMessage::Response(WsServerResponse::error("UPDATE_FAILED", &e.to_string()))
        }
    }
}

/// 删除指定的自定义扫描目录。
fn delete_custom_dir(state: &AppState, id: &str) -> WsServerMessage {
    let mut manager = crate::lock_unpoisoned(state.app_scanner.custom_dir_manager());
    match manager.remove(id) {
        Ok(()) => WsServerMessage::Response(WsServerResponse::success()),
        Err(e) => {
            WsServerMessage::Response(WsServerResponse::error("DELETE_FAILED", &e.to_string()))
        }
    }
}

/// 校验一个候选目录是否可用于扫描。
fn validate_dir(path: &str) -> WsServerMessage {
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

/// 把目录记录转换为下发结构（两侧字段一一对应）。
fn custom_dir_response(dir: crate::custom_dir::CustomDir) -> crate::protocol::responses::CustomDir {
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
    }
}
