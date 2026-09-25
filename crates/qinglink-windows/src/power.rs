#[cfg(target_os = "windows")]
mod imp {
    use anyhow::Context;
    use windows::Win32::System::Power::SetSuspendState;
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

    pub fn shutdown(force: bool, delay_secs: u64) -> anyhow::Result<()> {
        initiate_shutdown(force, delay_secs, false)
    }

    pub fn sleep(hibernate: bool) -> anyhow::Result<()> {
        let suspended = unsafe { SetSuspendState(hibernate, false, false) };
        anyhow::ensure!(suspended, "SetSuspendState failed");
        Ok(())
    }

    pub fn restart(force: bool, delay_secs: u64) -> anyhow::Result<()> {
        initiate_shutdown(force, delay_secs, true)
    }

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

#[cfg(not(target_os = "windows"))]
pub fn shutdown(_force: bool, _delay_secs: u64) -> anyhow::Result<()> {
    Err(unsupported())
}
#[cfg(not(target_os = "windows"))]
pub fn sleep(_hibernate: bool) -> anyhow::Result<()> {
    Err(unsupported())
}
#[cfg(not(target_os = "windows"))]
pub fn restart(_force: bool, _delay_secs: u64) -> anyhow::Result<()> {
    Err(unsupported())
}
#[cfg(not(target_os = "windows"))]
pub fn logoff() -> anyhow::Result<()> {
    Err(unsupported())
}
#[cfg(not(target_os = "windows"))]
pub fn lock() -> anyhow::Result<()> {
    Err(unsupported())
}
