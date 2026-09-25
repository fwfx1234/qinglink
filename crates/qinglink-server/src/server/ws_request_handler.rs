use axum::extract::ws::Message;

use crate::protocol::requests::{WsClientRequest, WsServerMessage, WsServerResponse};
use crate::server::{AppState, reload_visible_catalog};

fn is_supported_remote_key(key: &str) -> bool {
    matches!(
        key,
        "backspace"
            | "enter"
            | "escape"
            | "tab"
            | "delete"
            | "home"
            | "end"
            | "pageup"
            | "pagedown"
            | "up"
            | "down"
            | "left"
            | "right"
            | "space"
            | "f1"
            | "f2"
            | "f3"
            | "f4"
            | "f5"
            | "f6"
            | "f7"
            | "f8"
            | "f9"
            | "f10"
            | "f11"
            | "f12"
    )
}
fn scanning_is_paused(state: &AppState) -> bool {
    state
        .window_monitor
        .lock()
        .map(|monitor| monitor.is_gaming())
        .unwrap_or(true)
}

fn mouse_input_error(error: anyhow::Error) -> WsServerMessage {
    let message = error.to_string();
    let code = if message.starts_with("SECURE_DESKTOP_INPUT_BLOCKED:") {
        "SECURE_DESKTOP_INPUT_BLOCKED"
    } else if message.contains("AGENT_REQUEST_TIMEOUT") {
        "UAC_AGENT_TIMEOUT"
    } else if message.contains("OpenInputDesktop") || message.contains("SetThreadDesktop") {
        "UAC_DESKTOP_SWITCH_FAILED"
    } else {
        "MOUSE_INPUT_FAILED"
    };
    let user_message = match code {
        "SECURE_DESKTOP_INPUT_BLOCKED" => "安全桌面输入被阻止，请确保输入服务已安装并运行",
        "UAC_AGENT_TIMEOUT" => "UAC输入服务响应超时，请检查安全输入服务是否正常运行",
        "UAC_DESKTOP_SWITCH_FAILED" => "无法切换到安全桌面，UAC操作需要管理员权限",
        _ => &message,
    };
    WsServerMessage::Response(WsServerResponse::error(code, user_message))
}

mod auth {
    include!("ws_handler_auth.rs");
}
mod system {
    include!("ws_handler_system.rs");
}
mod display_input {
    include!("ws_handler_display_input.rs");
}
mod process {
    include!("ws_handler_process.rs");
}
mod apps {
    include!("ws_handler_apps.rs");
}
mod mobile_catalog {
    include!("ws_handler_mobile_catalog.rs");
}
mod legacy_scanner {
    include!("ws_handler_legacy_scanner.rs");
}
mod steam {
    include!("ws_handler_steam.rs");
}
mod custom_dirs {
    include!("ws_handler_custom_dirs.rs");
}
mod tasks {
    include!("ws_handler_tasks.rs");
}
mod volume_media {
    include!("ws_handler_volume_media.rs");
}
mod files {
    include!("ws_handler_files.rs");
}
mod windows {
    include!("ws_handler_windows.rs");
}
mod scanner_management {
    include!("ws_handler_scanner_management.rs");
}

