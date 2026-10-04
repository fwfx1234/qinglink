//! 窗口动作：激活、最小化、最大化、还原、关闭、移动与置顶。

use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
use windows::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, GetForegroundWindow, GetWindowThreadProcessId, HWND_NOTOPMOST, HWND_TOPMOST,
    IsWindow, MSG, PM_NOREMOVE, PeekMessageW, PostMessageW, SW_RESTORE, SW_SHOWMAXIMIZED,
    SW_SHOWMINIMIZED, SWP_NOSIZE, SWP_NOZORDER, SWP_SHOWWINDOW, SetForegroundWindow, SetWindowPos,
    ShowWindow, WM_CLOSE,
};

use super::window_catalog::enum_windows_light;
use super::window_state::checked_hwnd;

/// 激活指定窗口
///
/// # Errors
/// 句柄为空或窗口已不存在（“窗口已不存在”）；或 `SetForegroundWindow` 之后目标窗口
/// 仍不是前台窗口（“Windows 拒绝激活目标窗口”）时返回错误。
pub fn focus_window(hwnd: usize) -> anyhow::Result<()> {
    let hwnd = checked_hwnd(hwnd)?;
    unsafe {
        let _ = ShowWindow(hwnd, SW_RESTORE);

        // The WebSocket handler runs on a Tokio worker thread. Ensure it has
        // a message queue before using AttachThreadInput with GUI threads.
        let mut message = MSG::default();
        let _ = PeekMessageW(&mut message, None, 0, 0, PM_NOREMOVE);
        let caller_thread = GetCurrentThreadId();
        let foreground = GetForegroundWindow();
        let foreground_thread = if foreground.0.is_null() {
            0
        } else {
            GetWindowThreadProcessId(foreground, None)
        };
        let target_thread = GetWindowThreadProcessId(hwnd, None);
        let attached_foreground = foreground_thread != 0
            && foreground_thread != caller_thread
            && AttachThreadInput(caller_thread, foreground_thread, true).as_bool();
        let attached_target = target_thread != 0
            && target_thread != caller_thread
            && target_thread != foreground_thread
            && AttachThreadInput(caller_thread, target_thread, true).as_bool();

        // BringWindowToTop can fail for windows owned by a different thread or
        // integrity level even though SetForegroundWindow still succeeds.
        // Treat it as best-effort instead of aborting the whole activation.
        let _ = BringWindowToTop(hwnd);
        let activated = SetForegroundWindow(hwnd).as_bool();

        if attached_target {
            let _ = AttachThreadInput(caller_thread, target_thread, false);
        }
        if attached_foreground {
            let _ = AttachThreadInput(caller_thread, foreground_thread, false);
        }

        if !activated && GetForegroundWindow() != hwnd {
            anyhow::bail!("Windows 拒绝激活目标窗口");
        }
    }
    Ok(())
}

/// Wait for the first visible top-level window owned by a process and activate it.
/// GUI applications often create their process before creating the actual window.
///
/// # Errors
/// Returns an error when no visible top-level window of `pid` appears within
/// roughly four seconds ("进程 {pid} 未创建可激活的窗口"), and propagates any
/// error from [`focus_window`] for the window that was found.
pub fn focus_process(pid: u32) -> anyhow::Result<()> {
    const ATTEMPTS: u32 = 40;
    const INTERVAL: std::time::Duration = std::time::Duration::from_millis(100);

    for _ in 0..ATTEMPTS {
        if let Some(window) = enum_windows_light()
            .into_iter()
            .find(|window| window.pid == pid && window.is_visible)
        {
            return focus_window(window.hwnd);
        }
        std::thread::sleep(INTERVAL);
    }

    anyhow::bail!("进程 {pid} 未创建可激活的窗口")
}

/// 最小化窗口
///
/// # Errors
/// 句柄为空或窗口已不存在（“窗口已不存在”）时返回错误。
pub fn minimize_window(hwnd: usize) -> anyhow::Result<()> {
    let hwnd = checked_hwnd(hwnd)?;
    unsafe {
        let _ = ShowWindow(hwnd, SW_SHOWMINIMIZED);
    }
    Ok(())
}

/// 最大化窗口
///
/// # Errors
/// 句柄为空或窗口已不存在（“窗口已不存在”）时返回错误。
pub fn maximize_window(hwnd: usize) -> anyhow::Result<()> {
    let hwnd = checked_hwnd(hwnd)?;
    unsafe {
        let _ = ShowWindow(hwnd, SW_SHOWMAXIMIZED);
    }
    Ok(())
}

/// 还原窗口
///
/// # Errors
/// 句柄为空或窗口已不存在（“窗口已不存在”）时返回错误。
pub fn restore_window(hwnd: usize) -> anyhow::Result<()> {
    let hwnd = checked_hwnd(hwnd)?;
    unsafe {
        let _ = ShowWindow(hwnd, SW_RESTORE);
    }
    Ok(())
}

/// 关闭窗口。
///
/// 优先发送 `WM_CLOSE`，让应用自行优雅退出。刚启动的游戏等应用在初始化
/// 期间常处于忙等待或自绘消息循环，无法及时处理关闭消息，导致"关闭窗口"
/// 看起来无效；因此短暂等待后若窗口仍然存在，就强制结束其进程，行为与
/// 任务管理器的"结束任务"一致。
///
/// # Errors
/// 句柄为空或窗口已不存在（“窗口已不存在”）；`PostMessageW(WM_CLOSE)` 失败；或
/// 宽限期后窗口仍存在且 `crate::process::kill(pid)` 失败时返回错误。
pub fn close_window(hwnd: usize) -> anyhow::Result<()> {
    let hwnd = checked_hwnd(hwnd)?;
    let mut pid: u32 = 0;
    unsafe {
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0))?;
    }

    // 给应用约 1.5 秒响应 WM_CLOSE；若窗口仍在则回退到结束进程。
    const GRACE_STEPS: u32 = 15;
    const GRACE_STEP: std::time::Duration = std::time::Duration::from_millis(100);
    for _ in 0..GRACE_STEPS {
        std::thread::sleep(GRACE_STEP);
        if !unsafe { IsWindow(Some(hwnd)).as_bool() } {
            return Ok(());
        }
    }

    if pid != 0 {
        crate::process::kill(pid)?;
    }
    Ok(())
}

/// 移动/调整窗口大小
///
/// # Errors
/// 句柄为空或窗口已不存在（“窗口已不存在”），或 `SetWindowPos` 调用失败时返回错误。
pub fn move_window(hwnd: usize, x: i32, y: i32, width: u32, height: u32) -> anyhow::Result<()> {
    let hwnd = checked_hwnd(hwnd)?;
    unsafe {
        SetWindowPos(
            hwnd,
            None,
            x,
            y,
            width as i32,
            height as i32,
            SWP_NOZORDER | SWP_SHOWWINDOW,
        )?;
    }
    Ok(())
}

/// 设置窗口置顶/取消置顶
///
/// # Errors
/// 句柄为空或窗口已不存在（“窗口已不存在”），或 `SetWindowPos` 调用失败时返回错误。
pub fn set_always_on_top(hwnd: usize, enable: bool) -> anyhow::Result<()> {
    let hwnd = checked_hwnd(hwnd)?;
    let z_order = if enable { HWND_TOPMOST } else { HWND_NOTOPMOST };
    unsafe {
        SetWindowPos(hwnd, Some(z_order), 0, 0, 0, 0, SWP_NOSIZE | SWP_SHOWWINDOW)?;
    }
    Ok(())
}
