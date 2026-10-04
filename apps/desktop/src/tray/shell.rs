//! 通知区域：`Shell_NotifyIcon` 的直接调用、失败判读与重试预算。
//!
//! 这里只做四件事：组装 [`NOTIFYICONDATAW`]、调用 shell 并把返回值与
//! `GetLastError` 翻译成结论、按定时器重试直到预算耗尽、把每一步的结论写回
//! [`RegistrationReport`]。之所以每次调用都留痕，是因为"图标没出现"只有
//! `Shell_NotifyIcon` 的返回值能说明，而这个返回值一旦被吞掉就无法还原。
//!
//! 所有函数都必须在创建托盘窗口的那个线程（main 线程）上调用。

use windows::Win32::Foundation::{GetLastError, HWND, RECT};
use windows::Win32::UI::Shell::{
    NIF_ICON, NIF_MESSAGE, NIF_SHOWTIP, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY, NIM_SETVERSION,
    NOTIFYICON_VERSION_4, NOTIFYICONDATAW, NOTIFYICONIDENTIFIER, Shell_NotifyIconGetRect,
    Shell_NotifyIconW,
};
use windows::Win32::UI::WindowsAndMessaging::{HICON, KillTimer, SetTimer, WM_APP};
use windows::core::GUID;

use crate::tray::report::default_tooltip;
use crate::tray::{RegistrationReport, TrayRect, blocked_reason};

/// Icon identity inside the notification area.
const ICON_ID: u32 = 1;

/// Timer used while Explorer is starting and has not accepted the icon yet.
pub(super) const REGISTRATION_TIMER_ID: usize = 1;
const REGISTRATION_RETRY_INTERVAL_MS: u32 = 250;
pub(super) const MAX_REGISTRATION_RETRIES: u32 = 20;

/// How long a refused registration keeps the retry timer armed.
///
/// A refusal with `ERROR_ACCESS_DENIED` is almost always an integrity-level
/// mismatch — the shell refuses icons from a process below or above its own level
/// — and no amount of retrying changes that. The budget exists for the one case
/// that can change: Explorer restarts, after which the shell may be at our level.
/// Past this deadline the controller stops retrying and keeps reporting why.
pub(super) const BLOCKED_RETRY_WINDOW: std::time::Duration = std::time::Duration::from_secs(60);

/// Message the shell sends for events on our icon.
///
/// `WM_APP + 1` is in the range reserved for application use; `tray-icon` used
/// `WM_USER + 6002`, which is legal but needlessly far from the documented
/// application range.
pub(super) const WM_TRAY_CALLBACK: u32 = WM_APP + 1;

/// Add one icon and capture the only reliable failure detail exposed by the API.
pub(super) fn add_icon(data: &NOTIFYICONDATAW) -> (bool, Option<u32>) {
    // SAFETY: the structure is fully initialised and the shell only reads it.
    let added = unsafe { Shell_NotifyIconW(NIM_ADD, data) }.as_bool();
    let error_code = (!added).then(|| {
        // SAFETY: `GetLastError` is thread-local and belongs to the call above.
        unsafe { GetLastError() }.0
    });
    (added, error_code)
}

/// Set the modern callback version after a successful add.
///
/// `NOTIFYICON_VERSION_4` is what makes the callback carry the icon's screen
/// coordinates, which is what lets the context menu open next to the icon. A
/// refusal is not fatal — the menu falls back to the cursor position — so it is a
/// warning with the shell's own error code rather than a failure.
pub(super) fn set_icon_version(data: &NOTIFYICONDATAW) {
    // SAFETY: the same fully initialised structure is read-only for this call.
    let versioned = unsafe { Shell_NotifyIconW(NIM_SETVERSION, data) }.as_bool();
    if !versioned {
        // SAFETY: `GetLastError` is thread-local and belongs to the call above.
        let error_code = unsafe { GetLastError() }.0;
        tracing::warn!(
            error_code,
            "托盘图标未能设置 NOTIFYICON_VERSION_4，右键菜单将使用光标位置"
        );
    }
}

