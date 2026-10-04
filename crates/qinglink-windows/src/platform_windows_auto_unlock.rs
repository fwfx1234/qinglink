//! 自动解锁流程：唤醒锁屏、提交 PIN 或确认键，并等待会话恢复可用。

use super::lock_input_pipe::submit_lock_pin;
use super::session_account::{is_account_passwordless, is_no_password_wake_enabled};
use super::session_lock::query_lock_state;

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
///
/// # Errors
/// 以下情况返回错误：`pin` 长度不在 4-32 位或含非数字字符
/// （“INVALID_PIN_FORMAT: PIN 必须是 4-32 位数字”）；当前是 UAC 提示
/// （“UAC_PROMPT_ACTIVE: 当前为 UAC 提示，已跳过自动解锁”）；10 秒内安全桌面输入
/// 代理未就绪（“SESSION_AGENT_NOT_READY: 安全桌面输入代理未就绪”）；发送唤醒或
/// 确认回车失败（透传 `send_unlock_enter` 的错误）；提交 PIN 失败（错误带
/// `PIN_SENT_NOT_UNLOCKED: ` 前缀）；或 8 秒内仍未解锁
/// （“LOGIN_SURFACE_NOT_READY: 解锁操作未完成，可能需要 PIN 或密码，请使用控制页输入”）。
/// 已确认会话可用时直接返回 `Ok(())`。
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
    if let Ok(status) = crate::secure_desktop::status()
        && status.uac_detected
    {
        anyhow::bail!("UAC_PROMPT_ACTIVE: 当前为 UAC 提示，已跳过自动解锁");
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

pub(super) fn wait_for_session_unlock(timeout: std::time::Duration) -> anyhow::Result<()> {
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
