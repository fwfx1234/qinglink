//! 前台窗口信息、全屏判定与前台窗口变化钩子。

use super::platform_impl;

use crate::protocol::responses::ForegroundResponse;

#[cfg(target_os = "windows")]
use ::windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

#[cfg(target_os = "windows")]
use super::windows;

/// 获取当前前台窗口的进程、标题与句柄信息。
///
/// # Errors
///
/// 当前没有前台窗口（含 Session 0 下枚举不到前台窗口），或底层读取前台窗口
/// 信息失败时返回 `Err`；非 Windows 平台恒返回不支持错误。
pub fn get_foreground_window_info() -> anyhow::Result<ForegroundResponse> {
    #[cfg(target_os = "windows")]
    if qinglink_windows::secure_desktop::running_in_session_zero() {
        let window = qinglink_windows::secure_desktop::enum_windows_light()?
            .into_iter()
            .find(|window| window.is_foreground)
            .ok_or_else(|| anyhow::anyhow!("No foreground window"))?;
        let process_name = window
            .exe_path
            .as_deref()
            .map(std::path::Path::new)
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_string();
        return Ok(ForegroundResponse {
            pid: window.pid,
            title: window.title,
            path: window.exe_path,
            process_name,
            hwnd: window.hwnd,
        });
    }
    platform_impl::get_foreground_window_info().map(|value| ForegroundResponse {
        pid: value.pid,
        title: value.title,
        path: value.path,
        process_name: value.process_name,
        hwnd: value.hwnd,
    })
}

/// 判断当前前台窗口是否处于全屏状态。
pub fn is_foreground_fullscreen() -> bool {
    platform_impl::is_foreground_fullscreen()
}

/// 获取当前前台窗口句柄。
#[cfg(target_os = "windows")]
pub fn get_foreground_window() -> Option<usize> {
    if qinglink_windows::secure_desktop::running_in_session_zero() {
        return get_foreground_window_info().ok().map(|info| info.hwnd);
    }
    let hwnd = unsafe { GetForegroundWindow() };
    (!hwnd.0.is_null()).then_some(hwnd.0 as usize)
}

/// 非 Windows 平台没有前台窗口概念，恒返回 `None`。
#[cfg(not(target_os = "windows"))]
pub fn get_foreground_window() -> Option<usize> {
    None
}

// === 窗口监控钩子 ===

/// 注册前台窗口变化钩子，窗口事件发生时回调给定闭包。
///
/// # Errors
///
/// 当 `SetWinEventHook` 注册钩子失败时返回 `Err`（此时已保存的回调状态会被清理）。
#[cfg(target_os = "windows")]
pub fn set_foreground_hook(
    callback: Box<dyn Fn(isize) + Send + Sync>,
) -> anyhow::Result<::windows::Win32::UI::Accessibility::HWINEVENTHOOK> {
    windows::set_foreground_hook(callback)
}

/// 非 Windows 平台不支持窗口事件钩子，返回不支持错误。
///
/// # Errors
///
/// 非 Windows 平台恒返回不支持错误（该操作仅支持 Windows）。
#[cfg(not(target_os = "windows"))]
pub fn set_foreground_hook(_callback: Box<dyn Fn(isize) + Send + Sync>) -> anyhow::Result<usize> {
    anyhow::bail!("SetWinEventHook is only supported on Windows")
}

/// 注销先前注册的前台窗口变化钩子。
#[cfg(target_os = "windows")]
pub fn unhook_foreground(hook: ::windows::Win32::UI::Accessibility::HWINEVENTHOOK) {
    windows::unhook_foreground(hook)
}

/// 非 Windows 平台没有需要注销的钩子，此函数为空实现。
#[cfg(not(target_os = "windows"))]
pub fn unhook_foreground(_hook: usize) {}
