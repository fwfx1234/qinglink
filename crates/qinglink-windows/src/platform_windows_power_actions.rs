//! 电源动作：关机、重启、睡眠/休眠、注销、锁定，以及关机收尾用的控制台控制处理器。

/// 关闭系统，转发到 `crate::power::shutdown`。
///
/// # Errors
/// 透传 [`crate::power::shutdown`] 的错误：`delay_secs` 超出 `u32` 表示范围、
/// 无法启用 `SeShutdownPrivilege`，或 `InitiateSystemShutdownExW` 调用失败时返回错误。
pub fn shutdown(force: bool, delay_secs: u64) -> anyhow::Result<()> {
    crate::power::shutdown(force, delay_secs)
}

/// 让系统睡眠或休眠，转发到 `crate::power::sleep`。
///
/// # Errors
/// 透传 [`crate::power::sleep`] 的错误：`SetSuspendState` 返回失败时返回错误。
pub fn sleep(hibernate: bool) -> anyhow::Result<()> {
    crate::power::sleep(hibernate)
}

/// 重启系统，转发到 `crate::power::restart`。
///
/// # Errors
/// 透传 [`crate::power::restart`] 的错误：`delay_secs` 超出 `u32` 表示范围、
/// 无法启用 `SeShutdownPrivilege`，或 `InitiateSystemShutdownExW` 调用失败时返回错误。
pub fn restart(force: bool, delay_secs: u64) -> anyhow::Result<()> {
    crate::power::restart(force, delay_secs)
}

/// 注销当前用户会话，转发到 `crate::power::logoff`。
///
/// # Errors
/// 透传 [`crate::power::logoff`] 的错误：Session 0 路径下取不到活动交互式会话或
/// `WTSLogoffSession` 失败，非 Session 0 路径下 `ExitWindowsEx` 失败时返回错误。
pub fn logoff() -> anyhow::Result<()> {
    crate::power::logoff()
}

/// # Safety
/// Windows invokes this callback from the console control dispatcher. It only
/// reads the event code and acknowledges the shutdown/logoff event.
pub unsafe extern "system" fn console_ctrl_handler(ctrl_type: u32) -> windows::core::BOOL {
    use windows::Win32::System::Console::*;
    match ctrl_type {
        CTRL_SHUTDOWN_EVENT | CTRL_LOGOFF_EVENT => {
            std::thread::sleep(std::time::Duration::from_secs(2));
            windows::core::BOOL(1)
        }
        _ => windows::core::BOOL(0),
    }
}

/// 注册控制台控制处理器，使进程能在系统关机/注销时争取短暂的收尾时间。
///
/// # Errors
/// `SetConsoleCtrlHandler` 调用失败时返回
/// “Failed to install console control handler”。
pub fn install_console_ctrl_handler() -> anyhow::Result<()> {
    use windows::Win32::System::Console::SetConsoleCtrlHandler;
    unsafe {
        if SetConsoleCtrlHandler(Some(console_ctrl_handler), true).is_err() {
            anyhow::bail!("Failed to install console control handler");
        }
    }
    Ok(())
}

/// 锁定当前工作站，转发到 `crate::power::lock`。
///
/// # Errors
/// 透传 [`crate::power::lock`] 的错误：Session 0 路径下会话代理请求失败，非
/// Session 0 路径下 `LockWorkStation` 失败时返回错误。
pub fn lock() -> anyhow::Result<()> {
    crate::power::lock()
}

/// 开启或关闭保持唤醒，转发到 `crate::power::set_keep_awake`。
///
/// # Errors
/// 透传 [`crate::power::set_keep_awake`] 的错误：`SetThreadExecutionState` 返回 0。
pub fn set_keep_awake(enabled: bool) -> anyhow::Result<()> {
    crate::power::set_keep_awake(enabled)
}
