//! 会话代理命令：锁屏、安全桌面文本、窗口枚举、会话状态与显示器控制。
//!
//! 这些命令只在会话代理进程（用户桌面所在的交互会话）里执行。服务进程收到它们时
//! 不自己处理，而是转发给代理——见 `qinglink_windows::display` 与
//! `qinglink_windows::secure_desktop` 里的转发分支。

use anyhow::Result;

use super::dispatch::{Request, respond_result, write_response};
use super::pipe::Reply;

/// 分发一条会话代理命令。
pub(super) fn dispatch(stream: &mut Reply, request: &Request) -> Result<()> {
    match request.command.as_str() {
        "lock" => respond_result(
            stream,
            qinglink_windows::power::lock(),
            "LOCK_FAILED",
            "SERVICE_OK",
            "锁屏已请求",
        ),
        "secure_ascii" => input_response(
            stream,
            qinglink_windows::secure_desktop::send_secure_ascii(
                request.text.as_deref().unwrap_or(""),
            ),
            "安全文本已发送",
        ),
        "send_text" => input_response(
            stream,
            qinglink_windows::secure_desktop::send_text(request.text.as_deref().unwrap_or("")),
            "文字已发送",
        ),
        "list_windows" => {
            match serde_json::to_string(&qinglink_windows::platform_windows::enum_windows_light()) {
                Ok(payload) => write_response(stream, true, "WINDOW_LIST", &payload),
                Err(error) => {
                    write_response(stream, false, "WINDOW_LIST_FAILED", &error.to_string())
                }
            }
        }
        "status" => match qinglink_windows::secure_desktop::status() {
            Ok(status) => write_agent_status_response(stream, &status),
            Err(error) => write_response(stream, false, "STATUS_FAILED", &error.to_string()),
        },
        // 焦点输入框文本放在 message 里回传（与 list_windows 同一约定）：
        // 空串表示"没有可回填的输入框"，不是错误。
        "focus_text" => match qinglink_windows::platform_windows::read_focus_text() {
            Ok(Some(text)) => write_response(stream, true, "FOCUS_TEXT", &text),
            Ok(None) => write_response(stream, true, "FOCUS_TEXT_EMPTY", ""),
            Err(error) => write_response(stream, false, "FOCUS_TEXT_FAILED", &error.to_string()),
        },
        // 焦点是否在可编辑控件上：只回传代理后台线程维护的缓存，**不做 UIA**。
        // 这里单线程串行处理请求，命令里读 UIA 会堵住后面的鼠标/键盘注入。
        "focus_input" => write_response(
            stream,
            true,
            "FOCUS_INPUT",
            if super::focus_cache::active() {
                "1"
            } else {
                "0"
            },
        ),
        // 物理鼠标按键状态：JSON 放在 message 里回传，读失败按"全部抬起"处理。
        "input_state" => {
            let payload =
                serde_json::to_string(&qinglink_windows::platform_windows::pressed_buttons())
                    .unwrap_or_else(|_| {
                        r#"{"left":false,"right":false,"middle":false}"#.to_string()
                    });
            write_response(stream, true, "INPUT_STATE", &payload)
        }
        // 显示器列表放在 message 里回传，由服务进程原样交给客户端。
        //
        // 走 `qinglink_windows::display` 门面而不是 `platform_windows`：代理进程会
        // 直接本地枚举，服务进程（Session 0）则自动转发给代理，两条路径都不会在
        // 错误的会话里调 Win32。
        "get_displays" => match qinglink_windows::display::get_displays() {
            Ok(displays) => match serde_json::to_string(&displays) {
                Ok(payload) => write_response(stream, true, "DISPLAY_LIST", &payload),
                Err(error) => {
                    write_response(stream, false, "DISPLAY_LIST_FAILED", &error.to_string())
                }
            },
            Err(error) => write_response(stream, false, "DISPLAY_LIST_FAILED", &error.to_string()),
        },
        "set_display_mode" => {
            let result = match (
                request.display_id.as_deref(),
                request.width,
                request.height,
                request.refresh_millihz,
            ) {
                (Some(display_id), Some(width), Some(height), Some(refresh_millihz)) => {
                    qinglink_windows::display::set_display_mode(
                        display_id,
                        width,
                        height,
                        refresh_millihz,
                    )
                }
                _ => Err(anyhow::anyhow!(
                    "缺少 display_id / width / height / refresh_millihz"
                )),
            };
            respond_result(
                stream,
                result,
                "DISPLAY_SET_FAILED",
                "SERVICE_OK",
                "显示模式已切换",
            )
        }
        _ => write_response(stream, false, "UNKNOWN_COMMAND", "未知输入命令"),
    }
}

/// 把输入类失败统一映射成 `INPUT_FAILED`。
fn input_response(stream: &mut Reply, result: Result<()>, message: &str) -> Result<()> {
    respond_result(stream, result, "INPUT_FAILED", "SERVICE_OK", message)
}

/// 会话代理就绪状态；字段名与 `SecureDesktopStatus` 一一对应。
fn write_agent_status_response(
    stream: &mut Reply,
    status: &qinglink_windows::secure_desktop::SecureDesktopStatus,
) -> Result<()> {
    #[derive(serde::Serialize)]
    struct AgentStatus<'a> {
        ok: bool,
        code: &'a str,
        message: &'a str,
        desktop: &'a str,
        locked: bool,
        uac_detected: bool,
        input_ready: bool,
        session_id: u32,
        token_integrity_level: u32,
        agent_state: &'a str,
        last_error: &'a str,
    }
    stream.send(&AgentStatus {
        ok: status.input_ready,
        code: if status.input_ready {
            "AGENT_READY"
        } else {
            "AGENT_NOT_READY"
        },
        message: "QingLink 会话代理已就绪",
        desktop: &status.desktop,
        locked: status.locked,
        uac_detected: status.uac_detected,
        input_ready: status.input_ready,
        session_id: status.session_id,
        token_integrity_level: status.token_integrity_level,
        agent_state: &status.agent_state,
        last_error: &status.last_error,
    })
}
