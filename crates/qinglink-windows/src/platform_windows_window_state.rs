//! 窗口信息采集：句柄有效性与置顶、全屏状态查询。

use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GWL_EXSTYLE, GetForegroundWindow, GetWindowLongW, GetWindowRect, IsWindow, WS_EX_TOPMOST,
};

pub(super) fn checked_hwnd(raw: usize) -> anyhow::Result<HWND> {
    let hwnd = HWND(raw as *mut _);
    unsafe {
        anyhow::ensure!(
            !hwnd.0.is_null() && IsWindow(Some(hwnd)).as_bool(),
            "窗口已不存在"
        );
    }
    Ok(hwnd)
}

/// 检测窗口是否置顶
pub(super) fn is_topmost_window(hwnd: HWND) -> bool {
    let ex_style = unsafe { GetWindowLongW(hwnd, GWL_EXSTYLE) };
    (ex_style as u32 & WS_EX_TOPMOST.0) != 0
}

/// 检测窗口是否全屏
pub(super) fn is_fullscreen_window(hwnd: HWND) -> bool {
    unsafe {
        let mut rect = std::mem::zeroed();
        if GetWindowRect(hwnd, &mut rect).is_err() {
            return false;
        }
        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if GetMonitorInfoW(monitor, &mut info).0 == 0 {
            return false;
        }
        rect.left == info.rcMonitor.left
            && rect.top == info.rcMonitor.top
            && rect.right == info.rcMonitor.right
            && rect.bottom == info.rcMonitor.bottom
    }
}

/// 判断当前前台窗口是否全屏。
pub fn is_foreground_fullscreen() -> bool {
    unsafe { is_fullscreen_window(GetForegroundWindow()) }
}
