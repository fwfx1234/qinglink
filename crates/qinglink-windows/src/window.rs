use crate::{Result, WindowsCapabilityError};

/// 校验当前平台是否支持窗口控制能力。
///
/// # Errors
/// 当编译目标不是 Windows 时返回
/// [`WindowsCapabilityError::Unsupported`]（能力名为 `window control`）。
pub fn ensure_supported() -> Result<()> {
    super::is_supported()
        .then_some(())
        .ok_or(WindowsCapabilityError::Unsupported("window control"))
}

/// 隐藏指定句柄对应的原生窗口，并在调用后确认其确实不可见。
///
/// # Errors
/// 当调用 `ShowWindow(SW_HIDE)` 之后窗口仍然可见时，返回
/// [`WindowsCapabilityError::Unsupported`]（`"hide window failed"`）。
#[cfg(target_os = "windows")]
pub fn hide_native_window(handle: isize) -> Result<()> {
    use windows::Win32::{
        Foundation::HWND,
        UI::WindowsAndMessaging::{IsWindowVisible, SW_HIDE, ShowWindow},
    };

    let hwnd = HWND(handle as *mut _);

    // 检查调用前的窗口状态
    let was_visible = unsafe { IsWindowVisible(hwnd).as_bool() };
    tracing::debug!(handle, was_visible, "hide_native_window 调用前");

    // 调用 ShowWindow(SW_HIDE)
    let result = unsafe { ShowWindow(hwnd, SW_HIDE) };
    tracing::debug!(handle, result = result.0, "ShowWindow(SW_HIDE) 返回值");

    // 验证调用后的窗口状态
    let is_visible = unsafe { IsWindowVisible(hwnd).as_bool() };
    tracing::debug!(handle, is_visible, "hide_native_window 调用后");

    if is_visible {
        tracing::error!(
            handle,
            "窗口隐藏失败：调用 ShowWindow(SW_HIDE) 后窗口仍然可见"
        );
        return Err(crate::WindowsCapabilityError::Unsupported(
            "hide window failed",
        ));
    }

    tracing::info!(handle, "窗口已成功隐藏");
    Ok(())
}

/// 恢复并前置指定句柄对应的原生窗口，若仍不可见则记录告警。
///
/// # Errors
/// 当前实现不返回 `Err`：即使窗口在调用后仍不可见，也只记录一条 `warn`
/// 日志并返回 `Ok(())`。返回值保留 `Result` 以维持接口稳定。
#[cfg(target_os = "windows")]
pub fn show_native_window(handle: isize) -> Result<()> {
    use windows::Win32::{
        Foundation::HWND,
        UI::WindowsAndMessaging::{IsWindowVisible, SW_RESTORE, SetForegroundWindow, ShowWindow},
    };

    let hwnd = HWND(handle as *mut _);

    // 检查调用前的窗口状态
    let was_visible = unsafe { IsWindowVisible(hwnd).as_bool() };
    tracing::debug!(handle, was_visible, "show_native_window 调用前");

    // 调用 ShowWindow(SW_RESTORE)
    let result = unsafe { ShowWindow(hwnd, SW_RESTORE) };
    tracing::debug!(handle, result = result.0, "ShowWindow(SW_RESTORE) 返回值");

    // 设置前台窗口
    let _ = unsafe { SetForegroundWindow(hwnd) };

    // 验证调用后的窗口状态
    let is_visible = unsafe { IsWindowVisible(hwnd).as_bool() };
    tracing::debug!(handle, is_visible, "show_native_window 调用后");

    if !is_visible {
        tracing::warn!(
            handle,
            "窗口显示可能失败：调用 ShowWindow(SW_RESTORE) 后窗口仍然不可见"
        );
    } else {
        tracing::info!(handle, "窗口已成功显示");
    }

    Ok(())
}

/// 查询指定句柄对应的原生窗口当前是否可见。
#[cfg(target_os = "windows")]
pub fn is_window_visible(handle: isize) -> bool {
    use windows::Win32::{Foundation::HWND, UI::WindowsAndMessaging::IsWindowVisible};
    let visible = unsafe { IsWindowVisible(HWND(handle as *mut _)).as_bool() };
    tracing::trace!(handle, visible, "is_window_visible 查询结果");
    visible
}
