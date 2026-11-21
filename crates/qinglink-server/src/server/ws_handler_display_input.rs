use super::*;

fn key_input_error(error: &anyhow::Error) -> WsServerMessage {
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

pub(super) fn handle(
    state: &AppState,
    req: &WsClientRequest,
    _session_token: Option<&str>,
) -> WsServerMessage {
    match req {
        // === Display & remote input ===
        WsClientRequest::GetDisplays => handle_display_list(),
        WsClientRequest::SetDisplayMode {
            display_id,
            width,
            height,
            refresh_millihz,
        } => handle_set_display_mode(display_id, *width, *height, *refresh_millihz),
        WsClientRequest::MouseMove { dx, dy } => handle_mouse_move(*dx, *dy),
        WsClientRequest::MouseMoveAbsolute { x, y } => handle_mouse_move_absolute(*x, *y),
        WsClientRequest::MouseButton { button } => handle_mouse_button(button),
        WsClientRequest::MouseButtonState { button, pressed } => {
            handle_mouse_button_state(state, button, *pressed)
        }
        WsClientRequest::MouseWheel { delta } => handle_mouse_wheel(*delta),
        WsClientRequest::SendText { text } => handle_send_text(text),
        WsClientRequest::KeyPress { key } => handle_key_press(key),
        WsClientRequest::KeyEvent {
            usage,
            modifiers,
            pressed,
        } => handle_key_event(state, *usage, *modifiers, *pressed),

        _ => super::unrouted_request(),
    }
}

/// 返回显示器列表；平台失败时映射为 `DISPLAY_LIST_FAILED`。
fn handle_display_list() -> WsServerMessage {
    match crate::platform::get_displays() {
        Ok(displays) => WsServerMessage::Response(WsServerResponse::Displays(
            crate::protocol::responses::DisplaysResponse { displays },
        )),
        Err(error) => WsServerMessage::Response(WsServerResponse::error(
            "DISPLAY_LIST_FAILED",
            &error.to_string(),
        )),
    }
}

/// 切换指定显示器的分辨率与刷新率。
fn handle_set_display_mode(
    display_id: &str,
    width: u32,
    height: u32,
    refresh_millihz: u32,
) -> WsServerMessage {
    match crate::platform::set_display_mode(display_id, width, height, refresh_millihz) {
        Ok(()) => WsServerMessage::Response(WsServerResponse::success()),
        Err(error) => WsServerMessage::Response(WsServerResponse::error(
            "DISPLAY_SET_FAILED",
            &error.to_string(),
        )),
    }
}

/// 按协议上限裁剪相对位移后移动指针。
fn handle_mouse_move(dx: i32, dy: i32) -> WsServerMessage {
    let dx = dx.clamp(
        -crate::protocol::requests::MAX_MOUSE_MOVE_DELTA,
        crate::protocol::requests::MAX_MOUSE_MOVE_DELTA,
    );
    let dy = dy.clamp(
        -crate::protocol::requests::MAX_MOUSE_MOVE_DELTA,
        crate::protocol::requests::MAX_MOUSE_MOVE_DELTA,
    );
    match crate::platform::mouse_move(dx, dy) {
        Ok(()) => WsServerMessage::Response(WsServerResponse::success()),
        Err(error) => {
            tracing::warn!(dx, dy, %error, "mouse move failed");
            mouse_input_error(&error)
        }
    }
}

/// 把指针移动到屏幕绝对坐标。
fn handle_mouse_move_absolute(x: u32, y: u32) -> WsServerMessage {
    tracing::debug!(x, y, "absolute mouse move");
    match crate::platform::mouse_move_absolute(x, y) {
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

/// 在指针当前位置单击一次鼠标按键。
fn handle_mouse_button(button: &str) -> WsServerMessage {
    tracing::info!(button, "mouse button click requested");
    match crate::platform::mouse_button(button) {
        Ok(()) => {
            tracing::info!(button, "mouse button click delivered");
            WsServerMessage::Response(WsServerResponse::success())
        }
        Err(error) => {
            tracing::warn!(button, %error, "mouse button click failed");
            mouse_input_error(&error)
        }
    }
}

/// 按下或抬起鼠标按键；仅在平台调用成功后同步 held_input。
fn handle_mouse_button_state(state: &AppState, button: &str, pressed: bool) -> WsServerMessage {
    tracing::info!(button, pressed, "mouse button state requested");
    match crate::platform::mouse_button_state(button, pressed) {
        Ok(()) => {
            if let Ok(mut held) = state.held_input.lock() {
                held.button(button, pressed);
            }
            tracing::info!(button, pressed, "mouse button state delivered");
            WsServerMessage::Response(WsServerResponse::success())
        }
        Err(error) => {
            tracing::warn!(button, pressed, %error, "mouse button state failed");
            mouse_input_error(&error)
        }
    }
}

/// 滚动鼠标滚轮。
fn handle_mouse_wheel(delta: i32) -> WsServerMessage {
    match crate::platform::mouse_wheel(delta) {
        Ok(()) => WsServerMessage::Response(WsServerResponse::success()),
        Err(error) => mouse_input_error(&error),
    }
}

/// 向前台窗口注入文本；超过 64 KiB 时直接拒绝。
fn handle_send_text(text: &str) -> WsServerMessage {
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

/// 触发一次具名按键；不在白名单内的按键返回 `INVALID_INPUT`。
fn handle_key_press(key: &str) -> WsServerMessage {
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

/// 发送原始 HID 键盘事件；usage 越界时在触达平台前拒绝。
fn handle_key_event(state: &AppState, usage: u8, modifiers: u8, pressed: bool) -> WsServerMessage {
    if !(0x04..=0x73).contains(&usage) {
        tracing::warn!(usage, modifiers, "invalid HID usage");
        WsServerMessage::Response(WsServerResponse::error(
            "INVALID_INPUT",
            "HID usage 或修饰键无效",
        ))
    } else {
        tracing::debug!(usage, modifiers, pressed, "HID key event");
        match crate::platform::key_event(usage, modifiers, pressed) {
            Ok(()) => {
                if let Ok(mut held) = state.held_input.lock() {
                    held.key(usage, modifiers, pressed);
                }
                WsServerMessage::Response(WsServerResponse::success())
            }
            Err(error) => {
                tracing::warn!(usage, modifiers, pressed, %error, "HID key event failed");
                key_input_error(&error)
            }
        }
    }
}
