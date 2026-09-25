pub fn suspend(pid: u32) -> anyhow::Result<()> {
    crate::process::suspend(pid)
}

pub fn resume(pid: u32) -> anyhow::Result<()> {
    crate::process::resume(pid)
}

pub fn kill(pid: u32) -> anyhow::Result<()> {
    crate::process::kill(pid)
}

pub fn shutdown(force: bool, delay_secs: u64) -> anyhow::Result<()> {
    crate::power::shutdown(force, delay_secs)
}

pub fn sleep(hibernate: bool) -> anyhow::Result<()> {
    crate::power::sleep(hibernate)
}

pub fn restart(force: bool, delay_secs: u64) -> anyhow::Result<()> {
    crate::power::restart(force, delay_secs)
}

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

pub fn install_console_ctrl_handler() -> anyhow::Result<()> {
    use windows::Win32::System::Console::SetConsoleCtrlHandler;
    unsafe {
        if SetConsoleCtrlHandler(Some(console_ctrl_handler), true).is_err() {
            anyhow::bail!("Failed to install console control handler");
        }
    }
    Ok(())
}

pub fn lock() -> anyhow::Result<()> {
    crate::power::lock()
}

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

/// Wake the Windows lock screen and complete either a PIN or confirmation
/// login, then verify that the session is usable.
///
/// Windows does not expose a general-purpose user-mode unlock API, and the
/// lock screen runs on the Winlogon secure desktop where a normal user process
/// cannot inject input (UIPI). The first Enter dismisses the wake/advertisement
/// surface. PIN input is sent only after that transition has settled. When no
/// PIN is supplied, the second Enter is attempted as a confirmation action.
/// The post-action session state, rather than account heuristics, determines
/// whether that action actually unlocked Windows.
pub fn auto_unlock(pin: Option<&str>) -> anyhow::Result<()> {
    if let Some(pin) = pin {
        anyhow::ensure!(
            (4..=32).contains(&pin.len()) && pin.bytes().all(|byte| byte.is_ascii_digit()),
            "INVALID_PIN_FORMAT: PIN 必须是 4-32 位数字"
        );
    } else {
        // These values are useful when diagnosing machines whose lock screen
        // accepts confirmation but whose account/power metadata does not make
        // that capability obvious. They are intentionally advisory only: the
        // actual Enter sequence below is verified by the session state.
        tracing::info!(
            no_password_wake = is_no_password_wake_enabled(),
            account_passwordless = is_account_passwordless(),
            "auto unlock: attempting confirmation flow without a PIN"
        );
    }

    // A UAC prompt also lives on a secure desktop, but confirming it is not the
    // same as dismissing the lock screen. Never send the wake/confirm Enters
    // into a consent dialog.
    if let Ok(status) = crate::secure_desktop::status() {
        if status.uac_detected {
            anyhow::bail!("UAC_PROMPT_ACTIVE: 当前为 UAC 提示，已跳过自动解锁");
        }
    }
    // Sending Enter is only safe while the lock/login surface owns input. Skip
    // only when the session is confidently already usable; an unknown state
    // still goes through the confirmation flow.
    if matches!(query_lock_state(), Some(false)) {
        tracing::info!("auto unlock: session already unlocked");
        return Ok(());
    }

    wait_for_secure_input_ready()?;
    tracing::info!(has_pin = pin.is_some(), "auto unlock: sending wake Enter");
    send_unlock_enter()?;
    tracing::info!("auto unlock: wake Enter delivered");

    // The lock-screen advertisement must first transition to the account/PIN
    // surface. A fixed short delay here prevents PIN characters from being
    // sent while the first Enter is still being consumed by the wake screen.
    std::thread::sleep(std::time::Duration::from_millis(1500));

    if let Some(pin) = pin {
        tracing::info!(length = pin.len(), "auto unlock: submitting PIN");
        submit_lock_pin(pin)
            .map(|_| ())
            .map_err(|error| anyhow::anyhow!("PIN_SENT_NOT_UNLOCKED: {error}"))?;
        wait_for_session_unlock(std::time::Duration::from_secs(8))?;
        return Ok(());
    }

    tracing::info!("auto unlock: sending confirmation Enter");
    send_unlock_enter()?;
    tracing::info!("auto unlock: confirmation Enter delivered; waiting for session unlock");
    wait_for_session_unlock(std::time::Duration::from_secs(8))
}

