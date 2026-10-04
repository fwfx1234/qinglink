//! 窗口管理：窗口枚举、激活、最小化/最大化/恢复/关闭、移动、置顶与切换。

#[cfg(not(target_os = "windows"))]
use super::stub;
#[cfg(target_os = "windows")]
use super::windows;

#[cfg(not(target_os = "windows"))]
pub use super::stub::WindowInfo;
#[cfg(target_os = "windows")]
pub use super::windows::WindowInfo;

// === 窗口管理 ===

/// 枚举当前会话中所有顶层窗口。
#[cfg(target_os = "windows")]
pub fn enum_windows() -> Vec<WindowInfo> {
    if qinglink_windows::secure_desktop::running_in_session_zero() {
        return qinglink_windows::secure_desktop::enum_windows_light().unwrap_or_default();
    }
    windows::enum_windows()
}

/// 非 Windows 平台没有窗口系统，返回空列表。
#[cfg(not(target_os = "windows"))]
pub fn enum_windows() -> Vec<WindowInfo> {
    stub::enum_windows()
}

/// 以较轻量的方式枚举当前会话的顶层窗口。
#[cfg(target_os = "windows")]
pub fn enum_windows_light() -> Vec<WindowInfo> {
    if qinglink_windows::secure_desktop::running_in_session_zero() {
        return qinglink_windows::secure_desktop::enum_windows_light().unwrap_or_default();
    }
    windows::enum_windows_light()
}

/// 非 Windows 平台没有窗口系统，返回空列表。
#[cfg(not(target_os = "windows"))]
pub fn enum_windows_light() -> Vec<WindowInfo> {
    stub::enum_windows_light()
}

/// 获取简化窗口列表，用于 WebSocket API。
pub fn enum_windows_list() -> Vec<crate::protocol::requests::WindowInfo> {
    #[cfg(target_os = "windows")]
    if qinglink_windows::secure_desktop::running_in_session_zero()
        && let Ok(windows) = qinglink_windows::secure_desktop::enum_windows_light()
    {
        return windows
            .into_iter()
            .map(|w| crate::protocol::requests::WindowInfo {
                id: w.hwnd,
                title: w.title,
                pid: w.pid,
                exe_path: w.exe_path,
                is_visible: w.is_visible,
                is_foreground: w.is_foreground,
                is_fullscreen: w.is_fullscreen,
                is_topmost: w.is_topmost,
            })
            .collect();
    }
    enum_windows_light()
        .into_iter()
        .map(|w| crate::protocol::requests::WindowInfo {
            id: w.hwnd,
            title: w.title,
            pid: w.pid,
            exe_path: w.exe_path,
            is_visible: w.is_visible,
            is_foreground: w.is_foreground,
            is_fullscreen: w.is_fullscreen,
            is_topmost: w.is_topmost,
        })
        .collect()
}

/// 将指定窗口置于前台并获取焦点。
///
/// # Errors
///
/// 当窗口句柄无效或窗口已不存在、Windows 拒绝激活目标窗口，或底层
/// `SetForegroundWindow` 相关调用失败时返回 `Err`。
#[cfg(target_os = "windows")]
pub fn focus_window(hwnd: usize) -> anyhow::Result<()> {
    if qinglink_windows::secure_desktop::running_in_session_zero() {
        return qinglink_windows::secure_desktop::focus_window(hwnd);
    }
    windows::focus_window(hwnd)
}

/// 非 Windows 平台不支持窗口操作，返回不支持错误。
///
/// # Errors
///
/// 非 Windows 平台恒返回不支持错误（该操作仅支持 Windows）。
#[cfg(not(target_os = "windows"))]
pub fn focus_window(hwnd: usize) -> anyhow::Result<()> {
    stub::focus_window(hwnd)
}

/// 最小化指定窗口。
///
/// # Errors
///
/// 当窗口句柄无效或窗口已不存在时返回 `Err`（`ShowWindow` 自身的返回值被
/// 忽略，不视为失败）。
#[cfg(target_os = "windows")]
pub fn minimize_window(hwnd: usize) -> anyhow::Result<()> {
    if qinglink_windows::secure_desktop::running_in_session_zero() {
        return qinglink_windows::secure_desktop::window_action("minimize_window", hwnd);
    }
    windows::minimize_window(hwnd)
}

/// 非 Windows 平台不支持窗口操作，返回不支持错误。
///
/// # Errors
///
/// 非 Windows 平台恒返回不支持错误（该操作仅支持 Windows）。
#[cfg(not(target_os = "windows"))]
pub fn minimize_window(hwnd: usize) -> anyhow::Result<()> {
    stub::minimize_window(hwnd)
}

