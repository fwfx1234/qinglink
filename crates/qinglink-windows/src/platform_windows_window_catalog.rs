// === 窗口管理 ===

//! 窗口枚举与回调：一次性枚举与 Alt+Tab 一致的顶层窗口，不轮询、调用完释放。

use std::sync::{Arc, Mutex};

use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM};
use windows::Win32::Graphics::Dwm::{DWMWA_CLOAKED, DwmGetWindowAttribute};
use windows::Win32::System::ProcessStatus::GetModuleFileNameExW;
use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GW_OWNER, GWL_EXSTYLE, GetForegroundWindow, GetWindow, GetWindowLongW,
    GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible, WS_EX_APPWINDOW, WS_EX_TOOLWINDOW,
};

use super::lock_unpoisoned;
use super::window_state::{is_fullscreen_window, is_topmost_window};

/// 窗口信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WindowInfo {
    /// 窗口句柄的数值形式。
    pub hwnd: usize,
    /// 窗口标题文本。
    pub title: String,
    /// 拥有该窗口的进程 ID。
    pub pid: u32,
    /// 进程可执行文件路径，无法获取时为 `None`。
    pub exe_path: Option<String>,
    /// 窗口当前是否可见。
    pub is_visible: bool,
    /// 该窗口是否为当前前台窗口。
    pub is_foreground: bool,
    /// 该窗口是否铺满其所在显示器。
    pub is_fullscreen: bool,
    /// 该窗口是否设置了置顶样式。
    pub is_topmost: bool,
}

/// 一次性枚举与 Alt+Tab 一致的顶层窗口（不轮询，调用完释放）。
pub fn enum_windows() -> Vec<WindowInfo> {
    let results: Arc<Mutex<Vec<WindowInfo>>> = Arc::new(Mutex::new(Vec::new()));

    #[allow(unsafe_op_in_unsafe_fn)]
    unsafe extern "system" fn enum_callback(hwnd: HWND, lparam: LPARAM) -> windows::core::BOOL {
        let results = &*(lparam.0 as *const Arc<Mutex<Vec<WindowInfo>>>);
        if is_alt_tab_window(hwnd) {
            let mut title_buf = [0u16; 512];
            let len = GetWindowTextW(hwnd, &mut title_buf);
            let title = String::from_utf16_lossy(&title_buf[..len as usize]);
            let mut pid: u32 = 0;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            let exe_path = if pid != 0 {
                let process = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid);
                if let Ok(handle) = process {
                    if !handle.is_invalid() {
                        let mut path_buf = [0u16; 260];
                        let len = GetModuleFileNameExW(Some(handle), None, &mut path_buf);
                        let _ = CloseHandle(handle);
                        if len > 0 {
                            Some(String::from_utf16_lossy(&path_buf[..len as usize]))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            };
            let hwnd_ptr = hwnd.0 as usize;
            let is_fullscreen = is_fullscreen_window(hwnd);
            let is_topmost = is_topmost_window(hwnd);
            lock_unpoisoned(results).push(WindowInfo {
                hwnd: hwnd_ptr,
                title,
                pid,
                exe_path,
                is_visible: true,
                is_foreground: false,
                is_fullscreen,
                is_topmost,
            });
        }
        windows::core::BOOL(1)
    }

    unsafe {
        let _ = EnumWindows(Some(enum_callback), LPARAM(&results as *const _ as isize));
    }

    let mut windows = Arc::try_unwrap(results)
        .unwrap_or_else(|_| unreachable!("window enumeration callbacks have completed"))
        .into_inner()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    // 统一标记前台窗口
    let foreground = unsafe { GetForegroundWindow().0 as usize };
    for w in &mut windows {
        if w.hwnd == foreground {
            w.is_foreground = true;
        }
    }

    windows
}

/// Fast list variant used by the mobile window switcher.  It deliberately
/// avoids opening every target process and avoids DWM/monitor queries; those
/// calls dominate list latency when many applications are open.
pub fn enum_windows_light() -> Vec<WindowInfo> {
    let results: Arc<Mutex<Vec<WindowInfo>>> = Arc::new(Mutex::new(Vec::new()));

    #[allow(unsafe_op_in_unsafe_fn)]
    unsafe extern "system" fn enum_callback(hwnd: HWND, lparam: LPARAM) -> windows::core::BOOL {
        let results = &*(lparam.0 as *const Arc<Mutex<Vec<WindowInfo>>>);
        if is_alt_tab_window(hwnd) {
            let mut title_buf = [0u16; 512];
            let len = GetWindowTextW(hwnd, &mut title_buf);
            let title = String::from_utf16_lossy(&title_buf[..len as usize]);
            let mut pid = 0u32;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            lock_unpoisoned(results).push(WindowInfo {
                hwnd: hwnd.0 as usize,
                title,
                pid,
                exe_path: None,
                is_visible: true,
                is_foreground: false,
                is_fullscreen: false,
                // TOPMOST is a window style bit and is cheap to query; keep
                // it in the light representation so Session 0 callers do
                // not lose the user's pin state through the session agent.
                is_topmost: is_topmost_window(hwnd),
            });
        }
        windows::core::BOOL(1)
    }

    unsafe {
        let _ = EnumWindows(Some(enum_callback), LPARAM(&results as *const _ as isize));
    }
    let mut windows = Arc::try_unwrap(results)
        .unwrap_or_else(|_| unreachable!("window enumeration callbacks have completed"))
        .into_inner()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let foreground = unsafe { GetForegroundWindow().0 as usize };
    for window in &mut windows {
        window.is_foreground = window.hwnd == foreground;
    }
    windows
}

/// Applies the window traits that determine whether a top-level window is an
/// Alt+Tab candidate.  This avoids exposing helper, owned, and off-desktop
/// windows that happen to be visible to `EnumWindows`.
fn is_alt_tab_window(hwnd: HWND) -> bool {
    unsafe {
        if !IsWindowVisible(hwnd).as_bool() {
            return false;
        }

        let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
        if ex_style & WS_EX_TOOLWINDOW.0 != 0 {
            return false;
        }

        let has_owner = GetWindow(hwnd, GW_OWNER)
            .map(|owner| !owner.0.is_null())
            .unwrap_or(false);
        if has_owner && ex_style & WS_EX_APPWINDOW.0 == 0 {
            return false;
        }

        !is_window_cloaked(hwnd)
    }
}

fn is_window_cloaked(hwnd: HWND) -> bool {
    unsafe {
        let mut cloaked = 0u32;
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED,
            &mut cloaked as *mut _ as *mut core::ffi::c_void,
            std::mem::size_of::<u32>() as u32,
        )
        .is_ok()
            && cloaked != 0
    }
}
