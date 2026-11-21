use super::*;

pub(super) fn handle(
    state: &AppState,
    req: &WsClientRequest,
    _session_token: Option<&str>,
) -> WsServerMessage {
    match req {
        // === Processes ===
        WsClientRequest::GetProcesses {
            search,
            page,
            page_size,
            filter,
            sort: _,
        } => {
            let mut pm = crate::lock_unpoisoned(&state.process_manager);
            let list =
                pm.list_processes(search.as_deref(), *page, (*page_size).min(50), Some(filter));
            WsServerMessage::Response(WsServerResponse::ProcessList(list))
        }
        WsClientRequest::GetForeground => {
            let pm = crate::lock_unpoisoned(&state.process_manager);
            match pm.get_foreground() {
                Ok(fg) => WsServerMessage::Response(WsServerResponse::Foreground(fg)),
                Err(e) => WsServerMessage::Response(WsServerResponse::error(
                    "FOREGROUND_ERROR",
                    &e.to_string(),
                )),
            }
        }
        WsClientRequest::KillProcess { pid } => {
            let pm = crate::lock_unpoisoned(&state.process_manager);
            match pm.kill_process(*pid) {
                Ok(()) => WsServerMessage::Response(WsServerResponse::success()),
                Err(e) => WsServerMessage::Response(WsServerResponse::error(
                    "KILL_FAILED",
                    &e.to_string(),
                )),
            }
        }
        WsClientRequest::SuspendProcess { pid } => {
            let pm = crate::lock_unpoisoned(&state.process_manager);
            match pm.suspend_process(*pid) {
                Ok(()) => WsServerMessage::Response(WsServerResponse::success()),
                Err(e) => WsServerMessage::Response(WsServerResponse::error(
                    "SUSPEND_FAILED",
                    &e.to_string(),
                )),
            }
        }
        WsClientRequest::ResumeProcess { pid } => {
            let pm = crate::lock_unpoisoned(&state.process_manager);
            match pm.resume_process(*pid) {
                Ok(()) => WsServerMessage::Response(WsServerResponse::success()),
                Err(e) => WsServerMessage::Response(WsServerResponse::error(
                    "RESUME_FAILED",
                    &e.to_string(),
                )),
            }
        }
        WsClientRequest::GetProcessIcon(query) => {
            WsServerMessage::Response(WsServerResponse::ProcessIcon(state.process_icon(query.pid)))
        }

        _ => super::unrouted_request(),
    }
}