/// Overwrite the tooltip of a live icon; `true` when the shell accepted it.
pub(super) fn modify_tooltip(hwnd: HWND, icon: HICON, tooltip: &str) -> bool {
    let mut data = blank_notify_data(hwnd);
    data.uFlags = NIF_TIP;
    data.hIcon = icon;
    write_tip(&mut data, tooltip);
    // SAFETY: the structure is fully initialised and only read by the call.
    unsafe { Shell_NotifyIconW(NIM_MODIFY, &data) }.as_bool()
}

/// Remove the icon from the notification area.
pub(super) fn remove_icon(hwnd: HWND) {
    // `Remove` on exit keeps a stale icon out of the notification area when
    // the process dies without the shell noticing.
    let mut data = blank_notify_data(hwnd);
    data.uFlags = NIF_MESSAGE;
    // SAFETY: the structure is fully initialised; the call is a no-op when
    // the icon is already gone.
    unsafe {
        let _ = Shell_NotifyIconW(NIM_DELETE, &data);
    }
}

pub(super) fn arm_registration_timer(hwnd: HWND) {
    // SAFETY: `hwnd` belongs to this thread and the timer callback is delivered
    // through its normal message queue.
    let _ = unsafe {
        SetTimer(
            Some(hwnd),
            REGISTRATION_TIMER_ID,
            REGISTRATION_RETRY_INTERVAL_MS,
            None,
        )
    };
}

pub(super) fn stop_registration_timer(hwnd: HWND) {
    // SAFETY: the timer belongs to this window and is stopped on its owner thread.
    unsafe {
        let _ = KillTimer(Some(hwnd), REGISTRATION_TIMER_ID);
    }
}

/// The `detail` written after a registration the shell accepted.
fn accepted_detail() -> String {
    "Shell_NotifyIcon(NIM_ADD) 已接受托盘图标".to_string()
}

/// Why a refusal cannot be retried into success, if that is already decidable.
///
/// The refusal that matters is `ERROR_ACCESS_DENIED`: the shell returns it when
/// the caller sits at a different integrity level than the notification area, and
/// no amount of retrying changes a token. A process below the shell usually
/// cannot read the shell's token either — that is why the level is reported as
/// `unknown` in this case, which is itself the diagnostic.
pub(super) fn refused_reason(error_code: Option<u32>) -> Option<String> {
    let error_code = error_code.unwrap_or_default();
    let integrity = integrity_level();
    let shell = shell_integrity_level().unwrap_or_else(|| "unknown".to_string());
    blocked_reason(
        false,
        error_code,
        &integrity,
        &shell,
        qinglink_windows::integrity::is_below_shell().unwrap_or(false),
    )
}

/// The `detail` for a refused registration, recoverable or not.
pub(super) fn refused_detail(error_code: Option<u32>, blocked_reason: &Option<String>) -> String {
    match blocked_reason {
        Some(reason) => format!(
            "Shell_NotifyIcon(NIM_ADD) 被 shell 拒绝，GetLastError={}。{reason}",
            error_code.unwrap_or_default()
        ),
        None => format!(
            "Shell_NotifyIcon(NIM_ADD) 被 shell 拒绝，GetLastError={}；将等待 shell 就绪后重试",
            error_code.unwrap_or_default()
        ),
    }
}

/// Record a registration the shell accepted.
///
/// Every accepted path goes through here, so the report cannot end up claiming
/// "registered" while still describing the previous state: the acceptance is what
/// `visible` means, and the integrity snapshot is what makes the report readable
/// next to a refusal from an earlier or later attempt.
pub(super) fn finish_accepted(registration: &mut RegistrationReport) {
    registration.visible = true;
    registration.error_code = None;
    registration.blocked_reason = None;
    registration.detail = accepted_detail();
    registration.integrity = integrity_level();
    registration.shell_integrity = shell_integrity_level().unwrap_or_else(|| "unknown".to_string());
}

