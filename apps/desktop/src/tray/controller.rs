//! `TrayController`：托盘窗口、图标与菜单的持有者与生命周期。
//!
//! 控制器只负责"什么时候做什么"：建窗前的状态发布、菜单与图标的创建与销毁、
//! 注册结果缓存、tooltip 与保持唤醒状态的镜像。具体的 shell 调用在
//! [`super::shell`]，窗口与窗口过程在 [`super::window`]。
//!
//! 控制器不做任何线程切换：它由 `main` 持有，所有方法都必须在创建窗口的那条
//! 线程（main 线程）上调用，托盘操作因此不会离开 main 线程。

use std::cell::RefCell;

use anyhow::{Context, Result};

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{DestroyWindow, HICON};

use crate::tray::icon;
use crate::tray::menu::KEEP_AWAKE_ID;
use crate::tray::report::default_tooltip;
use crate::tray::shell::{
    BLOCKED_RETRY_WINDOW, MAX_REGISTRATION_RETRIES, add_icon, arm_registration_timer,
    finish_accepted, finish_refused, modify_tooltip, notify_data, refused_detail, refused_reason,
    remove_icon, set_icon_version, stop_registration_timer, verify_registration,
};
use crate::tray::window::{TrayEvent, clear_window_state, create_tray_window, set_window_state};
use crate::tray::{RegistrationReport, TrayMenu, TrayStatus, build_tooltip};

/// Result of a registration attempt made after startup.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TrayRetryResult {
    Registered,
    Pending,
    /// The shell refused for a reason retrying cannot fix; the timer is stopped.
    Blocked,
    Exhausted,
}

/// Owns the tray icon, its menu and its message window.
pub struct TrayController {
    hwnd: HWND,
    menu: TrayMenu,
    icon: Option<HICON>,
    tooltip: RefCell<String>,
    registration: RegistrationReport,
    registration_retries: u32,
    /// While set, a refused registration is not retried, because the refusal is
    /// an integrity-level mismatch that retrying cannot change. A shell restart
    /// (`TaskbarCreated`) clears it, since the new notification area may sit at
    /// our own level.
    blocked_until: Option<std::time::Instant>,
}

impl std::fmt::Debug for TrayController {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("TrayController")
            .field("registration", &self.registration)
            .field("blocked_until", &self.blocked_until)
            .field("tooltip", &self.tooltip.borrow())
            .finish_non_exhaustive()
    }
}

impl TrayController {
    /// Create the hidden window, register the icon and install the callbacks.
    ///
    /// Returns the controller plus the channel of user and shell events. A
    /// rejected `NIM_ADD` remains recoverable while Explorer finishes startup.
    ///
    /// # Errors
    /// Fails when the window class, the window, the popup menu or the bitmaps
    /// cannot be created. A shell that refuses registration is reported through
    /// the returned controller and retried without blocking startup.
    pub fn create(
        keep_awake: bool,
    ) -> Result<(Self, tokio::sync::mpsc::UnboundedReceiver<TrayEvent>)> {
        let (events, receiver) = tokio::sync::mpsc::unbounded_channel();
        let menu = TrayMenu::build()?;
        if keep_awake {
            menu.set_checked(KEEP_AWAKE_ID, true);
        }

        let rendered = match icon::brand_icon() {
            Ok(rendered) => rendered,
            Err(error) => {
                tracing::warn!(%error, "托盘图标渲染失败，使用兜底图标");
                icon::fallback_icon()
            }
        };
        let handle = icon::bitmap_from_rgba(&rendered.rgba, rendered.size)
            .context("cannot build the tray icon handle")?;

        // Publish the callback state before the window exists, so a message that
        // arrives during creation still finds its handler.
        set_window_state(events, menu.handle());

        let hwnd = match create_tray_window() {
            Ok(hwnd) => hwnd,
            Err(error) => {
                clear_window_state();
                // SAFETY: the handle came from `bitmap_from_rgba` and is unused.
                unsafe { icon::destroy(handle) };
                return Err(error);
            }
        };

        let mut registration = RegistrationReport {
            icon_size: rendered.size,
            ..RegistrationReport::default()
        };
        let data = notify_data(hwnd, handle);
        let (added, error_code) = add_icon(&data);
        let blocked_until = if added {
            tracing::info!(size = rendered.size, "托盘图标已注册");
            set_icon_version(&data);
            finish_accepted(&mut registration);
            verify_registration(&mut registration, hwnd);
            None
        } else {
            finish_refused(&mut registration, error_code);
            if registration.is_blocked() {
                tracing::error!(
                    error_code = error_code.unwrap_or_default(),
                    integrity = %registration.integrity,
                    shell_integrity = %registration.shell_integrity,
                    reason = %registration.detail,
                    "托盘图标被 shell 永久拒绝（完整性级别不一致），停止重试"
                );
                Some(std::time::Instant::now() + BLOCKED_RETRY_WINDOW)
            } else {
                tracing::warn!(
                    error_code = error_code.unwrap_or_default(),
                    "托盘图标注册被 shell 拒绝，将等待 shell 就绪后重试"
                );
                arm_registration_timer(hwnd);
                None
            }
        };

        Ok((
            Self {
                hwnd,
                menu,
                icon: Some(handle),
                tooltip: RefCell::new(default_tooltip().to_string()),
                registration,
                registration_retries: 0,
                blocked_until,
            },
            receiver,
        ))
    }

