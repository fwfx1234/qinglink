#[cfg(target_os = "windows")]
mod imp {
    use anyhow::Context;
    use windows::Win32::System::Power::{
        ES_CONTINUOUS, ES_DISPLAY_REQUIRED, ES_SYSTEM_REQUIRED, EXECUTION_STATE, SetSuspendState,
        SetThreadExecutionState,
    };
    use windows::Win32::System::RemoteDesktop::{WTSGetActiveConsoleSessionId, WTSLogoffSession};
    use windows::Win32::System::Shutdown::{
        EWX_LOGOFF, ExitWindowsEx, InitiateSystemShutdownExW, LockWorkStation, SHUTDOWN_REASON,
    };
    use windows::core::PCWSTR;

    fn initiate_shutdown(force: bool, delay_secs: u64, reboot: bool) -> anyhow::Result<()> {
        let timeout = u32::try_from(delay_secs)
            .map_err(|_| anyhow::anyhow!("shutdown delay exceeds the Windows limit"))?;
        // A Session 0 LocalSystem service holds SeShutdownPrivilege but does not
        // have it enabled by default; without it InitiateSystemShutdownExW fails
        // with ERROR_ACCESS_DENIED ("拒绝访问").
        enable_shutdown_privilege()?;
        // InitiateSystemShutdownExW applies the timeout and force policy at the
        // system level, which also works when the caller is the Session 0
        // service. ExitWindowsEx would otherwise treat a zero shutdown flag as
        // a logoff and silently ignore the requested delay.
        unsafe {
            InitiateSystemShutdownExW(
                PCWSTR::null(),
                PCWSTR::null(),
                timeout,
                force,
                reboot,
                SHUTDOWN_REASON(0),
            )?;
        }
        Ok(())
    }

    /// Enable `SeShutdownPrivilege` on the current process token.
    fn enable_shutdown_privilege() -> anyhow::Result<()> {
        use windows::Win32::Foundation::{CloseHandle, HANDLE, LUID};
        use windows::Win32::Security::{
            AdjustTokenPrivileges, LUID_AND_ATTRIBUTES, LookupPrivilegeValueW,
            SE_PRIVILEGE_ENABLED, SE_SHUTDOWN_NAME, TOKEN_ADJUST_PRIVILEGES, TOKEN_PRIVILEGES,
            TOKEN_QUERY,
        };
        use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

        unsafe {
            let mut token = HANDLE::default();
            OpenProcessToken(
                GetCurrentProcess(),
                TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY,
                &mut token,
            )
            .context("无法打开进程令牌以启用关机权限")?;

            let mut luid = LUID::default();
            let lookup = LookupPrivilegeValueW(PCWSTR::null(), SE_SHUTDOWN_NAME, &mut luid);

            let privileges = TOKEN_PRIVILEGES {
                PrivilegeCount: 1,
                Privileges: [LUID_AND_ATTRIBUTES {
                    Luid: luid,
                    Attributes: SE_PRIVILEGE_ENABLED,
                }],
            };
            let adjust = lookup.and_then(|()| {
                AdjustTokenPrivileges(token, false, Some(&privileges as *const _), 0, None, None)
            });
            let _ = CloseHandle(token);
            adjust.context("无法启用关机权限 (SeShutdownPrivilege)")?;
        }
        Ok(())
    }

    /// 在可选延迟后关闭系统，`force` 表示强制关闭未保存的程序。
    ///
    /// # Errors
    /// `delay_secs` 超出 `u32` 表示范围（“shutdown delay exceeds the Windows
    /// limit”）、无法打开进程令牌或启用 `SeShutdownPrivilege` 失败
    /// （“无法打开进程令牌以启用关机权限”/“无法启用关机权限 (SeShutdownPrivilege)”）、
    /// 或 `InitiateSystemShutdownExW` 调用失败时返回错误。
    pub fn shutdown(force: bool, delay_secs: u64) -> anyhow::Result<()> {
        initiate_shutdown(force, delay_secs, false)
    }

    /// 让系统进入睡眠；`hibernate` 为真时进入休眠而非睡眠。
    ///
    /// # Errors
    /// `SetSuspendState` 返回失败时返回“SetSuspendState failed”。
    pub fn sleep(hibernate: bool) -> anyhow::Result<()> {
        let suspended = unsafe { SetSuspendState(hibernate, false, false) };
        anyhow::ensure!(suspended, "SetSuspendState failed");
        Ok(())
    }

