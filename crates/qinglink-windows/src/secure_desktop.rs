//! Windows secure-desktop input support.
//!
//! The router deliberately lives on a dedicated call thread.  Windows binds a
//! thread to a desktop, so the handle is refreshed for every operation instead
//! of being cached across Winlogon/UAC transitions.

use anyhow::{Context, Result, ensure};
use std::io::{Read, Write};

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct SecureDesktopStatus {
    pub desktop: String,
    pub locked: bool,
    pub input_ready: bool,
    pub sas_supported: bool,
    pub session_id: u32,
    pub agent_state: String,
    #[serde(default)]
    pub uac_detected: bool,
    #[serde(default)]
    pub token_integrity_level: u32,
    #[serde(default)]
    pub last_error: String,
}

/// Returns true for the LocalSystem service process (Session 0). Services
/// cannot inject into the interactive window station directly and must use
/// the per-session agent pipe even when Session 0 reports `Default`.
pub fn running_in_session_zero() -> bool {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::System::RemoteDesktop::ProcessIdToSessionId;
        use windows::Win32::System::Threading::GetCurrentProcessId;
        let mut session = u32::MAX;
        return unsafe { ProcessIdToSessionId(GetCurrentProcessId(), &mut session).is_ok() }
            && session == 0;
    }
    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}

#[cfg(target_os = "windows")]
fn desktop_name(desktop: windows::Win32::System::StationsAndDesktops::HDESK) -> Result<String> {
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::StationsAndDesktops::{GetUserObjectInformationW, UOI_NAME};
    let mut buffer = [0u16; 128];
    let mut needed = 0u32;
    unsafe {
        GetUserObjectInformationW(
            HANDLE(desktop.0),
            UOI_NAME,
            Some(buffer.as_mut_ptr().cast()),
            (buffer.len() * 2) as u32,
            Some(&mut needed),
        )?;
    }
    let end = buffer
        .iter()
        .position(|unit| *unit == 0)
        .unwrap_or(buffer.len());
    Ok(String::from_utf16_lossy(&buffer[..end]))
}

#[cfg(target_os = "windows")]
fn attach_input_desktop() -> Result<String> {
    use windows::Win32::Foundation::GENERIC_WRITE;
    use windows::Win32::System::StationsAndDesktops::{
        CloseDesktop, DESKTOP_ACCESS_FLAGS, DESKTOP_CREATEMENU, DESKTOP_CREATEWINDOW,
        DESKTOP_ENUMERATE, DESKTOP_HOOKCONTROL, DESKTOP_READOBJECTS, DESKTOP_SWITCHDESKTOP,
        DESKTOP_WRITEOBJECTS, GetThreadDesktop, OpenInputDesktop, SetThreadDesktop,
    };
    use windows::Win32::System::Threading::GetCurrentThreadId;
    let access = DESKTOP_ACCESS_FLAGS(
        DESKTOP_CREATEWINDOW.0
            | DESKTOP_CREATEMENU.0
            | DESKTOP_HOOKCONTROL.0
            | DESKTOP_ENUMERATE.0
            | DESKTOP_READOBJECTS.0
            | DESKTOP_SWITCHDESKTOP.0
            | DESKTOP_WRITEOBJECTS.0
            // GENERIC_WRITE is required by Winlogon/UAC desktops on some
            // Windows builds even when the specific desktop rights are set.
            | GENERIC_WRITE.0,
    );
    unsafe {
        let input = OpenInputDesktop(Default::default(), false, access)
            .context("OpenInputDesktop failed")?;
        let name = desktop_name(input)?;
        let current = GetThreadDesktop(GetCurrentThreadId())?;
        let current_name = desktop_name(current).unwrap_or_default();
        if current_name.eq_ignore_ascii_case(&name) {
            tracing::debug!(desktop = %name, "already attached to input desktop");
            CloseDesktop(input).ok();
        } else {
            tracing::info!(from = %current_name, to = %name, "switching thread desktop for secure input");
            if let Err(error) = SetThreadDesktop(input) {
                CloseDesktop(input).ok();
                return Err(error).context("SetThreadDesktop failed");
            }
        }
        Ok(name)
    }
}

#[cfg(not(target_os = "windows"))]
fn attach_input_desktop() -> Result<String> {
    anyhow::bail!("secure desktop input is only supported on Windows")
}

