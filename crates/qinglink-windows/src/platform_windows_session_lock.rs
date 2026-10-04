//! 会话与锁屏状态查询：锁屏与安全桌面判定、WTS 会话锁定状态。

use qinglink_protocol::UnlockStatus;
use windows::Win32::Foundation::HANDLE;

use super::session_account::{
    is_account_passwordless, is_auto_admin_logon_enabled, is_no_password_wake_enabled,
};

/// 查询当前锁屏状态：是否已锁定、是否检测到 UAC 以及当前锁屏界面类型。
///
/// # Errors
/// 当前实现不返回 `Err`：安全桌面探测、锁屏状态查询、自动登录与无密码唤醒
/// 判定全部是尽力而为的，失败时退化为保守取值并返回 `Ok`。返回值保留 `Result`
/// 以维持接口稳定。
pub fn get_unlock_status() -> anyhow::Result<UnlockStatus> {
    // `locked` describes only what the user currently sees: whether the
    // lock/login surface is up. Input-injection readiness and session-agent
    // availability are deliberately excluded, because a missing agent does not
    // mean the workstation is locked.
    let snapshot = crate::secure_desktop::status().ok();
    let uac_detected = snapshot.as_ref().is_some_and(|status| status.uac_detected);
    let state = query_lock_state();
    let locked = !uac_detected && state.unwrap_or(false);
    let surface = if uac_detected {
        "uac"
    } else if locked {
        "login"
    } else if state.is_none() {
        "unknown"
    } else {
        "default"
    };
    let auto_login_enabled = is_auto_admin_logon_enabled();
    let no_password_wake_enabled = is_no_password_wake_enabled();
    let account_passwordless = is_account_passwordless();
    let no_password_mode = no_password_wake_enabled || account_passwordless;
    let mode = if uac_detected {
        "uac"
    } else if no_password_mode {
        "no_password"
    } else if auto_login_enabled {
        "password"
    } else {
        "manual"
    };
    let message = if uac_detected {
        "Windows 正在等待 UAC 确认，当前账户已登录"
    } else if locked && no_password_mode {
        "PC 已上线，正在尝试确认解锁；如仍锁定请使用控制页输入 PIN 或密码"
    } else if locked {
        "PC 已上线但仍需本机解锁"
    } else if state.is_none() {
        "无法确认锁屏状态，已按可用处理"
    } else if no_password_mode {
        "PC 已解锁，无密码账户或已配置唤醒后不要求登录"
    } else if auto_login_enabled {
        "PC 已解锁；Windows 自动登录仅适用于开机，不用于唤醒解锁"
    } else {
        "PC 已解锁"
    };

    Ok(UnlockStatus {
        locked,
        mode: mode.to_string(),
        auto_login_enabled,
        message: message.to_string(),
        uac_detected,
        surface: surface.to_string(),
    })
}

/// Best-effort lock-screen observation.
///
/// `Some(true)` means the lock/login surface is showing, `Some(false)` means the
/// interactive session is usable, and `None` means Windows did not provide a
/// usable answer. The WTS session flag is authoritative; the session agent's
/// view of the input desktop is only a fallback when WTS is inconclusive.
pub(super) fn query_lock_state() -> Option<bool> {
    if crate::secure_desktop::running_in_session_zero() {
        if let Some(session_id) = active_session_id() {
            match query_wts_session_locked(session_id) {
                Ok(Some(locked)) => return Some(locked),
                Ok(None) => {}
                Err(error) => tracing::debug!(%error, "WTS lock state unavailable"),
            }
        }
        // An unavailable session agent means the state is unknown, never
        // "locked": the agent only exists to inject input.
        if let Ok(status) = crate::secure_desktop::status()
            && status.agent_state != "unavailable"
        {
            return Some(status.locked);
        }
        return None;
    }
    is_workstation_locked().ok()
}

/// The interactive session whose lock screen should be observed. Prefer the
/// console/RDP session reported by WTS enumeration, but fall back to the active
/// console session id during transitions where enumeration reports none.
fn active_session_id() -> Option<u32> {
    use windows::Win32::System::RemoteDesktop::WTSGetActiveConsoleSessionId;
    let session = crate::secure_desktop::active_interactive_session()
        .unwrap_or_else(|| unsafe { WTSGetActiveConsoleSessionId() });
    (session != u32::MAX).then_some(session)
}