    /// 阻止或允许系统自动休眠与显示器熄屏，即"保持唤醒"。
    ///
    /// 系统与显示器都要请求：只请求 `ES_SYSTEM_REQUIRED` 时系统确实不会睡，但显示器仍
    /// 会在"关闭显示器"超时后熄屏；用户看到黑屏会认为保持唤醒没有生效，而远程控制也
    /// 失去了可见画面。Windows 在本线程退出时自动清除该状态，显式关闭时用
    /// `ES_CONTINUOUS` 立刻释放两项请求。
    ///
    /// # Errors
    /// 返回值为 0（调用失败）时返回 "SetThreadExecutionState failed"。
    pub fn set_keep_awake(enabled: bool) -> anyhow::Result<()> {
        let flags = if enabled {
            ES_CONTINUOUS | ES_SYSTEM_REQUIRED | ES_DISPLAY_REQUIRED
        } else {
            ES_CONTINUOUS
        };
        let previous = unsafe { SetThreadExecutionState(flags) };
        anyhow::ensure!(
            previous != EXECUTION_STATE(0),
            "SetThreadExecutionState failed"
        );
        Ok(())
    }

    /// 在可选延迟后重启系统，`force` 表示强制重启未保存的程序。
    ///
    /// # Errors
    /// 与 [`shutdown`] 相同：`delay_secs` 超出 `u32` 表示范围、无法启用
    /// `SeShutdownPrivilege`，或 `InitiateSystemShutdownExW`（`reboot = true`）
    /// 调用失败时返回错误。
    pub fn restart(force: bool, delay_secs: u64) -> anyhow::Result<()> {
        initiate_shutdown(force, delay_secs, true)
    }

    /// 注销当前交互式用户会话；在 Session 0 服务中改为注销活动控制台会话。
    ///
    /// # Errors
    /// Session 0 路径下取不到活动交互式会话（“no active interactive session”）或
    /// `WTSLogoffSession` 失败时返回错误；非 Session 0 路径下 `ExitWindowsEx`
    /// 失败时返回错误。
    pub fn logoff() -> anyhow::Result<()> {
        if crate::secure_desktop::running_in_session_zero() {
            let session = crate::secure_desktop::active_interactive_session()
                .unwrap_or_else(|| unsafe { WTSGetActiveConsoleSessionId() });
            anyhow::ensure!(session != u32::MAX, "no active interactive session");
            unsafe { WTSLogoffSession(None, session, false)? };
            return Ok(());
        }
        unsafe {
            ExitWindowsEx(EWX_LOGOFF, SHUTDOWN_REASON(0))?;
        }
        Ok(())
    }

    /// 锁定当前工作站；在 Session 0 服务中改为委派给会话代理执行。
    ///
    /// # Errors
    /// Session 0 路径下会话代理请求失败时返回带上下文
    /// `SESSION_AGENT_UNAVAILABLE: lock requires the session agent` 的错误；非
    /// Session 0 路径下 `LockWorkStation` 失败时返回错误。
    pub fn lock() -> anyhow::Result<()> {
        if crate::secure_desktop::running_in_session_zero() {
            return crate::secure_desktop::agent_request("lock", None, None, None, None)
                .context("SESSION_AGENT_UNAVAILABLE: lock requires the session agent");
        }
        unsafe {
            LockWorkStation()?;
        }
        Ok(())
    }
}

#[cfg(target_os = "windows")]
pub use imp::*;

#[cfg(not(target_os = "windows"))]
fn unsupported() -> anyhow::Error {
    anyhow::anyhow!("power control is only supported on Windows")
}

/// 非 Windows 平台的占位实现，始终返回不支持错误。
#[cfg(not(target_os = "windows"))]
pub fn shutdown(_force: bool, _delay_secs: u64) -> anyhow::Result<()> {
    Err(unsupported())
}
/// 非 Windows 平台的占位实现，始终返回不支持错误。
#[cfg(not(target_os = "windows"))]
pub fn sleep(_hibernate: bool) -> anyhow::Result<()> {
    Err(unsupported())
}
/// 非 Windows 平台的占位实现，始终返回不支持错误。
#[cfg(not(target_os = "windows"))]
pub fn set_keep_awake(_enabled: bool) -> anyhow::Result<()> {
    Err(unsupported())
}
/// 非 Windows 平台的占位实现，始终返回不支持错误。
#[cfg(not(target_os = "windows"))]
pub fn restart(_force: bool, _delay_secs: u64) -> anyhow::Result<()> {
    Err(unsupported())
}
/// 非 Windows 平台的占位实现，始终返回不支持错误。
#[cfg(not(target_os = "windows"))]
pub fn logoff() -> anyhow::Result<()> {
    Err(unsupported())
}
/// 非 Windows 平台的占位实现，始终返回不支持错误。
#[cfg(not(target_os = "windows"))]
pub fn lock() -> anyhow::Result<()> {
    Err(unsupported())
}