pub fn status() -> Result<SecureDesktopStatus> {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::System::RemoteDesktop::WTSGetActiveConsoleSessionId;
        let session_id = active_interactive_session()
            .unwrap_or_else(|| unsafe { WTSGetActiveConsoleSessionId() });
        if running_in_session_zero() {
            match agent_status_request(session_id) {
                Ok(mut status) => {
                    status.session_id = session_id;
                    return Ok(status);
                }
                Err(error) => {
                    // Session 0 cannot observe the user's input desktop. Report an
                    // unavailable/conservative state instead of claiming unlocked.
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
        let desktop = attach_input_desktop()?;
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
        tracing::info!(
            session_id,
            desktop = %desktop_kind,
            locked,
            uac_detected = uac,
            token_integrity_level = integrity,
            "secure desktop status"
        );
        if uac {
            tracing::warn!(session_id, "UAC prompt detected (consent.exe running)");
        }
        let (input_ready, last_error) = match probe_input() {
            Ok(()) => (true, String::new()),
            Err(error) => (false, error.to_string()),
        };
        return Ok(SecureDesktopStatus {
            desktop: desktop_kind.into(),
            locked,
            input_ready,
            sas_supported: true,
            session_id,
            agent_state: "ready".into(),
            uac_detected: uac,
            token_integrity_level: integrity,
            last_error,
        });
    }
    #[cfg(not(target_os = "windows"))]
    {
        anyhow::bail!("secure desktop input is only supported on Windows")
    }
}

#[cfg(target_os = "windows")]
fn agent_status_request(session_id: u32) -> Result<SecureDesktopStatus> {
    use std::io::{Read, Write};
    let pipe = format!(r"\\.\pipe\QingLinkSessionAgent-{session_id}");
    let mut stream = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(pipe)
        .context("session agent pipe unavailable")?;
    #[derive(serde::Serialize)]
    struct Request<'a> {
        version: u32,
        command: &'a str,
    }
    #[derive(serde::Deserialize)]
    struct Response {
        ok: bool,
        #[serde(default)]
        message: String,
        #[serde(default)]
        desktop: Option<String>,
        #[serde(default)]
        locked: Option<bool>,
        #[serde(default)]
        input_ready: Option<bool>,
        #[serde(default)]
        sas_supported: Option<bool>,
        #[serde(default)]
        session_id: Option<u32>,
        #[serde(default)]
        agent_state: Option<String>,
        #[serde(default)]
        uac_detected: Option<bool>,
        #[serde(default)]
        token_integrity_level: Option<u32>,
        #[serde(default)]
        last_error: Option<String>,
    }
    let payload = serde_json::to_vec(&Request {
        version: 1,
        command: "status",
    })?;
    stream.write_all(&(payload.len() as u32).to_le_bytes())?;
    stream.write_all(&payload)?;
    stream.flush()?;
    let mut len = [0u8; 4];
    stream.read_exact(&mut len)?;
    let size = u32::from_le_bytes(len) as usize;
    ensure!(size > 0 && size <= 64 * 1024, "status response too large");
    let mut response = vec![0u8; size];
    stream.read_exact(&mut response)?;
    let response: Response = serde_json::from_slice(&response)?;
    ensure!(
        response.ok,
        "{}",
        if response.message.is_empty() {
            "agent status failed"
        } else {
            &response.message
        }
    );
    Ok(SecureDesktopStatus {
        desktop: response.desktop.unwrap_or_else(|| "unknown".into()),
        locked: response.locked.unwrap_or(true),
        input_ready: response.input_ready.unwrap_or(false),
        sas_supported: response.sas_supported.unwrap_or(true),
        session_id: response.session_id.unwrap_or(session_id),
        agent_state: response.agent_state.unwrap_or_else(|| "ready".into()),
        uac_detected: response.uac_detected.unwrap_or(false),
        token_integrity_level: response.token_integrity_level.unwrap_or(0),
        last_error: response.last_error.unwrap_or_default(),
    })
}

#[cfg(target_os = "windows")]
pub(crate) fn active_interactive_session() -> Option<u32> {
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
                while *entry.pWinStationName.0.add(length) != 0 && length < 256 {
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
    let mut entry = PROCESSENTRY32W::default();
    entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
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
        let authority = *(GetSidSubAuthority(sid, (count - 1) as u32) as *const u32);
        authority as u32
    }
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
    ensure!(inserted == 1, "SendInput probe failed");
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn current_token_integrity_level() -> u32 {
    0
}

pub fn send_secure_ascii(text: &str) -> Result<()> {
    validate_secure_ascii(text)?;
    tracing::info!(length = text.len(), "sending secure ASCII input");
    if std::env::var_os("QINGLINK_SESSION_AGENT").is_none() {
        return agent_request("secure_ascii", Some(text), None, None, None)
            .context("SESSION_AGENT_UNAVAILABLE: secure ASCII requires the session agent");
    }
    tracing::debug!("secure ASCII being injected by the session agent");
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            GetKeyboardLayout, INPUT, SendInput, VkKeyScanExW,
        };
        let _ = attach_input_desktop()?;
        let layout = unsafe { GetKeyboardLayout(0) };
        let mut inputs = Vec::with_capacity(text.len() * 8);
        for byte in text.bytes() {
            let mapped = unsafe { VkKeyScanExW(byte as u16, layout) };
            ensure!(
                mapped != -1,
                "character is unavailable in the active keyboard layout"
            );
            let vk = (mapped as u16 & 0xff) as u16;
            let modifiers = (mapped as u16 >> 8) as u8;
            for (bit, modifier) in [(1u8, 0x10u16), (2, 0x11), (4, 0x12)] {
                if modifiers & bit != 0 {
                    inputs.push(key_input(modifier, false));
                }
            }
            inputs.push(key_input(vk, false));
            inputs.push(key_input(vk, true));
            for (bit, modifier) in [(4u8, 0x12u16), (2, 0x11), (1, 0x10)] {
                if modifiers & bit != 0 {
                    inputs.push(key_input(modifier, true));
                }
            }
        }
        let inserted = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
        if inserted != inputs.len() as u32 {
            tracing::error!(
                inserted,
                expected = inputs.len(),
                "secure ASCII SendInput failed"
            );
        }
        ensure!(inserted == inputs.len() as u32, "secure ASCII input failed");
        return Ok(());

        fn key_input(vk: u16, key_up: bool) -> INPUT {
            use windows::Win32::UI::Input::KeyboardAndMouse::{
                INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VIRTUAL_KEY,
            };
            INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VIRTUAL_KEY(vk),
                        wScan: 0,
                        dwFlags: if key_up {
                            KEYEVENTF_KEYUP
                        } else {
                            Default::default()
                        },
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        anyhow::bail!("secure desktop input is only supported on Windows")
    }
}

/// Sends Unicode text from the interactive session agent. The service cannot
/// inject into the user's desktop directly because it runs in session 0.
pub fn send_text(text: &str) -> Result<()> {
    ensure!(!text.is_empty() && text.len() <= 64 * 1024, "文字长度无效");
    if std::env::var_os("QINGLINK_SESSION_AGENT").is_none() {
        return agent_request("send_text", Some(text), None, None, None)
            .context("SESSION_AGENT_UNAVAILABLE: text input requires the session agent");
    }
    let _ = attach_input_desktop()?;
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE,
            SendInput, VIRTUAL_KEY,
        };
        let mut inputs = Vec::with_capacity(text.encode_utf16().count() * 2);
        for unit in text.encode_utf16() {
            inputs.push(INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VIRTUAL_KEY(0),
                        wScan: unit,
                        dwFlags: KEYEVENTF_UNICODE,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            });
            inputs.push(INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VIRTUAL_KEY(0),
                        wScan: unit,
                        dwFlags: KEYEVENTF_UNICODE | KEYEVENTF_KEYUP,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            });
        }
        let inserted = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
        ensure!(inserted == inputs.len() as u32, "SendInput 发送文字失败");
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        anyhow::bail!("secure desktop input is only supported on Windows")
    }
}

