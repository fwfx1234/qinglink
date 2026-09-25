use super::*;

pub(super) fn handle(state: &AppState, req: &WsClientRequest, _session_token: Option<&str>) -> WsServerMessage {
    match req {
        // === Tasks ===
        WsClientRequest::GetTasks {
            search,
            page,
            page_size,
            filter,
        } => {
            let mut pm = crate::lock_unpoisoned(&state.process_manager);
            let list = pm.list_processes(search.as_deref(), *page, *page_size, Some(filter));
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

        _ => unreachable!("request routed to the wrong handler"),
    }
}


