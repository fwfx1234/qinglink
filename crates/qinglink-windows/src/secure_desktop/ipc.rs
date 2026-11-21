//! 会话代理请求模式与有界传输调用。

use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::sync::atomic::AtomicBool;
use std::time::Duration;

#[cfg(target_os = "windows")]
fn request<T: serde::de::DeserializeOwned>(path: &str, value: &impl Serialize) -> Result<T> {
    let cancelled = AtomicBool::new(false);
    crate::ipc::request(path, value, Duration::from_secs(3), &cancelled)
}

#[cfg(target_os = "windows")]
fn request_with_timeout<T: serde::de::DeserializeOwned>(
    path: &str,
    value: &impl Serialize,
    timeout_message: &'static str,
) -> Result<T> {
    request(path, value).map_err(|error| {
        if error.to_string().contains("timed out") {
            anyhow::anyhow!(timeout_message)
        } else {
            error
        }
    })
}

#[cfg(target_os = "windows")]
#[derive(Serialize)]
struct StatusRequest<'a> {
    version: u32,
    command: &'a str,
}

#[cfg(target_os = "windows")]
#[derive(Deserialize)]
struct StatusResponse {
    ok: bool,
    #[serde(default)]
    message: String,
    #[serde(default)]
    desktop: Option<String>,
    #[serde(default)]
    locked: Option<bool>,
    #[serde(default)]
    input_ready: Option<bool>,
    #[serde(default)]
    sas_supported: Option<bool>,
    #[serde(default)]
    session_id: Option<u32>,
    #[serde(default)]
    agent_state: Option<String>,
    #[serde(default)]
    uac_detected: Option<bool>,
    #[serde(default)]
    token_integrity_level: Option<u32>,
    #[serde(default)]
    last_error: Option<String>,
}

#[cfg(target_os = "windows")]
pub(crate) fn agent_status_request(session_id: u32) -> Result<super::SecureDesktopStatus> {
    let pipe = format!(r"\\.\pipe\QingLinkSessionAgent-{session_id}");
    let response: StatusResponse = request(
        &pipe,
        &StatusRequest {
            version: 1,
            command: "status",
        },
    )?;
    ensure!(
        response.ok,
        "{}",
        if response.message.is_empty() {
            "agent status failed"
        } else {
            &response.message
        }
    );
    Ok(super::SecureDesktopStatus {
        desktop: response.desktop.unwrap_or_else(|| "unknown".into()),
        locked: response.locked.unwrap_or(true),
        input_ready: response.input_ready.unwrap_or(false),
        sas_supported: response.sas_supported.unwrap_or(true),
        session_id: response.session_id.unwrap_or(session_id),
        agent_state: response.agent_state.unwrap_or_else(|| "ready".into()),
        uac_detected: response.uac_detected.unwrap_or(false),
        token_integrity_level: response.token_integrity_level.unwrap_or(0),
        last_error: response.last_error.unwrap_or_default(),
    })
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn agent_status_request(_: u32) -> Result<super::SecureDesktopStatus> {
    anyhow::bail!("secure desktop agent is only supported on Windows")
}

#[cfg(target_os = "windows")]
#[derive(Serialize)]
struct WindowRequest<'a> {
    version: u32,
    command: &'a str,
    hwnd: Option<usize>,
    pid: Option<u32>,
    direction: Option<&'a str>,
}

#[cfg(target_os = "windows")]
#[derive(Deserialize)]
struct UnitResponse {
    ok: bool,
    message: String,
}

#[cfg(target_os = "windows")]
pub(crate) fn agent_window_request(
    command: &str,
    hwnd: Option<usize>,
    pid: Option<u32>,
    direction: Option<&str>,
) -> Result<()> {
    use windows::Win32::System::RemoteDesktop::WTSGetActiveConsoleSessionId;
    let session = super::active_interactive_session()
        .unwrap_or_else(|| unsafe { WTSGetActiveConsoleSessionId() });
    if session == u32::MAX {
        anyhow::bail!("no active console session");
    }
    let pipe = format!(r"\\.\pipe\QingLinkSessionAgent-{session}");
    let response: UnitResponse = request_with_timeout(
        &pipe,
        &WindowRequest {
            version: 1,
            command,
            hwnd,
            pid,
            direction,
        },
        "AGENT_REQUEST_TIMEOUT: 窗口操作服务响应超时",
    )?;
    ensure!(response.ok, "{}", response.message);
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn agent_window_request(
    _: &str,
    _: Option<usize>,
    _: Option<u32>,
    _: Option<&str>,
) -> Result<()> {
    anyhow::bail!("secure desktop agent is only supported on Windows")
}

#[cfg(target_os = "windows")]
#[derive(Serialize)]
struct AgentRequest<'a> {
    version: u32,
    command: &'a str,
    text: Option<&'a str>,
    dx: Option<i32>,
    dy: Option<i32>,
    button: Option<&'a str>,
    pressed: Option<bool>,
    delta: Option<i32>,
}

