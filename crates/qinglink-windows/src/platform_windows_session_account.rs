//! 账户与唤醒策略判定：自动登录、无密码账户与唤醒后免密码设置。

use windows::Win32::Foundation::{CloseHandle, HANDLE};

use super::registry::{read_hklm_dword, read_hklm_string, read_winlogon_value};
use super::to_wide_z;

pub(super) fn is_auto_admin_logon_enabled() -> bool {
    read_winlogon_value("AutoAdminLogon").as_deref() == Some("1")
        && read_winlogon_value("DefaultUserName")
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false)
}

/// Returns true when the current interactive user can sign in with an empty
/// password, i.e. the account has no password and its lock screen only asks
/// for a confirmation. This mirrors what the Windows lock screen itself will
/// accept, so "no_password" mode does not depend on the power scheme setting.
pub(super) fn is_account_passwordless() -> bool {
    use windows::Win32::Security::{
        LOGON32_LOGON_INTERACTIVE, LOGON32_PROVIDER_DEFAULT, LogonUserW,
    };

    // The network server runs as LocalSystem (Session 0), so its own
    // environment names SYSTEM rather than the logged-in user. Ask the active
    // console session for the real user/domain instead.
    let Some((username, domain)) = interactive_user_and_domain() else {
        return false;
    };
    let username = username.trim();
    let domain = domain.trim();
    if username.is_empty() || domain.is_empty() {
        return false;
    }

    let user_wide = to_wide_z(username);
    let domain_wide = to_wide_z(domain);
    let mut token = HANDLE::default();
    // Interactive logon with an empty password succeeds exactly when the lock
    // screen of this account can be dismissed by a confirmation action.
    let result = unsafe {
        LogonUserW(
            windows::core::PCWSTR(user_wide.as_ptr()),
            windows::core::PCWSTR(domain_wide.as_ptr()),
            windows::core::PCWSTR::null(),
            LOGON32_LOGON_INTERACTIVE,
            LOGON32_PROVIDER_DEFAULT,
            &mut token,
        )
    };
    if result.is_err() {
        return false;
    }
    let _ = unsafe { CloseHandle(token) };
    true
}

/// Resolve the interactive console session's user name and domain.
fn interactive_user_and_domain() -> Option<(String, String)> {
    use windows::Win32::System::RemoteDesktop::{
        WTSDomainName, WTSGetActiveConsoleSessionId, WTSUserName,
    };

    let session = crate::secure_desktop::active_interactive_session()
        .unwrap_or_else(|| unsafe { WTSGetActiveConsoleSessionId() });
    if session == u32::MAX {
        return None;
    }

    let username = query_session_string(session, WTSUserName)?;
    if username.is_empty() {
        return None;
    }
    let domain = query_session_string(session, WTSDomainName)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| std::env::var("COMPUTERNAME").unwrap_or_default());
    Some((username, domain))
}

/// Query one `WTSQuerySessionInformationW` string value for a session.
fn query_session_string(
    session: u32,
    class: windows::Win32::System::RemoteDesktop::WTS_INFO_CLASS,
) -> Option<String> {
    use windows::Win32::System::RemoteDesktop::{WTSFreeMemory, WTSQuerySessionInformationW};

    let mut buffer = windows::core::PWSTR::null();
    let mut bytes = 0u32;
    unsafe {
        WTSQuerySessionInformationW(None, session, class, &mut buffer, &mut bytes).ok()?;
    }
    let value = if buffer.is_null() || bytes < 2 {
        String::new()
    } else {
        let len = (bytes as usize / 2).saturating_sub(1);
        let slice = unsafe { std::slice::from_raw_parts(buffer.0, len) };
        String::from_utf16_lossy(slice).trim().to_string()
    };
    unsafe {
        WTSFreeMemory(buffer.0.cast());
    }
    Some(value)
}

pub(super) fn is_no_password_wake_enabled() -> bool {
    const SUB_NONE: &str = "fea3413e-7e05-4911-9a71-700331f1c294";
    const CONSOLELOCK: &str = "0e796bdb-100d-47d6-a2d5-f7d2daa51f51";

    let Some(scheme) = read_hklm_string(
        r"SYSTEM\CurrentControlSet\Control\Power\User\PowerSchemes",
        "ActivePowerScheme",
    ) else {
        return false;
    };
    let scheme = scheme.trim().trim_matches(|c| c == '{' || c == '}');
    let setting_path = format!(
        r"SYSTEM\CurrentControlSet\Control\Power\User\PowerSchemes\{}\{}\{}",
        scheme, SUB_NONE, CONSOLELOCK
    );

    read_hklm_dword(&setting_path, "ACSettingIndex") == Some(0)
        && read_hklm_dword(&setting_path, "DCSettingIndex") == Some(0)
}

/// Configure Windows to not require a password on wake from sleep.
///
/// # Errors
/// Returns an error when `powercfg` cannot be spawned, when any of the
/// `SETACVALUEINDEX`/`SETDCVALUEINDEX`/`SETACTIVE` invocations exits with a
/// non-zero status ("powercfg {power} failed with status …" / "powercfg
/// SETACTIVE failed with status …"), or when the setting still reads as enabled
/// afterwards ("Windows did not apply the no-password wake setting").
pub fn configure_no_password_wake() -> anyhow::Result<()> {
    for power in ["SETACVALUEINDEX", "SETDCVALUEINDEX"] {
        let status = std::process::Command::new("powercfg")
            .args([power, "SCHEME_CURRENT", "SUB_NONE", "CONSOLELOCK", "0"])
            .status()?;
        if !status.success() {
            anyhow::bail!("powercfg {power} failed with status {status}");
        }
    }

    let status = std::process::Command::new("powercfg")
        .args(["SETACTIVE", "SCHEME_CURRENT"])
        .status()?;
    if !status.success() {
        anyhow::bail!("powercfg SETACTIVE failed with status {status}");
    }
    if !is_no_password_wake_enabled() {
        anyhow::bail!("Windows did not apply the no-password wake setting");
    }
    Ok(())
}
