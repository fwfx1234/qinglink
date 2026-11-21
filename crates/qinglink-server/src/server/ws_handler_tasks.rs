use super::*;

pub(super) fn handle(
    state: &AppState,
    req: &WsClientRequest,
    _session_token: Option<&str>,
) -> WsServerMessage {
    match req {
        // === Tasks ===
        WsClientRequest::GetTasks(query) => {
            let mut pm = crate::lock_unpoisoned(&state.process_manager);
            let list = pm.list_processes(
                query.search.as_deref(),
                query.page,
                query.page_size,
                Some(&query.filter),
            );
            WsServerMessage::Response(WsServerResponse::Tasks(list))
        }
        WsClientRequest::GetTasksStats => {
            let status = state.system_service.get_status();
            WsServerMessage::Response(WsServerResponse::SystemStatus(status))
        }
        WsClientRequest::KillTask { pid } => {
            let pm = crate::lock_unpoisoned(&state.process_manager);
            match pm.kill_process(*pid) {
                Ok(()) => WsServerMessage::Response(WsServerResponse::success()),
                Err(e) => WsServerMessage::Response(WsServerResponse::error(
                    "KILL_FAILED",
                    &e.to_string(),
                )),
            }
        }
        WsClientRequest::SetTaskPriority { pid, priority } => {
            let pm = crate::lock_unpoisoned(&state.process_manager);
            match pm.set_process_priority(*pid, priority) {
                Ok(()) => WsServerMessage::Response(WsServerResponse::success()),
                Err(e) => WsServerMessage::Response(WsServerResponse::error(
                    "PRIORITY_FAILED",
                    &e.to_string(),
                )),
            }
        }

        _ => super::unrouted_request(),
    }
}