/// Enumerate windows in the interactive session. The service must ask its
/// session agent because EnumWindows is scoped to the calling window station.
#[cfg(target_os = "windows")]
pub fn enum_windows_light() -> Result<Vec<crate::platform_windows::WindowInfo>> {
    if std::env::var_os("QINGLINK_SESSION_AGENT").is_some() {
        return Ok(crate::platform_windows::enum_windows_light());
    }
    let message = agent_request_message("list_windows", None)?;
    Ok(serde_json::from_str(&message).context("invalid window list from session agent")?)
}

/// Activates a window in the interactive session.  A LocalSystem service
/// cannot call SetForegroundWindow on the user's window station directly, so
/// it forwards the operation to the session agent.
#[cfg(target_os = "windows")]
pub fn focus_window(hwnd: usize) -> Result<()> {
    if std::env::var_os("QINGLINK_SESSION_AGENT").is_some() {
        return crate::platform_windows::focus_window(hwnd);
    }
    agent_window_request("focus_window", Some(hwnd), None, None)
}

/// Activates the first visible window created by a process in the interactive session.
#[cfg(target_os = "windows")]
pub fn focus_process(pid: u32) -> Result<()> {
    if std::env::var_os("QINGLINK_SESSION_AGENT").is_some() {
        return crate::platform_windows::focus_process(pid);
    }
    agent_window_request("focus_process", None, Some(pid), None)
}

/// Applies a window command in the interactive session.
#[cfg(target_os = "windows")]
pub fn window_action(command: &str, hwnd: usize) -> Result<()> {
    if std::env::var_os("QINGLINK_SESSION_AGENT").is_some() {
        return match command {
            "minimize_window" => crate::platform_windows::minimize_window(hwnd),
            "maximize_window" => crate::platform_windows::maximize_window(hwnd),
            "restore_window" => crate::platform_windows::restore_window(hwnd),
            "close_window" => crate::platform_windows::close_window(hwnd),
            _ => anyhow::bail!("unsupported window command"),
        };
    }
    agent_window_request(command, Some(hwnd), None, None)
}

