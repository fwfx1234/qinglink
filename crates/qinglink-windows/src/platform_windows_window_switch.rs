//! 窗口切换：按可见窗口的 z-order 切换前台窗口，行为接近 Alt+Tab。

use qinglink_protocol::WindowSwitchDirection;
use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

use super::window_actions::focus_window;
use super::window_catalog::enum_windows;

/// 按当前可见窗口的 z-order 切换前台窗口，行为接近 Alt+Tab。
///
/// # Errors
/// 当前没有可切换的可见顶层窗口（“No switchable windows”），或透传
/// [`focus_window`] 对目标窗口的错误（句柄失效或 Windows 拒绝激活）。
pub fn switch_window(direction: WindowSwitchDirection) -> anyhow::Result<()> {
    let windows = enum_windows();
    if windows.is_empty() {
        anyhow::bail!("No switchable windows");
    }

    let foreground = unsafe { GetForegroundWindow().0 as usize };
    let current_index = windows.iter().position(|w| w.hwnd == foreground);
    let target_index = match direction {
        WindowSwitchDirection::Next => current_index.map_or(0, |i| (i + 1) % windows.len()),
        WindowSwitchDirection::Previous => current_index.map_or(windows.len() - 1, |i| {
            if i == 0 { windows.len() - 1 } else { i - 1 }
        }),
    };

    focus_window(windows[target_index].hwnd)
}
