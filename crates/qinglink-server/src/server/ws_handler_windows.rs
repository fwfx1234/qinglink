use super::*;

pub(super) fn handle(
    state: &AppState,
    req: &WsClientRequest,
    _session_token: Option<&str>,
) -> WsServerMessage {
    match req {
        // === Window Control ===
        WsClientRequest::ListWindows { force_refresh } => {
            let windows = state.list_windows(*force_refresh);
            WsServerMessage::Response(WsServerResponse::WindowList(windows))
        }
        WsClientRequest::FocusWindow { hwnd } => match crate::platform::focus_window(*hwnd) {
            Ok(()) => {
                state.invalidate_window_list();
                WsServerMessage::Response(WsServerResponse::success())
            }
            Err(e) => {
                WsServerMessage::Response(WsServerResponse::error("FOCUS_FAILED", &e.to_string()))
            }
        },
        WsClientRequest::SwitchWindow { direction } => {
            match crate::platform::switch_window(*direction) {
                Ok(()) => {
                    state.invalidate_window_list();
                    WsServerMessage::Response(WsServerResponse::success())
                }
                Err(e) => WsServerMessage::Response(WsServerResponse::error(
                    "SWITCH_WINDOW_FAILED",
                    &e.to_string(),
                )),
            }
        }
        WsClientRequest::MinimizeWindow { hwnd } => handle_window_action(
            state,
            crate::platform::minimize_window(*hwnd),
            "MINIMIZE_FAILED",
        ),
        WsClientRequest::MaximizeWindow { hwnd } => handle_window_action(
            state,
            crate::platform::maximize_window(*hwnd),
            "MAXIMIZE_FAILED",
        ),
        WsClientRequest::RestoreWindow { hwnd } => handle_window_action(
            state,
            crate::platform::restore_window(*hwnd),
            "RESTORE_FAILED",
        ),
        WsClientRequest::CloseWindow { hwnd } => {
            handle_window_action(state, crate::platform::close_window(*hwnd), "CLOSE_FAILED")
        }
        WsClientRequest::MinimizeForeground => {
            handle_foreground_action(state, crate::platform::minimize_window, "MINIMIZE_FAILED")
        }
        WsClientRequest::MaximizeForeground => {
            handle_foreground_action(state, crate::platform::maximize_window, "MAXIMIZE_FAILED")
        }
        WsClientRequest::CloseForeground => {
            handle_foreground_action(state, crate::platform::close_window, "CLOSE_FAILED")
        }

        _ => unreachable!("request routed to the wrong handler"),
    }
}

fn handle_window_action(
    state: &AppState,
    result: anyhow::Result<()>,
    error_code: &str,
) -> WsServerMessage {
    match result {
        Ok(()) => {
            state.invalidate_window_list();
            WsServerMessage::Response(WsServerResponse::success())
        }
        Err(error) => {
            WsServerMessage::Response(WsServerResponse::error(error_code, &error.to_string()))
        }
    }
}

fn handle_foreground_action(
    state: &AppState,
    action: fn(usize) -> anyhow::Result<()>,
    error_code: &str,
) -> WsServerMessage {
    let result = crate::platform::get_foreground_window()
        .ok_or_else(|| anyhow::anyhow!("当前没有可操作的前台窗口"))
        .and_then(action);
    handle_window_action(state, result, error_code)
}