/// 最大化指定窗口。
///
/// # Errors
///
/// 当窗口句柄无效或窗口已不存在时返回 `Err`（`ShowWindow` 自身的返回值被
/// 忽略，不视为失败）。
#[cfg(target_os = "windows")]
pub fn maximize_window(hwnd: usize) -> anyhow::Result<()> {
    if qinglink_windows::secure_desktop::running_in_session_zero() {
        return qinglink_windows::secure_desktop::window_action("maximize_window", hwnd);
    }
    windows::maximize_window(hwnd)
}

/// 非 Windows 平台不支持窗口操作，返回不支持错误。
///
/// # Errors
///
/// 非 Windows 平台恒返回不支持错误（该操作仅支持 Windows）。
#[cfg(not(target_os = "windows"))]
pub fn maximize_window(hwnd: usize) -> anyhow::Result<()> {
    stub::maximize_window(hwnd)
}

/// 将指定窗口从最大化或最小化状态恢复。
///
/// # Errors
///
/// 当窗口句柄无效或窗口已不存在时返回 `Err`（`ShowWindow` 自身的返回值被
/// 忽略，不视为失败）。
#[cfg(target_os = "windows")]
pub fn restore_window(hwnd: usize) -> anyhow::Result<()> {
    if qinglink_windows::secure_desktop::running_in_session_zero() {
        return qinglink_windows::secure_desktop::window_action("restore_window", hwnd);
    }
    windows::restore_window(hwnd)
}

/// 非 Windows 平台不支持窗口操作，返回不支持错误。
///
/// # Errors
///
/// 非 Windows 平台恒返回不支持错误（该操作仅支持 Windows）。
#[cfg(not(target_os = "windows"))]
pub fn restore_window(hwnd: usize) -> anyhow::Result<()> {
    stub::restore_window(hwnd)
}

/// 关闭指定窗口。
///
/// # Errors
///
/// 当窗口句柄无效或窗口已不存在、`PostMessageW` 失败，或超时后回退结束进程
/// 失败时返回 `Err`。
#[cfg(target_os = "windows")]
pub fn close_window(hwnd: usize) -> anyhow::Result<()> {
    if qinglink_windows::secure_desktop::running_in_session_zero() {
        return qinglink_windows::secure_desktop::window_action("close_window", hwnd);
    }
    windows::close_window(hwnd)
}

/// 非 Windows 平台不支持窗口操作，返回不支持错误。
///
/// # Errors
///
/// 非 Windows 平台恒返回不支持错误（该操作仅支持 Windows）。
#[cfg(not(target_os = "windows"))]
pub fn close_window(hwnd: usize) -> anyhow::Result<()> {
    stub::close_window(hwnd)
}

platform_fn_args!(
    /// 移动窗口并调整其位置与尺寸。
    ///
    /// # Errors
    ///
    /// 当窗口句柄无效或窗口已不存在，或 `SetWindowPos` 调用失败时返回 `Err`；
    /// 非 Windows 平台恒返回不支持错误。
    move_window, anyhow::Result<()>, hwnd: usize, x: i32, y: i32, width: u32, height: u32
);
platform_fn_args!(
    /// 设置或取消窗口置顶。
    ///
    /// # Errors
    ///
    /// 当窗口句柄无效或窗口已不存在，或 `SetWindowPos` 调用失败时返回 `Err`；
    /// 非 Windows 平台恒返回不支持错误。
    set_always_on_top, anyhow::Result<()>, hwnd: usize, enable: bool
);

/// 按方向在窗口之间切换（下一个或上一个）。
///
/// # Errors
///
/// 当没有可切换的窗口、目标窗口激活失败，或底层 Win32 调用失败时返回 `Err`。
#[cfg(target_os = "windows")]
pub fn switch_window(
    direction: crate::protocol::requests::WindowSwitchDirection,
) -> anyhow::Result<()> {
    if qinglink_windows::secure_desktop::running_in_session_zero() {
        return qinglink_windows::secure_desktop::switch_window(match direction {
            crate::protocol::requests::WindowSwitchDirection::Next => {
                qinglink_protocol::WindowSwitchDirection::Next
            }
            crate::protocol::requests::WindowSwitchDirection::Previous => {
                qinglink_protocol::WindowSwitchDirection::Previous
            }
        });
    }
    windows::switch_window(match direction {
        crate::protocol::requests::WindowSwitchDirection::Next => {
            qinglink_protocol::WindowSwitchDirection::Next
        }
        crate::protocol::requests::WindowSwitchDirection::Previous => {
            qinglink_protocol::WindowSwitchDirection::Previous
        }
    })
}

/// 非 Windows 平台不支持窗口切换，返回不支持错误。
///
/// # Errors
///
/// 非 Windows 平台恒返回不支持错误（该操作仅支持 Windows）。
#[cfg(not(target_os = "windows"))]
pub fn switch_window(
    direction: crate::protocol::requests::WindowSwitchDirection,
) -> anyhow::Result<()> {
    stub::switch_window(direction)
}