pub fn handle_client_request(
    state: &AppState,
    req: &WsClientRequest,
    session_token: Option<&str>,
) -> WsServerMessage {
    match req {
        WsClientRequest::Authenticate { .. }
        | WsClientRequest::Ping
        | WsClientRequest::RevokeToken => auth::handle(state, req, session_token),
        WsClientRequest::GetSystemStatus
        | WsClientRequest::GetMacAddress
        | WsClientRequest::GetSubnetPrefix
        | WsClientRequest::GetWakeNetworks
        | WsClientRequest::Shutdown { .. }
        | WsClientRequest::Sleep { .. }
        | WsClientRequest::Restart { .. }
        | WsClientRequest::Logoff
        | WsClientRequest::Lock
        | WsClientRequest::GetUnlockStatus
        | WsClientRequest::AutoUnlock { .. }
        | WsClientRequest::GetLockInputStatus
        | WsClientRequest::SubmitLockPin { .. }
        | WsClientRequest::GetSecureDesktopStatus
        | WsClientRequest::SecureAttention
        | WsClientRequest::SendSecureAscii { .. } => system::handle(state, req, session_token),
        WsClientRequest::GetDisplays
        | WsClientRequest::SetDisplayMode { .. }
        | WsClientRequest::MouseMove { .. }
        | WsClientRequest::MouseMoveAbsolute { .. }
        | WsClientRequest::MouseButton { .. }
        | WsClientRequest::MouseButtonState { .. }
        | WsClientRequest::MouseWheel { .. }
        | WsClientRequest::SendText { .. }
        | WsClientRequest::KeyPress { .. }
        | WsClientRequest::KeyEvent { .. } => display_input::handle(state, req, session_token),
        WsClientRequest::GetProcesses { .. }
        | WsClientRequest::GetForeground
        | WsClientRequest::KillProcess { .. }
        | WsClientRequest::SuspendProcess { .. }
        | WsClientRequest::ResumeProcess { .. } => process::handle(state, req, session_token),
        WsClientRequest::SearchApps { .. } | WsClientRequest::LaunchApp { .. } => {
            apps::handle(state, req, session_token)
        }
        WsClientRequest::ListLauncherItems { .. }
        | WsClientRequest::GetRecentLauncherItems { .. }
        | WsClientRequest::GetLauncherIcon { .. }
        | WsClientRequest::RefreshScannerAppIcon { .. }
        | WsClientRequest::RefreshLauncherCatalog { .. } => {
            mobile_catalog::handle(state, req, session_token)
        }
        WsClientRequest::GetScannerApps
        | WsClientRequest::RefreshScannerApps
        | WsClientRequest::RenameScannerApp { .. }
        | WsClientRequest::LaunchScannerApp { .. }
        | WsClientRequest::ScanGameDirectory { .. }
        | WsClientRequest::AddScannedGames { .. } => {
            legacy_scanner::handle(state, req, session_token)
        }
        WsClientRequest::GetSteamGames
        | WsClientRequest::RefreshSteamGames
        | WsClientRequest::InstallSteamGame { .. }
        | WsClientRequest::LaunchSteamGame { .. } => steam::handle(state, req, session_token),
        WsClientRequest::GetCustomDirs
        | WsClientRequest::AddCustomDir { .. }
        | WsClientRequest::UpdateCustomDir { .. }
        | WsClientRequest::DeleteCustomDir { .. }
        | WsClientRequest::ValidateDir { .. } => custom_dirs::handle(state, req, session_token),
        WsClientRequest::GetTasks { .. }
        | WsClientRequest::GetTasksStats
        | WsClientRequest::KillTask { .. }
        | WsClientRequest::SetTaskPriority { .. } => tasks::handle(state, req, session_token),
        WsClientRequest::GetVolume
        | WsClientRequest::GetAudioOutputs
        | WsClientRequest::SetAudioOutput { .. }
        | WsClientRequest::SetVolume { .. }
        | WsClientRequest::SetMuted { .. }
        | WsClientRequest::VolumeUp
        | WsClientRequest::VolumeDown
        | WsClientRequest::VolumeMute
        | WsClientRequest::MediaPlayPause
        | WsClientRequest::MediaStop
        | WsClientRequest::MediaNext
        | WsClientRequest::MediaPrev => volume_media::handle(state, req, session_token),
        WsClientRequest::BrowseFiles { .. } | WsClientRequest::GetQuickAccess => {
            files::handle(state, req, session_token)
        }
        WsClientRequest::ListWindows { .. }
        | WsClientRequest::FocusWindow { .. }
        | WsClientRequest::SwitchWindow { .. }
        | WsClientRequest::MinimizeWindow { .. }
        | WsClientRequest::MaximizeWindow { .. }
        | WsClientRequest::RestoreWindow { .. }
        | WsClientRequest::CloseWindow { .. }
        | WsClientRequest::MinimizeForeground
        | WsClientRequest::MaximizeForeground
        | WsClientRequest::CloseForeground => windows::handle(state, req, session_token),
        WsClientRequest::AddScannerApp { .. }
        | WsClientRequest::UpdateScannerApp { .. }
        | WsClientRequest::DeleteScannerApp { .. }
        | WsClientRequest::SetScannerAppHidden { .. } => {
            scanner_management::handle(state, req, session_token)
        }
    }
}

pub async fn send_ws_message(
    socket: &mut axum::extract::ws::WebSocket,
    msg: WsServerMessage,
    request_id: Option<&str>,
) -> Result<(), ()> {
    let json = serialize_ws_message(msg, request_id).map_err(|_| ())?;
    socket.send(Message::Text(json)).await.map_err(|_| ())
}

fn serialize_ws_message(
    msg: WsServerMessage,
    request_id: Option<&str>,
) -> Result<String, serde_json::Error> {
    let mut value = serde_json::to_value(msg)?;
    if let (Some(request_id), Some(object)) = (request_id, value.as_object_mut()) {
        object.insert(
            "request_id".to_string(),
            serde_json::Value::String(request_id.to_string()),
        );
    }
    serde_json::to_string(&value)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn response_echoes_request_id() {
        let message = WsServerMessage::Response(WsServerResponse::success());
        let json = serialize_ws_message(message, Some("request-42")).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(value["type"], "success");
        assert_eq!(value["request_id"], "request-42");
    }

    #[test]
    fn supported_remote_keys_are_whitelisted() {
        for key in [
            "backspace",
            "enter",
            "escape",
            "tab",
            "delete",
            "home",
            "end",
            "pageup",
            "pagedown",
            "up",
            "down",
            "left",
            "right",
            "space",
        ] {
            assert!(is_supported_remote_key(key), "{} 应被支持", key);
        }
        assert!(!is_supported_remote_key("ctrl+v"));
        assert!(!is_supported_remote_key("unknown"));
    }
}
