use super::*;

fn key_input_error(error: anyhow::Error) -> WsServerMessage {
    let message = error.to_string();
    let code = if message.contains("AGENT_REQUEST_TIMEOUT") {
        "UAC_AGENT_TIMEOUT"
    } else if message.contains("OpenInputDesktop") || message.contains("SetThreadDesktop") {
        "UAC_DESKTOP_SWITCH_FAILED"
    } else {
        "KEY_EVENT_FAILED"
    };
    let user_message = match code {
        "UAC_AGENT_TIMEOUT" => "UAC输入服务响应超时，请检查安全输入服务是否正常运行",
        "UAC_DESKTOP_SWITCH_FAILED" => "无法切换到安全桌面，键盘输入需要管理员权限",
        _ => &message,
    };
    WsServerMessage::Response(WsServerResponse::error(code, user_message))
}

pub(super) fn handle(_state: &AppState, req: &WsClientRequest, _session_token: Option<&str>) -> WsServerMessage {
    match req {
        // === Display & remote input ===
        WsClientRequest::GetDisplays => match crate::platform::get_displays() {
            Ok(displays) => WsServerMessage::Response(WsServerResponse::Displays(
                crate::protocol::responses::DisplaysResponse { displays },
            )),
            Err(error) => WsServerMessage::Response(WsServerResponse::error(
                "DISPLAY_LIST_FAILED",
                &error.to_string(),
            )),
        },
        WsClientRequest::SetDisplayMode {
            display_id,
            width,
            height,
            refresh_millihz,
        } => match crate::platform::set_display_mode(display_id, *width, *height, *refresh_millihz)
        {
            Ok(()) => WsServerMessage::Response(WsServerResponse::success()),
            Err(error) => WsServerMessage::Response(WsServerResponse::error(
                "DISPLAY_SET_FAILED",
                &error.to_string(),
            )),
        },
        WsClientRequest::MouseMove { dx, dy } => {
            let dx = (*dx).clamp(
                -crate::protocol::requests::MAX_MOUSE_MOVE_DELTA,
                crate::protocol::requests::MAX_MOUSE_MOVE_DELTA,
            );
            let dy = (*dy).clamp(
                -crate::protocol::requests::MAX_MOUSE_MOVE_DELTA,
                crate::protocol::requests::MAX_MOUSE_MOVE_DELTA,
            );
            match crate::platform::mouse_move(dx, dy) {
                Ok(()) => WsServerMessage::Response(WsServerResponse::success()),
                Err(error) => {
                    tracing::warn!(dx, dy, %error, "mouse move failed");
                    mouse_input_error(error)
                }
            }
        }
        WsClientRequest::MouseMoveAbsolute { x, y } => {
            tracing::debug!(x, y, "absolute mouse move");
            match crate::platform::mouse_move_absolute(*x, *y) {
                Ok(()) => WsServerMessage::Response(WsServerResponse::success()),
                Err(error) => {
                    tracing::warn!(x, y, %error, "absolute mouse move failed");
                    WsServerMessage::Response(WsServerResponse::error(
                        "MOUSE_MOVE_ABSOLUTE_FAILED",
                        &error.to_string(),
                    ))
                }
            }
        }
        WsClientRequest::MouseButton { button } => {
            tracing::info!(button, "mouse button click requested");
            match crate::platform::mouse_button(button) {
                Ok(()) => {
                    tracing::info!(button, "mouse button click delivered");
                    WsServerMessage::Response(WsServerResponse::success())
                }
                Err(error) => {
                    tracing::warn!(button, %error, "mouse button click failed");
                    mouse_input_error(error)
                }
            }
        }
        WsClientRequest::MouseButtonState { button, pressed } => {
            tracing::info!(button, pressed, "mouse button state requested");
            match crate::platform::mouse_button_state(button, *pressed) {
                Ok(()) => {
                    tracing::info!(button, pressed, "mouse button state delivered");
                    WsServerMessage::Response(WsServerResponse::success())
                }
                Err(error) => {
                    tracing::warn!(button, pressed, %error, "mouse button state failed");
                    mouse_input_error(error)
                }
            }
        }
        WsClientRequest::MouseWheel { delta } => match crate::platform::mouse_wheel(*delta) {
            Ok(()) => WsServerMessage::Response(WsServerResponse::success()),
            Err(error) => mouse_input_error(error),
        },
        WsClientRequest::SendText { text } => {
            if text.len() > 64 * 1024 {
                WsServerMessage::Response(WsServerResponse::error(
                    "TEXT_TOO_LARGE",
                    "发送文字不能超过 64 KiB",
                ))
            } else {
                match crate::platform::send_text(text) {
                    Ok(()) => WsServerMessage::Response(WsServerResponse::success()),
                    Err(error) => WsServerMessage::Response(WsServerResponse::error(
                        "TEXT_SEND_FAILED",
                        &error.to_string(),
                    )),
                }
            }
        }
        WsClientRequest::KeyPress { key } => {
            if !is_supported_remote_key(key) {
                WsServerMessage::Response(WsServerResponse::error("INVALID_INPUT", "不支持的按键"))
            } else {
                match crate::platform::key_press(key) {
                    Ok(()) => WsServerMessage::Response(WsServerResponse::success()),
                    Err(error) => WsServerMessage::Response(WsServerResponse::error(
                        "KEY_PRESS_FAILED",
                        &error.to_string(),
                    )),
                }
            }
        }
        WsClientRequest::KeyEvent {
            usage,
            modifiers,
            pressed,
        } => {
            if !(*usage >= 0x04 && *usage <= 0x73) {
                tracing::warn!(usage, modifiers, "invalid HID usage");
                WsServerMessage::Response(WsServerResponse::error(
                    "INVALID_INPUT",
                    "HID usage 或修饰键无效",
                ))
            } else {
                tracing::debug!(usage, modifiers, pressed, "HID key event");
                match crate::platform::key_event(*usage, *modifiers, *pressed) {
                    Ok(()) => WsServerMessage::Response(WsServerResponse::success()),
                    Err(error) => {
                        tracing::warn!(usage, modifiers, pressed, %error, "HID key event failed");
                        key_input_error(error)
                    }
                }
            }
        }

        _ => unreachable!("request routed to the wrong handler"),
    }
}