/// Switches the foreground window in the interactive session.
#[cfg(target_os = "windows")]
pub fn switch_window(direction: qinglink_protocol::WindowSwitchDirection) -> Result<()> {
    if std::env::var_os("QINGLINK_SESSION_AGENT").is_some() {
        return crate::platform_windows::switch_window(direction);
    }
    let direction = match direction {
        qinglink_protocol::WindowSwitchDirection::Next => "next",
        qinglink_protocol::WindowSwitchDirection::Previous => "previous",
    };
    agent_window_request("switch_window", None, None, Some(direction))
}

#[cfg(target_os = "windows")]
fn agent_window_request(
    command: &str,
    hwnd: Option<usize>,
    pid: Option<u32>,
    direction: Option<&str>,
) -> Result<()> {
    use windows::Win32::System::RemoteDesktop::WTSGetActiveConsoleSessionId;

    let session =
        active_interactive_session().unwrap_or_else(|| unsafe { WTSGetActiveConsoleSessionId() });
    if session == u32::MAX {
        anyhow::bail!("no active console session");
    }
    let pipe = format!(r"\\.\pipe\QingLinkSessionAgent-{session}");
    let command = command.to_string();
    let direction = direction.map(str::to_owned);
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let result = (|| -> Result<()> {
            let mut stream = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(pipe)?;
            #[derive(serde::Serialize)]
            struct Request<'a> {
                version: u32,
                command: &'a str,
                hwnd: Option<usize>,
                pid: Option<u32>,
                direction: Option<&'a str>,
            }
            #[derive(serde::Deserialize)]
            struct Response {
                ok: bool,
                message: String,
            }
            let payload = serde_json::to_vec(&Request {
                version: 1,
                command: &command,
                hwnd,
                pid,
                direction: direction.as_deref(),
            })?;
            stream.write_all(&(payload.len() as u32).to_le_bytes())?;
            stream.write_all(&payload)?;
            stream.flush()?;
            let mut len = [0u8; 4];
            stream.read_exact(&mut len)?;
            let size = u32::from_le_bytes(len) as usize;
            anyhow::ensure!(size > 0 && size <= 64 * 1024, "agent response too large");
            let mut response = vec![0u8; size];
            stream.read_exact(&mut response)?;
            let response: Response = serde_json::from_slice(&response)?;
            anyhow::ensure!(response.ok, "{}", response.message);
            Ok(())
        })();
        let _ = tx.send(result);
    });
    match rx.recv_timeout(std::time::Duration::from_secs(3)) {
        Ok(result) => result,
        Err(_) => anyhow::bail!("AGENT_REQUEST_TIMEOUT: 窗口操作服务响应超时"),
    }
}

#[cfg(not(target_os = "windows"))]
pub fn enum_windows_light() -> Result<Vec<()>> {
    anyhow::bail!("window enumeration is only supported on Windows")
}