fn send_unlock_enter() -> anyhow::Result<()> {
    if crate::secure_desktop::running_in_session_zero() {
        crate::secure_desktop::send_hid_key(0x28, 0, true)?;
        crate::secure_desktop::send_hid_key(0x28, 0, false)?;
    } else {
        send_return_keypress()?;
    }
    Ok(())
}

fn wait_for_secure_input_ready() -> anyhow::Result<()> {
    if !crate::secure_desktop::running_in_session_zero() {
        return Ok(());
    }
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        match crate::secure_desktop::status() {
            Ok(status) if status.input_ready && status.agent_state == "ready" => {
                tracing::debug!(
                    session_id = status.session_id,
                    desktop = %status.desktop,
                    "auto unlock: secure input agent ready"
                );
                return Ok(());
            }
            Ok(status) => {
                tracing::debug!(
                    session_id = status.session_id,
                    desktop = %status.desktop,
                    agent_state = %status.agent_state,
                    input_ready = status.input_ready,
                    "auto unlock: waiting for secure input agent"
                );
            }
            Err(error) => {
                tracing::debug!(%error, "auto unlock: secure input status unavailable");
            }
        }
        if std::time::Instant::now() >= deadline {
            anyhow::bail!("SESSION_AGENT_NOT_READY: 安全桌面输入代理未就绪")
        }
        std::thread::sleep(std::time::Duration::from_millis(250));
    }
}

fn wait_for_session_unlock(timeout: std::time::Duration) -> anyhow::Result<()> {
    let deadline = std::time::Instant::now() + timeout;
    let mut saw_locked = false;
    loop {
        match query_lock_state() {
            Some(false) => return Ok(()),
            Some(true) => saw_locked = true,
            None => {}
        }
        if std::time::Instant::now() >= deadline {
            if saw_locked {
                anyhow::bail!(
                    "LOGIN_SURFACE_NOT_READY: 解锁操作未完成，可能需要 PIN 或密码，请使用控制页输入"
                )
            }
            // Windows never gave a usable answer; the input was still
            // delivered, so do not turn a blind spot into a false failure.
            tracing::warn!(
                "wait_for_session_unlock: lock state unavailable; assuming input delivered"
            );
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(250));
    }
}

