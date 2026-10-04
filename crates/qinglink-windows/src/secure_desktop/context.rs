//! 会话与安全桌面探测。

use anyhow::Result;

/// 当前安全桌面与用户会话的快照信息。
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct SecureDesktopStatus {
    /// 归一化后的桌面类型：`default`、`login`、`winlogon`、`uac` 或 `unknown`。
    pub desktop: String,
    /// 当前是否处于锁定界面。
    pub locked: bool,
    /// 输入注入是否可用。
    pub input_ready: bool,
    /// 是否支持发送安全注意序列（Ctrl+Alt+Del）。
    pub sas_supported: bool,
    /// 目标交互式会话的 ID。
    pub session_id: u32,
    /// 会话代理的状态，例如 `ready` 或 `unavailable`。
    pub agent_state: String,
    /// 是否检测到 UAC 同意提示界面。
    #[serde(default)]
    pub uac_detected: bool,
    /// 当前进程令牌的完整性级别（RID），0 表示未能读取。
    #[serde(default)]
    pub token_integrity_level: u32,
    /// 最近一次探测失败的错误描述，成功时为空串。
    #[serde(default)]
    pub last_error: String,
}

/// Returns whether this process is the LocalSystem service process (Session 0).
pub fn running_in_session_zero() -> bool {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::System::RemoteDesktop::ProcessIdToSessionId;
        use windows::Win32::System::Threading::GetCurrentProcessId;
        let mut session = u32::MAX;
        // The parentheses are required: a block-like expression at the start of a
        // statement would end the statement, splitting `&& session == 0` off.
        (unsafe { ProcessIdToSessionId(GetCurrentProcessId(), &mut session).is_ok() })
            && session == 0
    }
    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}

/// 采集当前安全桌面状态：Session 0 下向会话代理查询，否则本地探测桌面与输入可用性。
///
/// # Errors
/// 仅在非 Session 0 的 Windows 路径上，`attach_input_desktop()`
/// 失败（例如 `OpenInputDesktop`/`SetThreadDesktop` 报错）时返回错误；非 Windows
/// 目标下总是返回“secure desktop input is only supported on Windows”错误。
/// Session 0 下会话代理查询失败、以及输入探测 `SendInput` 失败都不会返回 `Err`，
/// 而是分别归入 `agent_state`/`last_error` 字段。
pub fn status() -> Result<SecureDesktopStatus> {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::System::RemoteDesktop::WTSGetActiveConsoleSessionId;
        let session_id = active_interactive_session()
            .unwrap_or_else(|| unsafe { WTSGetActiveConsoleSessionId() });
        if running_in_session_zero() {
            match super::agent_status_request(session_id) {
                Ok(mut status) => {
                    status.session_id = session_id;
                    return Ok(status);
                }
                Err(error) => {
                    tracing::warn!(session_id, %error, "secure desktop agent status unavailable");
                    return Ok(SecureDesktopStatus {
                        desktop: "unknown".into(),
                        locked: true,
                        input_ready: false,
                        sas_supported: true,
                        session_id,
                        agent_state: "unavailable".into(),
                        uac_detected: false,
                        token_integrity_level: 0,
                        last_error: error.to_string(),
                    });
                }
            }
        }
        let desktop = super::attach_input_desktop()?;
        let login_ui = has_process_in_session("LogonUI.exe", session_id);
        let uac = has_process_in_session("consent.exe", session_id);
        let locked = !desktop.eq_ignore_ascii_case("default") && !uac;
        let desktop_kind = if uac {
            "uac"
        } else if !locked {
            "default"
        } else if desktop.to_ascii_lowercase().contains("winlogon") {
            if login_ui { "login" } else { "winlogon" }
        } else {
            "unknown"
        };
        let integrity = current_token_integrity_level();
        let (input_ready, last_error) = match probe_input() {
            Ok(()) => (true, String::new()),
            Err(error) => (false, error.to_string()),
        };
        Ok(SecureDesktopStatus {
            desktop: desktop_kind.into(),
            locked,
            input_ready,
            sas_supported: true,
            session_id,
            agent_state: "ready".into(),
            uac_detected: uac,
            token_integrity_level: integrity,
            last_error,
        })
    }
    #[cfg(not(target_os = "windows"))]
    {
        anyhow::bail!("secure desktop input is only supported on Windows")
    }
}