pub fn send_secure_attention() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        use windows::core::BOOL;
        #[link(name = "sas")]
        unsafe extern "system" {
            fn SendSAS(as_user: BOOL);
        }
        tracing::info!("sending secure attention sequence (Ctrl+Alt+Del)");
        let previous = software_sas_generation().ok();
        if previous != Some(1) {
            let _ = set_software_sas_generation(1);
        }
        unsafe {
            SendSAS(false.into());
        }
        if let Some(value) = previous {
            let _ = set_software_sas_generation(value);
        }
        tracing::info!("secure attention sequence sent successfully");
        return Ok(());

        fn software_sas_generation() -> Result<u32> {
            use windows::Win32::System::Registry::{
                HKEY_LOCAL_MACHINE, KEY_READ, REG_VALUE_TYPE, RRF_RT_REG_DWORD, RegCloseKey,
                RegGetValueW, RegOpenKeyExW,
            };
            let subkey: Vec<u16> = "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Policies\\System"
                .encode_utf16()
                .chain([0])
                .collect();
            let value: Vec<u16> = "SoftwareSASGeneration".encode_utf16().chain([0]).collect();
            let mut key = windows::Win32::System::Registry::HKEY(std::ptr::null_mut());
            unsafe {
                RegOpenKeyExW(
                    HKEY_LOCAL_MACHINE,
                    windows::core::PCWSTR(subkey.as_ptr()),
                    None,
                    KEY_READ,
                    &mut key,
                )
                .ok()?;
            }
            let mut data = 0u32;
            let mut size = 4u32;
            let result = unsafe {
                RegGetValueW(
                    key,
                    windows::core::PCWSTR::null(),
                    windows::core::PCWSTR(value.as_ptr()),
                    RRF_RT_REG_DWORD,
                    Some(std::ptr::null_mut::<REG_VALUE_TYPE>()),
                    Some((&mut data as *mut u32).cast()),
                    Some(&mut size),
                )
            };
            unsafe {
                let _ = RegCloseKey(key);
            }
            result.ok()?;
            Ok(data)
        }

        fn set_software_sas_generation(value: u32) -> Result<()> {
            use windows::Win32::System::Registry::{
                HKEY_LOCAL_MACHINE, KEY_SET_VALUE, REG_DWORD, REG_OPTION_NON_VOLATILE, RegCloseKey,
                RegCreateKeyExW, RegSetValueExW,
            };
            let subkey: Vec<u16> = "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Policies\\System"
                .encode_utf16()
                .chain([0])
                .collect();
            let name: Vec<u16> = "SoftwareSASGeneration".encode_utf16().chain([0]).collect();
            let mut key = windows::Win32::System::Registry::HKEY(std::ptr::null_mut());
            unsafe {
                RegCreateKeyExW(
                    HKEY_LOCAL_MACHINE,
                    windows::core::PCWSTR(subkey.as_ptr()),
                    Some(0),
                    None,
                    REG_OPTION_NON_VOLATILE,
                    KEY_SET_VALUE,
                    None,
                    &mut key,
                    None,
                )
                .ok()?;
            }
            let result = unsafe {
                RegSetValueExW(
                    key,
                    windows::core::PCWSTR(name.as_ptr()),
                    None,
                    REG_DWORD,
                    Some(&value.to_ne_bytes()),
                )
            };
            unsafe {
                let _ = RegCloseKey(key);
            }
            result.ok()?;
            Ok(())
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        anyhow::bail!("secure attention is only supported on Windows")
    }
}

fn validate_secure_ascii(text: &str) -> Result<()> {
    ensure!(
        !text.is_empty() && text.len() <= 256,
        "secure text length is invalid"
    );
    ensure!(
        text.bytes().all(|byte| (0x20..=0x7e).contains(&byte)),
        "secure text must be printable ASCII"
    );
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn agent_request(
    _: &str,
    _: Option<&str>,
    _: Option<(i32, i32)>,
    _: Option<&str>,
    _: Option<bool>,
) -> Result<()> {
    anyhow::bail!("secure desktop agent is only supported on Windows")
}
#[cfg(not(target_os = "windows"))]
pub fn send_hid_key(_usage: u8, _modifiers: u8, _pressed: bool) -> Result<()> {
    anyhow::bail!("secure desktop input is only supported on Windows")
}
#[cfg(target_os = "windows")]
pub fn send_mouse_move(dx: i32, dy: i32) -> Result<()> {
    if std::env::var_os("QINGLINK_SESSION_AGENT").is_none() {
        return agent_request("mouse_move", None, Some((dx, dy)), None, None)
            .context("SESSION_AGENT_UNAVAILABLE: mouse input requires the session agent");
    }
    let _ = attach_input_desktop()?;
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_MOVE, MOUSEINPUT, SendInput,
    };
    let input = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx,
                dy,
                mouseData: 0,
                dwFlags: MOUSEEVENTF_MOVE,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    let inserted = unsafe { SendInput(&[input], std::mem::size_of::<INPUT>() as i32) };
    if inserted != 1 {
        tracing::error!(
            dx,
            dy,
            inserted,
            "SendInput mouse move failed (secure desktop direct injection)"
        );
    }
    ensure!(inserted == 1, "SendInput mouse move failed");
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn send_mouse_move(_dx: i32, _dy: i32) -> Result<()> {
    anyhow::bail!("secure desktop input is only supported on Windows")
}

#[cfg(target_os = "windows")]
pub fn send_mouse_move_absolute(x: u32, y: u32) -> Result<()> {
    use windows::Win32::UI::WindowsAndMessaging::{
        GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN,
        SM_YVIRTUALSCREEN,
    };
    let left = unsafe { GetSystemMetrics(SM_XVIRTUALSCREEN) };
    let top = unsafe { GetSystemMetrics(SM_YVIRTUALSCREEN) };
    let width = unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) };
    let height = unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) };
    ensure!(
        width > 0 && height > 0,
        "cannot read virtual-screen metrics"
    );
    let abs_x =
        ((((x as i64 - left as i64) * 65535) / (width as i64 - 1).max(1)).clamp(0, 65535)) as i32;
    let abs_y =
        ((((y as i64 - top as i64) * 65535) / (height as i64 - 1).max(1)).clamp(0, 65535)) as i32;
    if std::env::var_os("QINGLINK_SESSION_AGENT").is_none() {
        return agent_request(
            "mouse_move_absolute",
            None,
            Some((abs_x, abs_y)),
            None,
            None,
        )
        .context("SESSION_AGENT_UNAVAILABLE: absolute mouse input requires the session agent");
    }
    let _ = attach_input_desktop()?;
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_MOVE,
        MOUSEEVENTF_VIRTUALDESK, MOUSEINPUT, SendInput,
    };
    let input = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: abs_x,
                dy: abs_y,
                mouseData: 0,
                dwFlags: MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_MOVE | MOUSEEVENTF_VIRTUALDESK,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    let inserted = unsafe { SendInput(&[input], std::mem::size_of::<INPUT>() as i32) };
    if inserted != 1 {
        tracing::error!(
            x,
            y,
            abs_x,
            abs_y,
            inserted,
            "SendInput absolute mouse move failed"
        );
    }
    ensure!(inserted == 1, "SendInput absolute mouse move failed");
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn send_mouse_move_absolute(_x: u32, _y: u32) -> Result<()> {
    anyhow::bail!("secure desktop input is only supported on Windows")
}

