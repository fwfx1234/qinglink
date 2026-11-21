use super::*;

pub(super) fn handle(
    _state: &AppState,
    req: &WsClientRequest,
    _session_token: Option<&str>,
) -> WsServerMessage {
    match req {
        // === File System ===
        WsClientRequest::BrowseFiles { path } => {
            let path_str = path.as_deref().unwrap_or("");
            if path_str.is_empty() {
                let drives = crate::service::file_browser::get_drives();
                WsServerMessage::Response(WsServerResponse::DirectoryListing(
                    crate::service::file_browser::DirectoryListing {
                        current_path: String::new(),
                        parent_path: None,
                        entries: drives,
                        total_count: 0,
                    },
                ))
            } else {
                match crate::service::file_browser::browse_directory(path_str) {
                    Ok(listing) => {
                        WsServerMessage::Response(WsServerResponse::DirectoryListing(listing))
                    }
                    Err(e) => {
                        WsServerMessage::Response(WsServerResponse::error("BROWSE_FAILED", &e))
                    }
                }
            }
        }
        WsClientRequest::GetQuickAccess => {
            let items = crate::service::file_browser::get_quick_access();
            WsServerMessage::Response(WsServerResponse::QuickAccess(items))
        }

        _ => super::unrouted_request(),
    }
}