    /// Retry registration after a timer tick or a `TaskbarCreated` broadcast.
    pub(crate) fn retry_registration(&mut self, reset_budget: bool) -> TrayRetryResult {
        if reset_budget {
            // The notification area was recreated; the new shell may be at our
            // integrity level, so the block is lifted and the budget restarts.
            self.registration_retries = 0;
            self.registration.visible = false;
            self.registration.verified = false;
            self.registration.rect = None;
            self.registration.blocked_reason = None;
            self.blocked_until = None;
            arm_registration_timer(self.hwnd);
        } else if self.registration.visible {
            return TrayRetryResult::Registered;
        } else if self
            .blocked_until
            .is_some_and(|deadline| std::time::Instant::now() >= deadline)
        {
            // Past the recovery window: stop the timer and keep reporting why.
            stop_registration_timer(self.hwnd);
            return TrayRetryResult::Blocked;
        }

        self.registration_retries = self.registration_retries.saturating_add(1);
        self.registration.attempts = self.registration_retries;
        let Some(icon) = self.icon else {
            return TrayRetryResult::Exhausted;
        };
        let data = notify_data(self.hwnd, icon);
        let (added, error_code) = add_icon(&data);
        if added {
            finish_accepted(&mut self.registration);
            self.blocked_until = None;
            set_icon_version(&data);
            stop_registration_timer(self.hwnd);
            verify_registration(&mut self.registration, self.hwnd);
            tracing::info!(
                attempt = self.registration_retries,
                verified = self.registration.verified,
                "托盘图标已在重试后注册"
            );
            return TrayRetryResult::Registered;
        }

        let reason = refused_reason(error_code);
        if let Some(reason) = reason {
            self.registration.blocked_reason = Some(reason);
            self.registration.detail =
                refused_detail(error_code, &self.registration.blocked_reason);
            if self.blocked_until.is_none() {
                self.blocked_until = Some(std::time::Instant::now() + BLOCKED_RETRY_WINDOW);
            }
            tracing::error!(
                error_code = error_code.unwrap_or_default(),
                integrity = %self.registration.integrity,
                shell_integrity = %self.registration.shell_integrity,
                "托盘图标仍被 shell 永久拒绝（完整性级别不一致）"
            );
            return TrayRetryResult::Blocked;
        }

        self.registration.error_code = error_code;
        self.registration.detail = refused_detail(error_code, &None);
        if self.registration_retries >= MAX_REGISTRATION_RETRIES {
            stop_registration_timer(self.hwnd);
            tracing::warn!(
                attempts = self.registration_retries,
                error_code = error_code.unwrap_or_default(),
                "托盘图标注册重试耗尽"
            );
            TrayRetryResult::Exhausted
        } else {
            TrayRetryResult::Pending
        }
    }

    /// Registration outcome, for display in the web interface.
    pub fn registration(&self) -> RegistrationReport {
        self.registration.clone()
    }

    /// Reflect keep-awake state in the menu.
    pub fn set_keep_awake(&self, enabled: bool) {
        self.menu.set_checked(KEEP_AWAKE_ID, enabled);
    }

    /// Update the tooltip from the current service status.
    ///
    /// Native tooltip updates are relatively expensive, so an unchanged value is
    /// not written again.
    pub fn set_status(&self, status: &TrayStatus) {
        let tooltip = build_tooltip(status);
        if *self.tooltip.borrow() == tooltip {
            return;
        }
        let Some(icon) = self.icon else {
            return;
        };
        if modify_tooltip(self.hwnd, icon, &tooltip) {
            *self.tooltip.borrow_mut() = tooltip;
        }
    }
}

impl Drop for TrayController {
    fn drop(&mut self) {
        stop_registration_timer(self.hwnd);
        remove_icon(self.hwnd);
        if let Some(icon) = self.icon.take() {
            // SAFETY: the handle came from `icon::bitmap_from_rgba`, has not been
            // destroyed before, and the shell no longer references it.
            unsafe { icon::destroy(icon) };
        }
        clear_window_state();
        // SAFETY: the window was created on this thread and is not used again.
        let _ = unsafe { DestroyWindow(self.hwnd) };
    }
}