#[cfg(target_os = "windows")]
pub fn send_mouse_button(button: &str, pressed: bool) -> Result<()> {
    if std::env::var_os("QINGLINK_SESSION_AGENT").is_none() {
        return agent_request(
            "mouse_button_state",
            None,
            None,
            Some(button),
            Some(pressed),
        )
        .context("SESSION_AGENT_UNAVAILABLE: mouse input requires the session agent");
    }
    let _ = attach_input_desktop()?;
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP,
        MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEINPUT, SendInput,
    };
    let flag = match (button, pressed) {
        ("left", true) => MOUSEEVENTF_LEFTDOWN,
        ("left", false) => MOUSEEVENTF_LEFTUP,
        ("right", true) => MOUSEEVENTF_RIGHTDOWN,
        ("right", false) => MOUSEEVENTF_RIGHTUP,
        _ => anyhow::bail!("unsupported mouse button"),
    };
    let input = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: 0,
                dy: 0,
                mouseData: 0,
                dwFlags: flag,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    let inserted = unsafe { SendInput(&[input], std::mem::size_of::<INPUT>() as i32) };
    if inserted != 1 {
        tracing::error!(button, pressed, inserted, "SendInput mouse button failed");
    }
    ensure!(inserted == 1, "SendInput mouse button failed");
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn send_mouse_button(_button: &str, _pressed: bool) -> Result<()> {
    anyhow::bail!("secure desktop input is only supported on Windows")
}

#[cfg(target_os = "windows")]
pub fn send_mouse_wheel(delta: i32) -> Result<()> {
    if std::env::var_os("QINGLINK_SESSION_AGENT").is_none() {
        return agent_request("mouse_wheel", None, Some((delta, 0)), None, None)
            .context("SESSION_AGENT_UNAVAILABLE: mouse input requires the session agent");
    }
    let _ = attach_input_desktop()?;
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_WHEEL, MOUSEINPUT, SendInput,
    };
    let input = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: 0,
                dy: 0,
                mouseData: (delta.clamp(-20, 20) * 120) as u32,
                dwFlags: MOUSEEVENTF_WHEEL,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    let inserted = unsafe { SendInput(&[input], std::mem::size_of::<INPUT>() as i32) };
    if inserted != 1 {
        tracing::error!(delta, inserted, "SendInput mouse wheel failed");
    }
    ensure!(inserted == 1, "SendInput mouse wheel failed");
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn send_mouse_wheel(_delta: i32) -> Result<()> {
    anyhow::bail!("secure desktop input is only supported on Windows")
}