/// Query the lock state of an interactive session using the WTS extended
/// session information. `None` means Windows returned an unknown flag.
#[cfg(target_os = "windows")]
fn query_wts_session_locked(session_id: u32) -> anyhow::Result<Option<bool>> {
    use windows::Win32::System::RemoteDesktop::{
        WTSFreeMemory, WTSINFOEXW, WTSQuerySessionInformationW, WTSSessionInfoEx,
    };

    let mut buffer = windows::core::PWSTR::null();
    let mut bytes = 0u32;
    unsafe {
        WTSQuerySessionInformationW(None, session_id, WTSSessionInfoEx, &mut buffer, &mut bytes)?;
    }
    if buffer.is_null() {
        anyhow::bail!("WTS session information returned a null buffer");
    }

    let result = (|| {
        anyhow::ensure!(
            bytes as usize >= std::mem::size_of::<WTSINFOEXW>(),
            "WTS session information response is truncated"
        );
        // WTS allocates a WTSINFOEXW block for this query and the generated
        // bindings expose its address as PWSTR; the buffer is aligned for the
        // native structure returned by the API.
        let info = unsafe { &*(buffer.0 as *const WTSINFOEXW) };
        anyhow::ensure!(info.Level == 1, "unsupported WTS session information level");
        let flags = unsafe { info.Data.WTSInfoExLevel1.SessionFlags } as u32;
        Ok(map_wts_session_flags(flags))
    })();

    unsafe {
        WTSFreeMemory(buffer.0.cast());
    }
    result
}

fn map_wts_session_flags(flags: u32) -> Option<bool> {
    match flags {
        0 => Some(true),
        1 => Some(false),
        _ => None,
    }
}

#[cfg(not(target_os = "windows"))]
fn query_wts_session_locked(_session_id: u32) -> anyhow::Result<Option<bool>> {
    anyhow::bail!("WTS session state is only supported on Windows")
}

fn is_workstation_locked() -> anyhow::Result<bool> {
    if let Ok(status) = crate::secure_desktop::status() {
        if status.uac_detected {
            return Ok(false);
        }
        if status.desktop.eq_ignore_ascii_case("default") {
            return Ok(false);
        }
    }
    use windows::Win32::System::StationsAndDesktops::{
        CloseDesktop, DESKTOP_ACCESS_FLAGS, DESKTOP_READOBJECTS, DESKTOP_SWITCHDESKTOP,
        GetUserObjectInformationW, OpenInputDesktop, UOI_NAME,
    };

    unsafe {
        let desktop = OpenInputDesktop(
            Default::default(),
            false,
            DESKTOP_ACCESS_FLAGS(DESKTOP_READOBJECTS.0 | DESKTOP_SWITCHDESKTOP.0),
        )?;

        let mut needed = 0u32;
        let mut buffer = [0u16; 256];
        let result = GetUserObjectInformationW(
            HANDLE(desktop.0),
            UOI_NAME,
            Some(buffer.as_mut_ptr() as *mut _),
            (buffer.len() * 2) as u32,
            Some(&mut needed),
        );
        let _ = CloseDesktop(desktop);
        result?;

        let len = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
        let desktop_name = String::from_utf16_lossy(&buffer[..len]);
        Ok(!desktop_name.eq_ignore_ascii_case("Default"))
    }
}

/// 判断当前是否处于安全桌面（已锁定、UAC 提示或非 Default 桌面）。
///
/// # Errors
/// 透传 `crate::secure_desktop::status()` 的错误：附加输入桌面
/// （`OpenInputDesktop`/`SetThreadDesktop`）失败时返回错误。
pub fn is_secure_desktop() -> anyhow::Result<bool> {
    let status = crate::secure_desktop::status()?;
    Ok(status.locked || status.uac_detected || !status.desktop.eq_ignore_ascii_case("default"))
}

#[cfg(test)]
mod tests {
    use super::map_wts_session_flags;

    #[test]
    fn wts_lock_flag_maps_to_locked() {
        assert_eq!(map_wts_session_flags(0), Some(true));
    }

    #[test]
    fn wts_unlock_flag_maps_to_unlocked() {
        assert_eq!(map_wts_session_flags(1), Some(false));
    }

    #[test]
    fn unknown_wts_flag_is_conservative() {
        assert_eq!(map_wts_session_flags(u32::MAX), None);
        assert_eq!(map_wts_session_flags(2), None);
    }
}