/// Record a refused registration: the integrity names, the reason and the detail.
pub(super) fn finish_refused(registration: &mut RegistrationReport, error_code: Option<u32>) {
    registration.visible = false;
    registration.verified = false;
    registration.rect = None;
    registration.error_code = error_code;
    registration.integrity = integrity_level();
    registration.shell_integrity = shell_integrity_level().unwrap_or_else(|| "unknown".to_string());
    registration.blocked_reason = refused_reason(error_code);
    registration.detail = refused_detail(error_code, &registration.blocked_reason);
}

/// Ask the shell where the icon actually is.
///
/// `Shell_NotifyIcon(NIM_ADD)` answers "the shell accepted the call"; this answers
/// "the icon is in the notification area". That is the documented check, and it is
/// what makes "registered but folded away" and "never registered" distinguishable
/// without guessing.
pub(super) fn verify_registration(registration: &mut RegistrationReport, hwnd: HWND) {
    match icon_rect(hwnd) {
        Some(rect) => {
            registration.verified = true;
            registration.rect = Some(TrayRect {
                x: rect.left,
                y: rect.top,
                width: rect.right.saturating_sub(rect.left) as u32,
                height: rect.bottom.saturating_sub(rect.top) as u32,
            });
        }
        None => {
            // The shell answered with an error: either the icon is in the
            // overflow flyout, or the shell is not ready yet. `visible` keeps the
            // result of the registration call itself.
            tracing::debug!("Shell_NotifyIconGetRect 未返回图标位置（可能仍在折叠区）");
        }
    }
}

/// Ask the shell for the icon's screen rectangle.
fn icon_rect(hwnd: HWND) -> Option<RECT> {
    let identifier = NOTIFYICONIDENTIFIER {
        cbSize: std::mem::size_of::<NOTIFYICONIDENTIFIER>() as u32,
        hWnd: hwnd,
        uID: ICON_ID,
        guidItem: GUID::zeroed(),
    };
    // SAFETY: `identifier` is fully initialised and the call only reads it.
    unsafe { Shell_NotifyIconGetRect(&identifier) }.ok()
}

/// Integrity level of the process that owns the notification area.
///
/// Comparing the two turns "the icon does not appear" into a sentence: the shell
/// only accepts icons from a process at its own integrity level.
fn shell_integrity_level() -> Option<String> {
    qinglink_windows::integrity::shell_integrity().map(|level| level.name().to_string())
}

/// Human-readable integrity level of this process's token.
///
/// Placed in the report because a mismatch with the shell is the one tray
/// failure the application itself cannot work around.
fn integrity_level() -> String {
    qinglink_windows::integrity::current_integrity_name()
}

/// A `NOTIFYICONDATAW` with only `cbSize`, the window and the id filled in.
fn blank_notify_data(hwnd: HWND) -> NOTIFYICONDATAW {
    // SAFETY: the structure is a plain C aggregate of integers, pointers and
    // fixed-size arrays; all-zero is the documented "unset" state and every
    // field the shell reads is written by the caller.
    let mut data: NOTIFYICONDATAW = unsafe { std::mem::zeroed() };
    data.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    data.hWnd = hwnd;
    data.uID = ICON_ID;
    data
}

/// Fill in the tooltip array, truncating on a character boundary.
fn write_tip(data: &mut NOTIFYICONDATAW, tooltip: &str) {
    let capacity = data.szTip.len().saturating_sub(1);
    let encoded: Vec<u16> = tooltip.encode_utf16().take(capacity).collect();
    data.szTip[..encoded.len()].copy_from_slice(&encoded);
    for slot in &mut data.szTip[encoded.len()..] {
        *slot = 0;
    }
}

/// Build the registration structure for a new icon.
pub(super) fn notify_data(hwnd: HWND, handle: HICON) -> NOTIFYICONDATAW {
    let mut data = blank_notify_data(hwnd);
    data.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP | NIF_SHOWTIP;
    data.uCallbackMessage = WM_TRAY_CALLBACK;
    data.hIcon = handle;
    write_tip(&mut data, default_tooltip());
    // The version field shares storage with `uTimeout`; it is only read when
    // `NIF_INFO` is set, which never happens here.
    data.Anonymous.uVersion = NOTIFYICON_VERSION_4;
    data
}