#[cfg(target_os = "windows")]
pub fn active_interactive_session() -> Option<u32> {
    use windows::Win32::System::RemoteDesktop::{
        WTS_SESSION_INFOW, WTSActive, WTSEnumerateSessionsW, WTSFreeMemory,
    };
    let mut sessions: *mut WTS_SESSION_INFOW = std::ptr::null_mut();
    let mut count = 0u32;
    if unsafe { WTSEnumerateSessionsW(None, 0, 1, &mut sessions, &mut count) }.is_err()
        || sessions.is_null()
    {
        return None;
    }
    let entries = unsafe { std::slice::from_raw_parts(sessions, count as usize) };
    let mut console = None;
    let mut rdp = None;
    for entry in entries {
        if entry.State != WTSActive {
            continue;
        }
        let name = if entry.pWinStationName.is_null() {
            String::new()
        } else {
            let mut length = 0usize;
            unsafe {
                while length < 256 && *entry.pWinStationName.0.add(length) != 0 {
                    length += 1;
                }
                String::from_utf16_lossy(std::slice::from_raw_parts(
                    entry.pWinStationName.0,
                    length,
                ))
            }
        };
        if name.eq_ignore_ascii_case("Console") {
            console = Some(entry.SessionId);
        } else if name.to_ascii_lowercase().starts_with("rdp-") {
            rdp = Some(entry.SessionId);
        }
    }
    unsafe { WTSFreeMemory(sessions.cast()) };
    console.or(rdp)
}

#[cfg(not(target_os = "windows"))]
pub fn active_interactive_session() -> Option<u32> {
    None
}

#[cfg(target_os = "windows")]
fn has_process_in_session(name: &str, session_id: u32) -> bool {
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
        TH32CS_SNAPPROCESS,
    };
    use windows::Win32::System::RemoteDesktop::ProcessIdToSessionId;
    let Ok(snapshot) = (unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) }) else {
        return false;
    };
    let mut entry = PROCESSENTRY32W {
        dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
        ..Default::default()
    };
    let mut found = false;
    let mut next = unsafe { Process32FirstW(snapshot, &mut entry) }.is_ok();
    while next {
        let end = entry
            .szExeFile
            .iter()
            .position(|unit| *unit == 0)
            .unwrap_or(entry.szExeFile.len());
        if String::from_utf16_lossy(&entry.szExeFile[..end]).eq_ignore_ascii_case(name) {
            let mut process_session = u32::MAX;
            if unsafe { ProcessIdToSessionId(entry.th32ProcessID, &mut process_session) }.is_ok()
                && process_session == session_id
            {
                found = true;
                break;
            }
        }
        next = unsafe { Process32NextW(snapshot, &mut entry) }.is_ok();
    }
    unsafe {
        windows::Win32::Foundation::CloseHandle(snapshot).ok();
    }
    found
}

#[cfg(target_os = "windows")]
fn current_token_integrity_level() -> u32 {
    use windows::Win32::Security::{
        GetSidSubAuthority, GetSidSubAuthorityCount, GetTokenInformation, TOKEN_MANDATORY_LABEL,
        TOKEN_QUERY, TokenIntegrityLevel,
    };
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
    unsafe {
        let process = GetCurrentProcess();
        let mut token = windows::Win32::Foundation::HANDLE::default();
        if OpenProcessToken(process, TOKEN_QUERY, &mut token).is_err() {
            return 0;
        }
        let mut size = 0u32;
        let _ = GetTokenInformation(token, TokenIntegrityLevel, None, 0, &mut size);
        if size == 0 {
            windows::Win32::Foundation::CloseHandle(token).ok();
            return 0;
        }
        let mut buffer = vec![0u8; size as usize];
        if GetTokenInformation(
            token,
            TokenIntegrityLevel,
            Some(buffer.as_mut_ptr().cast()),
            size,
            &mut size,
        )
        .is_err()
        {
            windows::Win32::Foundation::CloseHandle(token).ok();
            return 0;
        }
        windows::Win32::Foundation::CloseHandle(token).ok();
        let label = &*(buffer.as_ptr().cast::<TOKEN_MANDATORY_LABEL>());
        let sid = label.Label.Sid;
        let count_ptr = GetSidSubAuthorityCount(sid);
        if count_ptr.is_null() {
            return 0;
        }
        let count = *count_ptr;
        if count < 1 {
            return 0;
        }
        *(GetSidSubAuthority(sid, (count - 1) as u32) as *const u32)
    }
}

#[cfg(not(target_os = "windows"))]
fn current_token_integrity_level() -> u32 {
    0
}

#[cfg(target_os = "windows")]
fn probe_input() -> Result<()> {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_MOVE, MOUSEINPUT, SendInput,
    };
    let input = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: 0,
                dy: 0,
                mouseData: 0,
                dwFlags: MOUSEEVENTF_MOVE,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    let inserted = unsafe { SendInput(&[input], std::mem::size_of::<INPUT>() as i32) };
    anyhow::ensure!(inserted == 1, "SendInput probe failed");
    Ok(())
}