/// Best-effort lock-screen observation.
///
/// `Some(true)` means the lock/login surface is showing, `Some(false)` means the
/// interactive session is usable, and `None` means Windows did not provide a
/// usable answer. The WTS session flag is authoritative; the session agent's
/// view of the input desktop is only a fallback when WTS is inconclusive.
fn query_lock_state() -> Option<bool> {
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
        if let Ok(status) = crate::secure_desktop::status() {
            if status.agent_state != "unavailable" {
                return Some(status.locked);
            }
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
        WTSQuerySessionInformationW(
            None,
            session_id,
            WTSSessionInfoEx,
            &mut buffer,
            &mut bytes,
        )?;
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

const LOCK_INPUT_PIPE: &str = r"\\.\pipe\QingLinkControlV2";

#[derive(Clone, serde::Serialize)]
struct LockInputPipeRequest<'a> {
    version: u32,
    command: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pin: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dx: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dy: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    button: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pressed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    delta: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    usage: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    modifiers: Option<u8>,
}

#[derive(serde::Deserialize)]
struct LockInputPipeResponse {
    ok: bool,
    code: String,
    #[serde(default)]
    message: String,
}

fn lock_input_pipe(request: &LockInputPipeRequest<'_>) -> anyhow::Result<LockInputPipeResponse> {
    let mut pipe = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(LOCK_INPUT_PIPE)
        .map_err(|_| anyhow::anyhow!("LOCK_INPUT_SERVICE_UNAVAILABLE: 输入服务未运行"))?;
    let payload = serde_json::to_vec(request)?;
    pipe.write_all(&(payload.len() as u32).to_le_bytes())?;
    pipe.write_all(&payload)?;
    pipe.flush()?;
    let mut len = [0u8; 4];
    pipe.read_exact(&mut len)?;
    anyhow::ensure!(
        len[0] as usize
            | ((len[1] as usize) << 8)
            | ((len[2] as usize) << 16)
            | ((len[3] as usize) << 24)
            <= 64 * 1024,
        "LOCK_INPUT_INVALID_RESPONSE: 输入服务响应过大"
    );
    let mut response = vec![0u8; u32::from_le_bytes(len) as usize];
    pipe.read_exact(&mut response)?;
    Ok(serde_json::from_slice(&response)?)
}

pub fn get_lock_input_status() -> anyhow::Result<qinglink_protocol::LockInputStatus> {
    let response = lock_input_pipe(&LockInputPipeRequest {
        version: 1,
        command: "status",
        pin: None,
        dx: None,
        dy: None,
        button: None,
        pressed: None,
        delta: None,
        usage: None,
        modifiers: None,
    });
    match response {
        Ok(response) => Ok(qinglink_protocol::LockInputStatus {
            available: response.ok,
            service_running: true,
            message: response.message,
        }),
        Err(error)
            if error
                .to_string()
                .starts_with("LOCK_INPUT_SERVICE_UNAVAILABLE") =>
        {
            Ok(qinglink_protocol::LockInputStatus {
                available: false,
                service_running: false,
                message: error.to_string(),
            })
        }
        Err(error) => Err(error),
    }
}

pub fn submit_lock_pin(pin: &str) -> anyhow::Result<qinglink_protocol::LockInputResult> {
    anyhow::ensure!(
        (4..=32).contains(&pin.len()) && pin.bytes().all(|byte| byte.is_ascii_digit()),
        "INVALID_PIN_FORMAT: PIN 必须是 4-32 位数字"
    );
    if crate::secure_desktop::running_in_session_zero() || is_secure_desktop()? {
        // A secure-desktop request must stay on the SYSTEM session-agent
        // route. Never retry through the service pipe, which would make a
        // failed injection look successful while Session 0 is unable to see
        // Winlogon.
        crate::secure_desktop::send_secure_ascii(pin)
            .context("SECURE_INPUT_FAILED: 无法向安全桌面发送 PIN")?;
        crate::secure_desktop::send_hid_key(0x28, 0, true)
            .context("SECURE_INPUT_FAILED: 无法发送确认按键")?;
        crate::secure_desktop::send_hid_key(0x28, 0, false)
            .context("SECURE_INPUT_FAILED: 无法释放确认按键")?;

        // SendInput returning success only means that Windows accepted the
        // event. Confirm that the lock surface is no longer up; an unknown
        // state is treated as accepted so a detection blind spot does not
        // report a delivered PIN as failed.
        for _ in 0..20 {
            std::thread::sleep(std::time::Duration::from_millis(250));
            if query_lock_state() != Some(true) {
                return Ok(qinglink_protocol::LockInputResult {
                    code: "SECURE_INPUT_ACCEPTED".into(),
                    message: "PIN 已通过安全桌面输入并完成解锁".into(),
                });
            }
        }
        anyhow::bail!("SECURE_INPUT_NOT_ACCEPTED: PIN 已发送，但 Windows 仍处于锁屏安全桌面")
    }
    let response = lock_input_pipe(&LockInputPipeRequest {
        version: 1,
        command: "submit_pin",
        pin: Some(pin),
        dx: None,
        dy: None,
        button: None,
        pressed: None,
        delta: None,
        usage: None,
        modifiers: None,
    })?;
    anyhow::ensure!(response.ok, "{}: {}", response.code, response.message);
    wait_for_session_unlock(std::time::Duration::from_secs(8))?;
    Ok(qinglink_protocol::LockInputResult {
        code: response.code,
        message: response.message,
    })
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

pub fn is_secure_desktop() -> anyhow::Result<bool> {
    let status = crate::secure_desktop::status()?;
    Ok(status.locked
        || status.uac_detected
        || !status.desktop.eq_ignore_ascii_case("default"))
}

fn is_auto_admin_logon_enabled() -> bool {
    read_winlogon_value("AutoAdminLogon").as_deref() == Some("1")
        && read_winlogon_value("DefaultUserName")
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false)
}

/// Returns true when the current interactive user can sign in with an empty
/// password, i.e. the account has no password and its lock screen only asks
/// for a confirmation. This mirrors what the Windows lock screen itself will
/// accept, so "no_password" mode does not depend on the power scheme setting.
fn is_account_passwordless() -> bool {
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

fn is_no_password_wake_enabled() -> bool {
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

fn send_return_keypress() -> anyhow::Result<()> {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, SendInput, VK_RETURN,
    };

    let inputs = [
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_RETURN,
                    wScan: 0,
                    dwFlags: Default::default(),
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_RETURN,
                    wScan: 0,
                    dwFlags: KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
    ];

    let inserted = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
    anyhow::ensure!(
        inserted == inputs.len() as u32,
        "无法向 Windows 锁屏发送确认操作"
    );
    Ok(())
}

/// Configure Windows to not require a password on wake from sleep.
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

fn read_winlogon_value(value_name: &str) -> Option<String> {
    read_hklm_string(
        r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Winlogon",
        value_name,
    )
}

fn read_hklm_string(subkey: &str, value_name: &str) -> Option<String> {
    use windows::Win32::System::Registry::{
        HKEY, HKEY_LOCAL_MACHINE, KEY_READ, REG_SZ, RegCloseKey, RegOpenKeyExW, RegQueryValueExW,
    };

    unsafe {
        let subkey = to_wide_z(subkey);
        let mut hkey = HKEY::default();
        if RegOpenKeyExW(
            HKEY_LOCAL_MACHINE,
            windows::core::PCWSTR(subkey.as_ptr()),
            Some(0),
            KEY_READ,
            &mut hkey,
        )
        .is_err()
        {
            return None;
        }

        let name = to_wide_z(value_name);
        let mut kind = REG_SZ;
        let mut buf = vec![0u16; 512];
        let mut len = (buf.len() * 2) as u32;
        let result = RegQueryValueExW(
            hkey,
            windows::core::PCWSTR(name.as_ptr()),
            None,
            Some(&mut kind),
            Some(buf.as_mut_ptr() as *mut u8),
            Some(&mut len),
        );
        let _ = RegCloseKey(hkey);
        if result.is_err() || kind != REG_SZ || len < 2 {
            return None;
        }

        let chars = (len as usize).saturating_sub(2) / 2;
        Some(String::from_utf16_lossy(&buf[..chars]))
    }
}

fn read_hklm_dword(subkey: &str, value_name: &str) -> Option<u32> {
    use windows::Win32::System::Registry::{
        HKEY, HKEY_LOCAL_MACHINE, KEY_READ, REG_DWORD, RegCloseKey, RegOpenKeyExW, RegQueryValueExW,
    };

    unsafe {
        let subkey = to_wide_z(subkey);
        let mut hkey = HKEY::default();
        if RegOpenKeyExW(
            HKEY_LOCAL_MACHINE,
            windows::core::PCWSTR(subkey.as_ptr()),
            Some(0),
            KEY_READ,
            &mut hkey,
        )
        .is_err()
        {
            return None;
        }

        let name = to_wide_z(value_name);
        let mut kind = REG_DWORD;
        let mut value = 0u32;
        let mut len = std::mem::size_of::<u32>() as u32;
        let result = RegQueryValueExW(
            hkey,
            windows::core::PCWSTR(name.as_ptr()),
            None,
            Some(&mut kind),
            Some(&mut value as *mut u32 as *mut u8),
            Some(&mut len),
        );
        let _ = RegCloseKey(hkey);
        if result.is_err() || kind != REG_DWORD || len != std::mem::size_of::<u32>() as u32 {
            return None;
        }

        Some(value)
    }
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

