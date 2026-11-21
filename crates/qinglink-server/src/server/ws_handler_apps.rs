use super::*;

pub(super) fn handle(
    _state: &AppState,
    req: &WsClientRequest,
    _session_token: Option<&str>,
) -> WsServerMessage {
    match req {
        // === Apps ===
        WsClientRequest::SearchApps { query } => {
            let apps = crate::platform::search_installed_apps(query);
            WsServerMessage::Response(WsServerResponse::SearchApps(
                crate::protocol::responses::SearchAppsResponse { apps },
            ))
        }
        WsClientRequest::LaunchApp { path, args } => {
            let path_ref = std::path::Path::new(path);
            if !path_ref.is_file() {
                return WsServerMessage::Response(WsServerResponse::error(
                    "APP_NOT_FOUND",
                    "应用路径不存在或不是文件",
                ));
            }
            match crate::platform::launch_app(path, args) {
                Ok(()) => WsServerMessage::Response(WsServerResponse::success()),
                Err(e) => WsServerMessage::Response(WsServerResponse::error(
                    "LAUNCH_FAILED",
                    &e.to_string(),
                )),
            }
        }

        _ => super::unrouted_request(),
    }
}