#[cfg(target_os = "windows")]
pub(crate) fn agent_request(
    command: &str,
    text: Option<&str>,
    delta: Option<(i32, i32)>,
    button: Option<&str>,
    pressed: Option<bool>,
) -> Result<()> {
    use windows::Win32::System::RemoteDesktop::WTSGetActiveConsoleSessionId;
    let session = super::active_interactive_session()
        .unwrap_or_else(|| unsafe { WTSGetActiveConsoleSessionId() });
    if session == u32::MAX {
        anyhow::bail!("no active console session");
    }
    tracing::debug!(
        session,
        command,
        "agent_request: sending command to session agent"
    );
    let pipe = format!(r"\\.\pipe\QingLinkSessionAgent-{session}");
    let response: UnitResponse = request_with_timeout(
        &pipe,
        &AgentRequest {
            version: 1,
            command,
            text,
            dx: delta.map(|value| value.0),
            dy: delta.map(|value| value.1),
            button,
            pressed,
            delta: (command == "mouse_wheel")
                .then(|| delta.map(|value| value.0))
                .flatten(),
        },
        "AGENT_REQUEST_TIMEOUT: 输入服务响应超时",
    )?;
    ensure!(response.ok, "{}", response.message);
    if matches!(command, "mouse_button" | "mouse_button_state") {
        tracing::info!(command, session, "agent mouse button request delivered");
    } else {
        tracing::debug!(command, session, "agent_request succeeded");
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn agent_request(
    _: &str,
    _: Option<&str>,
    _: Option<(i32, i32)>,
    _: Option<&str>,
    _: Option<bool>,
) -> Result<()> {
    anyhow::bail!("secure desktop agent is only supported on Windows")
}

#[cfg(target_os = "windows")]
#[derive(Serialize)]
struct HidRequest {
    version: u32,
    command: &'static str,
    usage: u8,
    modifiers: u8,
    pressed: bool,
}

#[cfg(target_os = "windows")]
pub(crate) fn agent_hid_key(usage: u8, modifiers: u8, pressed: bool) -> Result<()> {
    use windows::Win32::System::RemoteDesktop::WTSGetActiveConsoleSessionId;
    let session = super::active_interactive_session()
        .unwrap_or_else(|| unsafe { WTSGetActiveConsoleSessionId() });
    if session == u32::MAX {
        anyhow::bail!("no active console session");
    }
    let pipe = format!(r"\\.\pipe\QingLinkSessionAgent-{session}");
    let response: UnitResponse = request_with_timeout(
        &pipe,
        &HidRequest {
            version: 1,
            command: "key_event",
            usage,
            modifiers,
            pressed,
        },
        "AGENT_REQUEST_TIMEOUT: 键盘输入服务响应超时",
    )?;
    ensure!(response.ok, "{}", response.message);
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn agent_hid_key(_: u8, _: u8, _: bool) -> Result<()> {
    anyhow::bail!("secure desktop agent is only supported on Windows")
}

#[cfg(target_os = "windows")]
#[derive(Serialize)]
struct MessageRequest<'a> {
    version: u32,
    command: &'a str,
    text: Option<&'a str>,
}

/// 活动控制台会话的会话代理管道路径。
///
/// # Errors
/// 取不到活动会话（`u32::MAX`）时返回 “no active console session”。
#[cfg(target_os = "windows")]
fn agent_pipe_path() -> Result<String> {
    use windows::Win32::System::RemoteDesktop::WTSGetActiveConsoleSessionId;
    let session = super::active_interactive_session()
        .unwrap_or_else(|| unsafe { WTSGetActiveConsoleSessionId() });
    ensure!(session != u32::MAX, "no active console session");
    Ok(format!(r"\\.\pipe\QingLinkSessionAgent-{session}"))
}

#[cfg(target_os = "windows")]
fn agent_request_message_inner(command: &str, text: Option<&str>) -> Result<String> {
    let pipe = agent_pipe_path()?;
    let response: UnitResponse = request_with_timeout(
        &pipe,
        &MessageRequest {
            version: 1,
            command,
            text,
        },
        "AGENT_REQUEST_TIMEOUT: 窗口列表服务响应超时",
    )?;
    ensure!(response.ok, "{}", response.message);
    Ok(response.message)
}

#[cfg(target_os = "windows")]
pub(crate) fn agent_request_message(command: &str, text: Option<&str>) -> Result<String> {
    agent_request_message_inner(command, text)
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn agent_request_message(_: &str, _: Option<&str>) -> Result<String> {
    anyhow::bail!("secure desktop agent is only supported on Windows")
}

/// 通过会话代理枚举显示器；`message` 是 `DisplayInfoResponse` 数组的 JSON。
///
/// Session 0 的服务进程没有交互桌面，`EnumDisplayDevicesW` 在这里枚举不到任何
/// “连接到桌面”的显示器，所以必须在用户会话里执行。
#[cfg(target_os = "windows")]
pub(crate) fn agent_display_list() -> Result<String> {
    let pipe = agent_pipe_path()?;
    let response: UnitResponse = request_with_timeout(
        &pipe,
        &MessageRequest {
            version: 1,
            command: "get_displays",
            text: None,
        },
        "AGENT_REQUEST_TIMEOUT: 显示器服务响应超时",
    )?;
    ensure!(response.ok, "{}", response.message);
    Ok(response.message)
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn agent_display_list() -> Result<String> {
    anyhow::bail!("display control is only supported on Windows")
}

#[cfg(target_os = "windows")]
#[derive(Serialize)]
struct DisplayModeRequest<'a> {
    version: u32,
    command: &'a str,
    display_id: &'a str,
    width: u32,
    height: u32,
    refresh_millihz: u32,
}

/// 请求会话代理切换指定显示器的分辨率与刷新率。
#[cfg(target_os = "windows")]
pub(crate) fn agent_set_display_mode(
    display_id: &str,
    width: u32,
    height: u32,
    refresh_millihz: u32,
) -> Result<()> {
    let pipe = agent_pipe_path()?;
    let response: UnitResponse = request_with_timeout(
        &pipe,
        &DisplayModeRequest {
            version: 1,
            command: "set_display_mode",
            display_id,
            width,
            height,
            refresh_millihz,
        },
        "AGENT_REQUEST_TIMEOUT: 显示器服务响应超时",
    )?;
    ensure!(response.ok, "{}", response.message);
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn agent_set_display_mode(_: &str, _: u32, _: u32, _: u32) -> Result<()> {
    anyhow::bail!("display control is only supported on Windows")
}
