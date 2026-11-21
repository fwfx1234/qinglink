//! 服务管道与会话代理管道的命令解码与分发。

use super::{
    AGENT_MODE, authorization, desktop, dispatch_input, dispatch_session, input, lifecycle_monitor,
    management, pipe::Reply,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use windows::Win32::Foundation::HANDLE;

#[derive(Debug, Deserialize)]
pub struct Request {
    pub(super) version: u32,
    #[serde(default)]
    pub(super) request_id: Option<String>,
    pub(super) command: String,
    pub(super) pin: Option<String>,
    pub(super) text: Option<String>,
    pub(super) dx: Option<i32>,
    pub(super) dy: Option<i32>,
    pub(super) button: Option<String>,
    pub(super) pressed: Option<bool>,
    pub(super) delta: Option<i32>,
    pub(super) usage: Option<u8>,
    pub(super) modifiers: Option<u8>,
    pub(super) parent_pid: Option<u32>,
    pub(super) hwnd: Option<usize>,
    #[serde(default)]
    pub(super) pid: Option<u32>,
    pub(super) direction: Option<String>,
    #[serde(default)]
    pub(super) keys: Vec<(u8, u8)>,
    #[serde(default)]
    pub(super) buttons: Vec<String>,
    #[serde(default)]
    pub(super) display_id: Option<String>,
    #[serde(default)]
    pub(super) width: Option<u32>,
    #[serde(default)]
    pub(super) height: Option<u32>,
    #[serde(default)]
    pub(super) refresh_millihz: Option<u32>,
    #[serde(flatten)]
    pub(super) extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct Response<'a> {
    ok: bool,
    code: &'a str,
    message: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    request_id: Option<&'a str>,
}

pub(super) fn handle_client(
    mut stream: Reply,
    monitor: &lifecycle_monitor::LifecycleMonitor,
    state: Option<&qinglink_server::AppState>,
    stop_tx: Option<&mpsc::Sender<()>>,
) -> Result<()> {
    let client_pid = named_pipe_client_pid(stream.handle())?;
    let request: Request = stream.read_request()?;
    // 授权在读取请求之后、版本校验之前执行，顺序即安全边界，不得调整。
    let desktop_peer = if state.is_some() {
        match authorization::desktop(stream.handle()) {
            Ok(peer) => Some(peer),
            Err(error) => {
                tracing::warn!(%error, client_pid, "desktop authorization rejected");
                return write_response(
                    &mut stream,
                    false,
                    "UNAUTHORIZED_DESKTOP",
                    "桌面程序身份未获授权，请在设置中修复服务安装",
                );
            }
        }
    } else {
        None
    };
    if AGENT_MODE.load(Ordering::Acquire)
        && let Err(error) = desktop::attach_agent_desktop()
    {
        tracing::error!(%error, "Failed to attach agent desktop");
    }
    if request.version != 1 && request.version != 2 {
        return write_response(&mut stream, false, "UNSUPPORTED_VERSION", "协议版本不支持");
    }
    if dispatch_management(&mut stream, &request, state, desktop_peer)? {
        return Ok(());
    }
    match request.command.as_str() {
        "release_inputs" | "get_status" | "register_parent" | "heartbeat" | "shutdown" => {
            dispatch_service_command(&mut stream, &request, state, client_pid, monitor, stop_tx)
        }
        "lock" | "secure_ascii" | "send_text" | "list_windows" | "status" | "focus_text"
        | "input_state" | "focus_input" | "get_displays" | "set_display_mode" => {
            dispatch_session::dispatch(&mut stream, &request)
        }
        "focus_window" | "focus_process" | "minimize_window" | "maximize_window"
        | "restore_window" | "close_window" | "switch_window" => {
            dispatch_window_command(&mut stream, &request)
        }
        "mouse_move"
        | "mouse_move_absolute"
        | "mouse_button"
        | "mouse_button_state"
        | "mouse_wheel" => dispatch_input::dispatch_mouse_command(&mut stream, &request),
        "key_press" | "key_event" => dispatch_input::dispatch_key_command(&mut stream, &request),
        "submit_pin" => dispatch_input::submit_pin(&mut stream, request.pin.as_deref()),
        _ => write_response(&mut stream, false, "UNKNOWN_COMMAND", "未知输入命令"),
    }
}

/// 转交管理命令；返回 `true` 表示命令已被管理端处理并已写出响应。
fn dispatch_management(
    stream: &mut Reply,
    request: &Request,
    state: Option<&qinglink_server::AppState>,
    desktop_peer: Option<authorization::DesktopPeer>,
) -> Result<bool> {
    if let (Some(state), Some(peer)) = (state, desktop_peer) {
        return management::dispatch(stream, request, state, peer);
    }
    Ok(false)
}

/// 服务级命令：输入释放、状态查询、父进程登记、心跳与关闭。
fn dispatch_service_command(
    stream: &mut Reply,
    request: &Request,
    state: Option<&qinglink_server::AppState>,
    client_pid: u32,
    monitor: &lifecycle_monitor::LifecycleMonitor,
    stop_tx: Option<&mpsc::Sender<()>>,
) -> Result<()> {
    match request.command.as_str() {
        "release_inputs" if state.is_none() => release_held_inputs(stream, request),
        "get_status" => {
            let available = state.is_some_and(|state| state.service.is_server_running());
            write_response_with_id(
                stream,
                available,
                if available { "READY" } else { "DEGRADED" },
                if available {
                    "服务已就绪"
                } else {
                    "网络服务未就绪"
                },
                request.request_id.as_deref(),
            )
        }
        "register_parent" => match request.parent_pid {
            Some(pid) if pid == client_pid => {
                monitor.register_parent(pid);
                write_response(stream, true, "REGISTERED", "父进程已注册")
            }
            Some(_) => write_response(
                stream,
                false,
                "PID_MISMATCH",
                "父进程 PID 与管道客户端不匹配",
            ),
            None => write_response(stream, false, "MISSING_PID", "缺少 parent_pid"),
        },
        "heartbeat" => {
            if monitor.accepts_client_pid(client_pid) {
                write_response(stream, true, "HEARTBEAT_OK", "心跳已接收")
            } else {
                write_response(stream, false, "UNREGISTERED_CLIENT", "桌面端尚未登记")
            }
        }
        "shutdown" => {
            if !monitor.accepts_client_pid(client_pid) {
                return write_response(stream, false, "UNREGISTERED_CLIENT", "桌面端尚未登记");
            }
            let result = write_response(stream, true, "SHUTTING_DOWN", "服务正在关闭");
            monitor.should_stop.store(true, Ordering::Release);
            if let Some(stop_tx) = stop_tx {
                let _ = stop_tx.send(());
            }
            result
        }
        _ => write_response(stream, false, "UNKNOWN_COMMAND", "未知输入命令"),
    }
}

/// 释放会话代理管道持有的键盘与鼠标按键。
fn release_held_inputs(stream: &mut Reply, request: &Request) -> Result<()> {
    anyhow::ensure!(
        request.keys.len() <= 128 && request.buttons.len() <= 8,
        "too many held inputs"
    );
    let result = request
        .keys
        .iter()
        .try_for_each(|(usage, modifiers)| input::inject_key_event(*usage, *modifiers, false))
        .and_then(|()| {
            request
                .buttons
                .iter()
                .try_for_each(|button| input::inject_mouse_button_state(button, false))
        });
    respond_result(
        stream,
        result,
        "INPUT_RELEASE_FAILED",
        "INPUT_RELEASED",
        "已释放输入状态",
    )
}

/// 窗口聚焦、进程聚焦、窗口动作与窗口切换。
fn dispatch_window_command(stream: &mut Reply, request: &Request) -> Result<()> {
    match request.command.as_str() {
        "focus_window" => {
            let result = request
                .hwnd
                .ok_or_else(|| anyhow::anyhow!("缺少窗口句柄"))
                .and_then(qinglink_windows::platform_windows::focus_window);
            respond_result(stream, result, "FOCUS_FAILED", "WINDOW_OK", "窗口已激活")
        }
        "focus_process" => {
            let result = request
                .pid
                .ok_or_else(|| anyhow::anyhow!("缺少进程 PID"))
                .and_then(qinglink_windows::platform_windows::focus_process);
            respond_result(stream, result, "FOCUS_FAILED", "WINDOW_OK", "窗口已激活")
        }
        "minimize_window" | "maximize_window" | "restore_window" | "close_window" => {
            let result = request
                .hwnd
                .ok_or_else(|| anyhow::anyhow!("缺少窗口句柄"))
                .and_then(|hwnd| match request.command.as_str() {
                    "minimize_window" => qinglink_windows::platform_windows::minimize_window(hwnd),
                    "maximize_window" => qinglink_windows::platform_windows::maximize_window(hwnd),
                    "restore_window" => qinglink_windows::platform_windows::restore_window(hwnd),
                    "close_window" => qinglink_windows::platform_windows::close_window(hwnd),
                    _ => unreachable!(),
                });
            respond_result(
                stream,
                result,
                "WINDOW_ACTION_FAILED",
                "WINDOW_OK",
                "窗口操作已完成",
            )
        }
        "switch_window" => {
            let direction = match request.direction.as_deref() {
                Some("next") => qinglink_protocol::WindowSwitchDirection::Next,
                Some("previous") => qinglink_protocol::WindowSwitchDirection::Previous,
                _ => {
                    return write_response(stream, false, "INVALID_DIRECTION", "窗口切换方向无效");
                }
            };
            respond_result(
                stream,
                qinglink_windows::platform_windows::switch_window(direction),
                "SWITCH_WINDOW_FAILED",
                "WINDOW_OK",
                "窗口已切换",
            )
        }
        _ => write_response(stream, false, "UNKNOWN_COMMAND", "未知输入命令"),
    }
}

pub(super) fn named_pipe_client_pid(pipe: HANDLE) -> Result<u32> {
    use windows::Win32::System::Pipes::GetNamedPipeClientProcessId;
    let mut pid = 0;
    unsafe {
        GetNamedPipeClientProcessId(pipe, &mut pid)?;
    }
    anyhow::ensure!(pid != 0, "named pipe client process ID is empty");
    Ok(pid)
}

pub(super) fn write_response(
    stream: &mut Reply,
    ok: bool,
    code: &str,
    message: &str,
) -> Result<()> {
    stream.send(&Response {
        ok,
        code,
        message,
        request_id: None,
    })
}
pub(super) fn respond_result<T>(
    stream: &mut Reply,
    result: Result<T>,
    error_code: &str,
    success_code: &str,
    success_message: &str,
) -> Result<()> {
    match result {
        Ok(_) => write_response(stream, true, success_code, success_message),
        Err(error) => write_response(stream, false, error_code, &error.to_string()),
    }
}

fn write_response_with_id(
    stream: &mut Reply,
    ok: bool,
    code: &str,
    message: &str,
    request_id: Option<&str>,
) -> Result<()> {
    stream.send(&Response {
        ok,
        code,
        message,
        request_id,
    })
}