#[cfg(target_os = "windows")]
pub fn send_hid_key(usage: u8, modifiers: u8, pressed: bool) -> Result<()> {
    if std::env::var_os("QINGLINK_SESSION_AGENT").is_some() {
        use windows::Win32::UI::Input::KeyboardAndMouse::{INPUT, SendInput};
        let _ = attach_input_desktop()?;
        let (scan, extended) =
            usage_to_scancode(usage).ok_or_else(|| anyhow::anyhow!("unsupported HID usage"))?;
        let modifiers_table = [
            (0x01, 0x1d, false),
            (0x02, 0x2a, false),
            (0x04, 0x38, false),
            (0x08, 0x5b, true),
            (0x10, 0x1d, true),
            (0x20, 0x36, false),
            (0x40, 0x38, true),
            (0x80, 0x5c, true),
        ];
        let mut inputs = Vec::with_capacity(9);
        if pressed {
            for (bit, modifier_scan, modifier_extended) in modifiers_table {
                if modifiers & bit != 0 {
                    inputs.push(scan_input(modifier_scan, true, modifier_extended));
                }
            }
            inputs.push(scan_input(scan, true, extended));
        } else {
            inputs.push(scan_input(scan, false, extended));
            for (bit, modifier_scan, modifier_extended) in modifiers_table.into_iter().rev() {
                if modifiers & bit != 0 {
                    inputs.push(scan_input(modifier_scan, false, modifier_extended));
                }
            }
        }
        let inserted = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
        ensure!(
            inserted == inputs.len() as u32,
            "SendInput HID key event failed"
        );
        return Ok(());
    }
    tracing::debug!(
        usage,
        modifiers,
        pressed,
        "sending HID key event via agent pipe"
    );
    let (tx, rx) = std::sync::mpsc::channel();
    let cmd = "key_event".to_string();
    std::thread::spawn(move || {
        let result = (|| -> Result<()> {
            use windows::Win32::System::RemoteDesktop::WTSGetActiveConsoleSessionId;
            let session = active_interactive_session()
                .unwrap_or_else(|| unsafe { WTSGetActiveConsoleSessionId() });
            let pipe = format!(r"\\.\pipe\QingLinkSessionAgent-{session}");
            let mut stream = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(pipe)?;
            #[derive(serde::Serialize)]
            struct Request {
                version: u32,
                command: String,
                usage: u8,
                modifiers: u8,
                pressed: bool,
            }
            #[derive(serde::Deserialize)]
            struct Response {
                ok: bool,
                message: String,
            }
            let payload = serde_json::to_vec(&Request {
                version: 1,
                command: cmd,
                usage,
                modifiers,
                pressed,
            })?;
            stream.write_all(&(payload.len() as u32).to_le_bytes())?;
            stream.write_all(&payload)?;
            stream.flush()?;
            let mut len = [0u8; 4];
            stream.read_exact(&mut len)?;
            let size = u32::from_le_bytes(len) as usize;
            if size > 64 * 1024 {
                anyhow::bail!("response too large");
            }
            let mut response = vec![0u8; size];
            stream.read_exact(&mut response)?;
            let response: Response = serde_json::from_slice(&response)?;
            ensure!(response.ok, "{}", response.message);
            Ok(())
        })();
        let _ = tx.send(result);
    });
    match rx.recv_timeout(std::time::Duration::from_secs(3)) {
        Ok(result) => result,
        Err(_) => anyhow::bail!("AGENT_REQUEST_TIMEOUT: 键盘输入服务响应超时"),
    }
}
#[cfg(target_os = "windows")]
pub(crate) fn agent_request(
    command: &str,
    text: Option<&str>,
    delta: Option<(i32, i32)>,
    button: Option<&str>,
    pressed: Option<bool>,
) -> Result<()> {
    use windows::Win32::System::RemoteDesktop::WTSGetActiveConsoleSessionId;
    let session =
        active_interactive_session().unwrap_or_else(|| unsafe { WTSGetActiveConsoleSessionId() });
    if session == u32::MAX {
        anyhow::bail!("no active console session")
    }
    tracing::debug!(
        session,
        command,
        "agent_request: sending command to session agent"
    );
    let pipe = format!(r"\\.\pipe\QingLinkSessionAgent-{session}");
    let (tx, rx) = std::sync::mpsc::channel();
    let cmd = command.to_string();
    let txt = text.map(|s| s.to_string());
    let btn = button.map(|s| s.to_string());
    std::thread::spawn(move || {
        let result = (|| -> Result<()> {
            let mut stream = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(pipe)?;
            #[derive(serde::Serialize)]
            struct Request<'a> {
                version: u32,
                command: &'a str,
                text: Option<&'a str>,
                dx: Option<i32>,
                dy: Option<i32>,
                button: Option<&'a str>,
                pressed: Option<bool>,
                delta: Option<i32>,
            }
            #[derive(serde::Deserialize)]
            struct Response {
                ok: bool,
                message: String,
            }
            let payload = serde_json::to_vec(&Request {
                version: 1,
                command: &cmd,
                text: txt.as_deref(),
                dx: delta.map(|v| v.0),
                dy: delta.map(|v| v.1),
                button: btn.as_deref(),
                pressed,
                delta: (cmd == "mouse_wheel").then(|| delta.map(|v| v.0)).flatten(),
            })?;
            stream.write_all(&(payload.len() as u32).to_le_bytes())?;
            stream.write_all(&payload)?;
            stream.flush()?;
            let mut len = [0u8; 4];
            stream.read_exact(&mut len)?;
            let size = u32::from_le_bytes(len) as usize;
            if size > 64 * 1024 {
                anyhow::bail!("response too large")
            }
            let mut response = vec![0u8; size];
            stream.read_exact(&mut response)?;
            let response: Response = serde_json::from_slice(&response)?;
            ensure!(response.ok, "{}", response.message);
            Ok(())
        })();
        let _ = tx.send(result);
    });
    match rx.recv_timeout(std::time::Duration::from_secs(3)) {
        Ok(Ok(())) => {
            if matches!(command, "mouse_button" | "mouse_button_state") {
                tracing::info!(command, session, "agent mouse button request delivered");
            } else {
                tracing::debug!(command, session, "agent_request succeeded");
            }
            Ok(())
        }
        Ok(Err(error)) => {
            tracing::warn!(command, session, %error, "agent_request failed");
            Err(error)
        }
        Err(_) => {
            tracing::warn!(command, session, "agent_request timed out after 3s");
            anyhow::bail!("AGENT_REQUEST_TIMEOUT: 输入服务响应超时")
        }
    }
}

