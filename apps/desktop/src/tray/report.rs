//! 托盘对外报告：服务状态、图标几何、注册结果与失败分类。
//!
//! 这里只有数据模型与纯函数，不碰 shell：`blocked_reason` 只根据注册返回值与
//! 两边的完整性级别给出结论，所以"哪些失败重试也没有用"可以在没有通知区域的
//! 环境里单独判定。这些类型是 `--self-check`、Web 接口与 [`TrayController`]
//! 共用的词汇表。
//!
//! [`TrayController`]: super::TrayController

/// Service state as reflected in the tooltip.
#[derive(Clone, Debug, Default)]
pub struct TrayStatus {
    /// The service protocol server is reachable.
    pub service_running: bool,
    /// Most recent failure worth showing the user.
    pub service_error: Option<String>,
}

/// Shell-reported icon geometry.
#[derive(Clone, Copy, Debug, serde::Serialize)]
pub struct TrayRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// What happened when the icon was registered.
///
/// `detail` is written for a human reading `--self-check` output, because the
/// difference between "the shell refused the call", "the shell accepted it and
/// put it in the overflow flyout" and "a code bug" is the difference between three
/// very different responses.
#[derive(Clone, Debug, serde::Serialize)]
pub struct RegistrationReport {
    /// `Shell_NotifyIcon(NIM_ADD)` returned non-zero.
    pub visible: bool,
    /// `Shell_NotifyIconGetRect` confirmed the icon is in the notification area.
    ///
    /// This is the documented way to ask the shell where the icon is, and it is
    /// therefore the only answer that distinguishes "registered but hidden in the
    /// overflow flyout" from "registered and on the taskbar". A rejected
    /// registration leaves it `false`.
    pub verified: bool,
    /// Geometry reported by `Shell_NotifyIconGetRect`, when the shell answered.
    ///
    /// `None` after a successful registration means the shell did not answer —
    /// the icon is hidden in the overflow flyout, or the shell is busy.
    pub rect: Option<TrayRect>,
    /// Pixel size of the rasterized icon.
    pub icon_size: u32,
    /// `GetLastError` from the registration call, when it failed.
    pub error_code: Option<u32>,
    /// Whether the process runs at the same integrity level as the shell.
    ///
    /// Windows refuses notification-area icons registered by a process at a
    /// *different* integrity level than the taskbar, in either direction. This is
    /// the one tray failure the application cannot work around, so the shell's own
    /// level is reported next to ours.
    pub integrity: String,
    /// Integrity level of the process that owns the notification area.
    pub shell_integrity: String,
    /// Per-process retry counter, for the diagnostics panel.
    pub attempts: u32,
    /// Why this registration cannot succeed, when that is already decidable.
    ///
    /// `None` means the failure — if any — is considered recoverable, so the retry
    /// timer keeps running.
    pub blocked_reason: Option<String>,
    /// Human-readable outcome, suitable for display in the web interface.
    pub detail: String,
}

impl Default for RegistrationReport {
    fn default() -> Self {
        Self {
            visible: false,
            verified: false,
            rect: None,
            icon_size: 0,
            error_code: None,
            integrity: String::from("unknown"),
            shell_integrity: String::from("unknown"),
            attempts: 0,
            blocked_reason: None,
            detail: String::from("尚未注册"),
        }
    }
}

impl RegistrationReport {
    /// Whether this registration cannot be fixed by retrying.
    pub fn is_blocked(&self) -> bool {
        self.blocked_reason.is_some()
    }
}

/// `ERROR_ACCESS_DENIED`, the error a shell of a different integrity level returns.
const ERROR_ACCESS_DENIED: u32 = 5;

/// Why a refused registration cannot be retried into success.
///
/// Pure by design: it takes the shell's answer and the two integrity names and
/// decides, so the classification is testable without a shell. `below_shell` is
/// computed by the caller because a process below the shell may not be able to
/// read the shell's token at all.
pub fn blocked_reason(
    added: bool,
    error_code: u32,
    integrity: &str,
    shell_integrity: &str,
    below_shell: bool,
) -> Option<String> {
    if added || error_code != ERROR_ACCESS_DENIED {
        return None;
    }
    if below_shell {
        return Some(format!(
            "本进程完整性为 {integrity}，低于通知区域所有者（{shell_integrity}），\
             shell 拒绝注册托盘图标（GetLastError={ERROR_ACCESS_DENIED}）。\
             完整性由启动它的宿主决定，应用自身无法提升到任务栏级别：\
             请从资源管理器双击或普通用户终端启动（不要从沙箱、受限终端或低完整性宿主启动）。"
        ));
    }
    if shell_integrity != "unknown" {
        return Some(format!(
            "本进程完整性为 {integrity}，与通知区域所有者（{shell_integrity}）不一致，\
             shell 拒绝注册托盘图标（GetLastError={ERROR_ACCESS_DENIED}）。\
             通知区域不接受跨完整性级别的图标；请让任务栏与桌面端处于同一完整性级别。"
        ));
    }
    None
}

/// Tooltip text for a status.
pub fn build_tooltip(status: &TrayStatus) -> String {
    if let Some(error) = status.service_error.as_deref().filter(|e| !e.is_empty()) {
        let short: String = error.chars().take(60).collect();
        return format!("轻联 QingLink · 服务异常：{short}");
    }
    if status.service_running {
        "轻联 QingLink · 服务运行中".to_string()
    } else {
        "轻联 QingLink · 服务未连接".to_string()
    }
}

/// 尚未拿到服务状态时写入图标的提示文本。
pub(super) fn default_tooltip() -> &'static str {
    "轻联 QingLink · 启动中"
}

#[cfg(test)]
mod tests {
    use super::{ERROR_ACCESS_DENIED, blocked_reason};

    #[test]
    fn accepted_registration_is_never_blocked() {
        assert_eq!(blocked_reason(true, 0, "medium", "medium", false), None);
    }

    #[test]
    fn a_process_below_the_shell_is_blocked_with_both_levels_named() {
        let reason = blocked_reason(false, ERROR_ACCESS_DENIED, "low", "medium", true)
            .expect("an integrity mismatch must produce a reason");
        assert!(
            reason.contains("low"),
            "reason must name our level: {reason}"
        );
        assert!(
            reason.contains("medium"),
            "reason must name the shell level: {reason}"
        );
        assert!(
            reason.contains("GetLastError=5"),
            "reason must carry the shell's error: {reason}"
        );
    }

    #[test]
    fn a_readable_shell_of_another_level_is_blocked_without_claiming_we_are_lower() {
        let reason = blocked_reason(false, ERROR_ACCESS_DENIED, "high", "medium", false)
            .expect("a level mismatch in either direction must produce a reason");
        assert!(
            reason.contains("high") && reason.contains("medium"),
            "{reason}"
        );
        assert!(
            !reason.contains("低于通知区域所有者"),
            "an elevated process is not below the shell: {reason}"
        );
    }

    #[test]
    fn access_denied_with_an_unreadable_shell_is_retried_rather_than_declared_permanent() {
        // The shell token could not be read, so the mismatch cannot be proven;
        // Explorer may simply still be starting, and the retry timer stays armed.
        assert_eq!(
            blocked_reason(false, ERROR_ACCESS_DENIED, "medium", "unknown", false),
            None
        );
    }

    #[test]
    fn other_shell_errors_stay_recoverable() {
        // 1460 (`ERROR_TIMEOUT`) is what a shell that is still starting returns.
        assert_eq!(blocked_reason(false, 1460, "medium", "medium", false), None);
    }
}
