//! 在用户的默认浏览器中打开 URL。
//!
//! # 为什么不用 `cmd /c start`
//!
//! `start` 是 `cmd.exe` 的内建命令，浏览器因此被拉起为本进程子进程的子进程，并
//! 继承了**它的**令牌。这层绕路也是多余的：带空 verb 的 `ShellExecuteW` 会像双击
//! 那样解析默认处理程序（每用户的 `UserChoice`），且不存在一个可能把参数列表搞乱
//! 的中间进程。
//!
//! # 浏览器移交策略
//!
//! 本模块从不挑选浏览器：由系统解析默认处理程序。对已提权的调用方，URL 先交给
//! `explorer.exe`，让交互式用户 shell 在自己的完整性级别上解析浏览器。若 Explorer
//! 无法启动，实现回退到直接调用 `ShellExecuteW`。
//!
//! 直连路径**不会**改变令牌，所以提权后的浏览器仍可能提示管理员模式。
//!
//! # 为什么如实报告结果而不是假定成功
//!
//! 完整性级别*低于* shell 的调用方无法把任何东西交给 shell：强制完整性策略禁止
//! 向上写，但 `ShellExecuteW` **仍然返回成功**，而实际什么也没启动。正是这种静默
//! 无效，使"浏览器没有打开"过去完全无从解释；因此这种情况会被提前发现，并返回
//! [`OpenOutcome::BelowShellIntegrity`]；调用方必须改为让用户自己打开 URL，而不是
//! 假装已经打开。

#[cfg(target_os = "windows")]
use anyhow::Result;

#[cfg(not(target_os = "windows"))]
use anyhow::{Context, Result};

/// What handing the URL to the system actually did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpenOutcome {
    /// The shell accepted the launch; the system resolves the default handler.
    Opened,
    /// This process is below the notification area's integrity level, so the
    /// request would be silently dropped. Nothing was attempted.
    BelowShellIntegrity,
}

/// Open `url` in the default browser.
///
/// # Errors
/// Fails when the shell rejects the launch. A browser that refuses to start for
/// its own reasons (a broken profile, an administrator-mode restriction) is not
/// visible here — the call only reports whether the shell accepted it.
#[cfg(target_os = "windows")]
pub fn open_url(url: &str) -> Result<OpenOutcome> {
    // Below the shell's integrity the hand-off cannot work, and the API gives no
    // error to report. Returning that as an outcome keeps the failure visible.
    if crate::integrity::is_below_shell() == Some(true) {
        return Ok(OpenOutcome::BelowShellIntegrity);
    }
    if is_elevated() {
        match std::process::Command::new("explorer.exe").arg(url).spawn() {
            Ok(_) => return Ok(OpenOutcome::Opened),
            Err(error) => {
                tracing::warn!(%error, "无法通过 Explorer 打开管理界面，回退到默认 shell")
            }
        }
    }
    open_direct(url)
}

/// Whether this process runs with an administrator token.
///
/// A split token (`TokenElevationTypeLimited` with the linked token enabled)
/// reports `false` here, which is what we want: such a process is not elevated
/// and can launch the browser normally.
#[cfg(target_os = "windows")]
pub fn is_elevated() -> bool {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::Security::{
        GetTokenInformation, TOKEN_ELEVATION, TOKEN_QUERY, TokenElevation,
    };
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    // SAFETY: the pseudo process handle needs no release; the token handle is
    // closed below, and the output buffer is correctly sized for its class.
    unsafe {
        let mut token = Default::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
            return false;
        }
        let mut elevation = TOKEN_ELEVATION::default();
        let mut returned = 0_u32;
        let result = GetTokenInformation(
            token,
            TokenElevation,
            Some(&mut elevation as *mut _ as *mut core::ffi::c_void),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut returned,
        )
        .is_ok();
        let _ = CloseHandle(token);
        result && elevation.TokenIsElevated != 0
    }
}

/// `ShellExecuteW` with the default verb.
#[cfg(target_os = "windows")]
fn open_direct(url: &str) -> Result<OpenOutcome> {
    use anyhow::bail;
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
    use windows::core::PCWSTR;

    let operation = to_wide("");
    let file = to_wide(url);
    // SAFETY: both strings are NUL-terminated and outlive the call; a null
    // parent window is allowed and means "no owner window".
    let result = unsafe {
        ShellExecuteW(
            None,
            PCWSTR(operation.as_ptr()),
            PCWSTR(file.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    // ShellExecuteW only fails (<= 32) for a launch the shell could not start at
    // all; document sizes make the boolean comparison explicit.
    let code = result.0 as usize;
    if code <= 32 {
        bail!("ShellExecuteW 无法打开浏览器（返回码 {code}）");
    }
    Ok(OpenOutcome::Opened)
}

#[cfg(target_os = "windows")]
fn to_wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain([0]).collect()
}

/// Only Windows can perform an unelevated shell launch.
#[cfg(not(target_os = "windows"))]
pub fn open_url(url: &str) -> Result<OpenOutcome> {
    let result = std::process::Command::new("xdg-open").arg(url).spawn();
    result.context("cannot open the default browser")?;
    Ok(OpenOutcome::Opened)
}
