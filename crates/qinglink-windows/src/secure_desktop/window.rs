//! 窗口枚举、焦点切换与窗口操作的门面转发。
//!
//! 设置 `QINGLINK_SESSION_AGENT` 时直接调用本进程的窗口能力，否则把命令转发给
//! 活动会话中的会话代理。

use anyhow::{Context, Result};

/// 以轻量方式枚举当前可见的顶层窗口，必要时经会话代理转发。
///
/// # Errors
/// 设置了 `QINGLINK_SESSION_AGENT` 时透传本地窗口枚举的错误；否则向会话代理请求
/// `list_windows`，代理请求失败或返回内容不是合法 JSON 窗口列表（带上下文
/// `invalid window list from session agent`）时返回错误。
#[cfg(target_os = "windows")]
pub fn enum_windows_light() -> Result<Vec<crate::platform_windows::WindowInfo>> {
    if std::env::var_os("QINGLINK_SESSION_AGENT").is_some() {
        return Ok(crate::platform_windows::enum_windows_light());
    }
    let message = super::agent_request_message("list_windows", None)?;
    serde_json::from_str(&message).context("invalid window list from session agent")
}

/// 将指定窗口句柄置为前台窗口，必要时经会话代理转发。
///
/// # Errors
/// 设置了 `QINGLINK_SESSION_AGENT` 时透传
/// `crate::platform_windows::focus_window` 的错误；否则向会话代理转发
/// `focus_window`，代理请求失败时返回错误。
#[cfg(target_os = "windows")]
pub fn focus_window(hwnd: usize) -> Result<()> {
    if std::env::var_os("QINGLINK_SESSION_AGENT").is_some() {
        return crate::platform_windows::focus_window(hwnd);
    }
    super::agent_window_request("focus_window", Some(hwnd), None, None)
}

/// 将指定进程的主窗口置为前台窗口，必要时经会话代理转发。
///
/// # Errors
/// 设置了 `QINGLINK_SESSION_AGENT` 时透传
/// `crate::platform_windows::focus_process` 的错误；否则向会话代理转发
/// `focus_process`，代理请求失败时返回错误。
#[cfg(target_os = "windows")]
pub fn focus_process(pid: u32) -> Result<()> {
    if std::env::var_os("QINGLINK_SESSION_AGENT").is_some() {
        return crate::platform_windows::focus_process(pid);
    }
    super::agent_window_request("focus_process", None, Some(pid), None)
}

/// 对指定窗口执行最小化/最大化/还原/关闭操作，必要时经会话代理转发。
///
/// # Errors
/// 设置了 `QINGLINK_SESSION_AGENT` 时，透传底层窗口操作（`minimize_window`、
/// `maximize_window`、`restore_window`、`close_window`）的错误；`command` 不属于
/// 这四个取值时返回“unsupported window command”。否则将命令转发给会话代理，
/// 代理请求失败时返回错误。
#[cfg(target_os = "windows")]
pub fn window_action(command: &str, hwnd: usize) -> Result<()> {
    if std::env::var_os("QINGLINK_SESSION_AGENT").is_some() {
        return match command {
            "minimize_window" => crate::platform_windows::minimize_window(hwnd),
            "maximize_window" => crate::platform_windows::maximize_window(hwnd),
            "restore_window" => crate::platform_windows::restore_window(hwnd),
            "close_window" => crate::platform_windows::close_window(hwnd),
            _ => anyhow::bail!("unsupported window command"),
        };
    }
    super::agent_window_request(command, Some(hwnd), None, None)
}

/// 按方向切换到上一个或下一个顶层窗口，必要时经会话代理转发。
///
/// # Errors
/// 设置了 `QINGLINK_SESSION_AGENT` 时透传
/// `crate::platform_windows::switch_window` 的错误；否则向会话代理转发
/// `switch_window`，代理请求失败时返回错误。
#[cfg(target_os = "windows")]
pub fn switch_window(direction: qinglink_protocol::WindowSwitchDirection) -> Result<()> {
    if std::env::var_os("QINGLINK_SESSION_AGENT").is_some() {
        return crate::platform_windows::switch_window(direction);
    }
    let direction = match direction {
        qinglink_protocol::WindowSwitchDirection::Next => "next",
        qinglink_protocol::WindowSwitchDirection::Previous => "previous",
    };
    super::agent_window_request("switch_window", None, None, Some(direction))
}

#[cfg(not(target_os = "windows"))]
pub fn focus_window(_: usize) -> Result<()> {
    anyhow::bail!("window focus is only supported on Windows")
}

#[cfg(not(target_os = "windows"))]
pub fn focus_process(_: u32) -> Result<()> {
    anyhow::bail!("process focus is only supported on Windows")
}

#[cfg(not(target_os = "windows"))]
pub fn window_action(_: &str, _: usize) -> Result<()> {
    anyhow::bail!("window actions are only supported on Windows")
}

#[cfg(not(target_os = "windows"))]
pub fn switch_window(_: qinglink_protocol::WindowSwitchDirection) -> Result<()> {
    anyhow::bail!("window switching is only supported on Windows")
}

#[cfg(not(target_os = "windows"))]
pub fn enum_windows_light() -> Result<Vec<()>> {
    anyhow::bail!("window enumeration is only supported on Windows")
}
