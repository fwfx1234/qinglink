//! 系统电源与会话控制：关机、重启、睡眠、注销、锁屏，以及唤醒免密与服务退出清理。

use super::platform_impl;

#[cfg(target_os = "windows")]
use super::windows;

/// 系统电源与会话控制入口，提供关机、重启、休眠、注销与锁屏操作。
#[derive(Debug)]
pub struct SystemController;

impl SystemController {
    /// 关机，`force` 表示强制关闭未响应的程序，`delay_secs` 为延迟秒数。
    ///
    /// # Errors
    ///
    /// 当延迟秒数超出 Windows 可接受上限（`u32`）、启用 `SeShutdownPrivilege`
    /// 权限失败或 `InitiateSystemShutdownExW` 调用失败时返回 `Err`；非 Windows
    /// 平台恒返回不支持错误。
    pub fn shutdown(force: bool, delay_secs: u64) -> anyhow::Result<()> {
        platform_impl::shutdown(force, delay_secs)
    }

    /// 进入睡眠；`hibernate` 为真时改为休眠。
    ///
    /// # Errors
    ///
    /// 当 `SetSuspendState` 调用失败时返回 `Err`；非 Windows 平台恒返回不支持错误。
    pub fn sleep(hibernate: bool) -> anyhow::Result<()> {
        platform_impl::sleep(hibernate)
    }

    /// 重启，`force` 表示强制关闭未响应的程序，`delay_secs` 为延迟秒数。
    ///
    /// # Errors
    ///
    /// 当延迟秒数超出 Windows 可接受上限（`u32`）、启用 `SeShutdownPrivilege`
    /// 权限失败或 `InitiateSystemShutdownExW` 调用失败时返回 `Err`；非 Windows
    /// 平台恒返回不支持错误。
    pub fn restart(force: bool, delay_secs: u64) -> anyhow::Result<()> {
        platform_impl::restart(force, delay_secs)
    }

    /// 注销当前用户会话。
    ///
    /// # Errors
    ///
    /// 当 Session 0 下找不到活动的交互式会话、`WTSLogoffSession` 或
    /// `ExitWindowsEx` 调用失败时返回 `Err`；非 Windows 平台恒返回不支持错误。
    pub fn logoff() -> anyhow::Result<()> {
        platform_impl::logoff()
    }

    /// 锁定当前工作站。
    ///
    /// # Errors
    ///
    /// 当 Session 0 下会话代理不可用或 `LockWorkStation` 调用失败时返回 `Err`；
    /// 非 Windows 平台恒返回不支持错误。
    pub fn lock() -> anyhow::Result<()> {
        platform_impl::lock()
    }
}

/// 注册控制台控制事件处理器，用于在服务被停止时执行清理。
///
/// # Errors
///
/// 当 `SetConsoleCtrlHandler` 注册控制台控制处理器失败时返回 `Err`。
#[cfg(target_os = "windows")]
pub fn install_console_ctrl_handler() -> anyhow::Result<()> {
    windows::install_console_ctrl_handler()
}

/// 非 Windows 平台无需注册控制台控制事件处理器。
///
/// # Errors
///
/// 当前实现恒返回 `Ok`；返回值保留是为了统一各平台的函数签名。
#[cfg(not(target_os = "windows"))]
pub fn install_console_ctrl_handler() -> anyhow::Result<()> {
    Ok(())
}

/// 关闭唤醒时需要输入密码的要求。
///
/// # Errors
///
/// 当 `powercfg` 无法启动、返回非零状态，或设置后仍未生效（唤醒仍需密码）时
/// 返回 `Err`；非 Windows 平台恒返回不支持错误。
pub fn configure_no_password_wake() -> anyhow::Result<()> {
    platform_impl::configure_no_password_wake()
}