#[cfg(target_os = "windows")]
fn agent_request_message(command: &str, text: Option<&str>) -> Result<String> {
    let command = command.to_string();
    let text = text.map(str::to_owned);
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(agent_request_message_inner(&command, text.as_deref()));
    });
    match rx.recv_timeout(std::time::Duration::from_secs(3)) {
        Ok(result) => result,
        Err(_) => anyhow::bail!("AGENT_REQUEST_TIMEOUT: 窗口列表服务响应超时"),
    }
}

#[cfg(target_os = "windows")]
fn agent_request_message_inner(command: &str, text: Option<&str>) -> Result<String> {
    use windows::Win32::System::RemoteDesktop::WTSGetActiveConsoleSessionId;
    let session =
        active_interactive_session().unwrap_or_else(|| unsafe { WTSGetActiveConsoleSessionId() });
    if session == u32::MAX {
        anyhow::bail!("no active console session")
    }
    let pipe = format!(r"\\.\pipe\QingLinkSessionAgent-{session}");
    let mut stream = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(pipe)?;
    #[derive(serde::Serialize)]
    struct Request<'a> {
        version: u32,
        command: &'a str,
        text: Option<&'a str>,
    }
    #[derive(serde::Deserialize)]
    struct Response {
        ok: bool,
        message: String,
    }
    let payload = serde_json::to_vec(&Request {
        version: 1,
        command,
        text,
    })?;
    stream.write_all(&(payload.len() as u32).to_le_bytes())?;
    stream.write_all(&payload)?;
    stream.flush()?;
    let mut len = [0u8; 4];
    stream.read_exact(&mut len)?;
    let size = u32::from_le_bytes(len) as usize;
    ensure!(size > 0 && size <= 64 * 1024, "agent response too large");
    let mut response = vec![0u8; size];
    stream.read_exact(&mut response)?;
    let response: Response = serde_json::from_slice(&response)?;
    ensure!(response.ok, "{}", response.message);
    Ok(response.message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secure_ascii_rejects_control_and_non_ascii_data() {
        assert!(validate_secure_ascii("password").is_ok());
        assert!(validate_secure_ascii("bad\nvalue").is_err());
        assert!(validate_secure_ascii("密碼").is_err());
    }

    #[test]
    fn secure_ascii_rejects_empty_and_oversized_data() {
        assert!(validate_secure_ascii("").is_err());
        assert!(validate_secure_ascii(&"a".repeat(257)).is_err());
    }
}

#[cfg(target_os = "windows")]
fn scan_input(
    scan: u16,
    pressed: bool,
    extended: bool,
) -> windows::Win32::UI::Input::KeyboardAndMouse::INPUT {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP,
        KEYEVENTF_SCANCODE, VIRTUAL_KEY,
    };
    let mut flags = KEYEVENTF_SCANCODE;
    if !pressed {
        flags |= KEYEVENTF_KEYUP;
    }
    if extended {
        flags |= KEYEVENTF_EXTENDEDKEY;
    }
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(0),
                wScan: scan,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

#[cfg(target_os = "windows")]
fn usage_to_scancode(usage: u8) -> Option<(u16, bool)> {
    let scan = match usage {
        0x04..=0x1d => usage as u16 - 0x04 + 0x1e,
        0x1e..=0x27 => usage as u16 - 0x1e + 2,
        0x28 => 0x1c,
        0x29 => 0x01,
        0x2a => 0x0e,
        0x2b => 0x0f,
        0x2c => 0x39,
        0x4a => 0x47,
        0x4b => 0x49,
        0x4c => 0x53,
        0x4d => 0x4f,
        0x4e => 0x51,
        0x4f => 0x4d,
        0x50 => 0x4b,
        0x51 => 0x50,
        0x52 => 0x48,
        _ => return None,
    };
    Some((scan, matches!(usage, 0x4a..=0x52)))
}
