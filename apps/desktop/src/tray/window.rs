//! 托盘消息窗口：隐藏顶层窗口、窗口过程、事件分发与右键菜单弹出。
//!
//! 窗口过程由 Win32 直接调用，没有闭包状态，所以控制器在建窗之前把回调需要的
//! 东西（事件通道与菜单句柄）发布到本模块的 `thread_local` 中。窗口只在创建它
//! 的那个线程上收发消息，因此这份状态天然属于 main 线程，托盘操作不会离开它。

use std::cell::RefCell;

use anyhow::{Context, Result};

use windows::Win32::Foundation::{GetLastError, HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, GetCursorPos, HMENU, PostMessageW, RegisterClassW,
    RegisterWindowMessageW, SetForegroundWindow, TPM_BOTTOMALIGN, TPM_LEFTALIGN, TPM_RIGHTBUTTON,
    TrackPopupMenu, WM_COMMAND, WM_DESTROY, WM_LBUTTONUP, WM_NULL, WM_RBUTTONUP, WM_TIMER,
    WNDCLASSW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_OVERLAPPED,
};
use windows::core::{PCWSTR, w};

use crate::tray::menu::{MENU_ID_MASK, TrayAction, action_for};
use crate::tray::shell::{REGISTRATION_TIMER_ID, WM_TRAY_CALLBACK};

/// Class name of the hidden window that receives tray callbacks.
const WINDOW_CLASS: PCWSTR = w!("QingLink.Tray.Window");

/// Message registered by Explorer and broadcast after its notification area is
/// created or recreated.
const TASKBAR_CREATED: PCWSTR = w!("TaskbarCreated");

/// `ERROR_CLASS_ALREADY_EXISTS`, from `RegisterClassW`.
const ERROR_CLASS_ALREADY_EXISTS: u32 = 1410;

/// Events sent from the Win32 window procedure to the application loop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TrayEvent {
    /// A command or click selected by the user.
    Action(TrayAction),
    /// The retry timer says the shell may be ready now.
    RetryRegistration,
    /// Explorer recreated the notification area.
    TaskbarCreated,
}

/// State the window procedure needs, kept per thread.
///
/// The procedure has no closure state, so the controller publishes what it
/// needs here before the window is created. `WndProc` runs only on the thread
/// that created the window, and so does this `thread_local`.
struct WindowState {
    events: tokio::sync::mpsc::UnboundedSender<TrayEvent>,
    menu: HMENU,
}

thread_local! {
    static WINDOW_STATE: RefCell<Option<WindowState>> = const { RefCell::new(None) };
}

/// 发布窗口过程需要的事件通道与菜单句柄。
pub(super) fn set_window_state(events: tokio::sync::mpsc::UnboundedSender<TrayEvent>, menu: HMENU) {
    WINDOW_STATE.with(|slot| {
        *slot.borrow_mut() = Some(WindowState { events, menu });
    });
}

/// 收回窗口过程的状态：建窗失败时回滚，控制器析构时清空。
pub(super) fn clear_window_state() {
    WINDOW_STATE.with(|slot| *slot.borrow_mut() = None);
}

fn taskbar_created_message() -> u32 {
    // RegisterWindowMessage returns a system-wide atom shared by all processes.
    // Calling it lazily keeps class setup independent from shell availability.
    static MESSAGE: std::sync::OnceLock<u32> = std::sync::OnceLock::new();
    *MESSAGE.get_or_init(|| {
        // SAFETY: the string is static and NUL-terminated.
        unsafe { RegisterWindowMessageW(TASKBAR_CREATED) }
    })
}

/// Create the hidden top-level window whose procedure receives tray callbacks
/// and the shell's `TaskbarCreated` broadcast.
pub(super) fn create_tray_window() -> Result<HWND> {
    // SAFETY: the module handle of the running executable needs no release.
    let instance = unsafe { GetModuleHandleW(None) }.context("cannot resolve the module handle")?;
    let class = WNDCLASSW {
        lpfnWndProc: Some(window_procedure),
        hInstance: instance.into(),
        lpszClassName: WINDOW_CLASS,
        ..Default::default()
    };
    // SAFETY: `class` is fully initialised; a duplicate registration attempt is
    // an accepted outcome and reported by the null return value.
    if unsafe { RegisterClassW(&class) } == 0 {
        // SAFETY: reads the thread's last error from `RegisterClassW`.
        let error = unsafe { GetLastError() };
        anyhow::ensure!(
            error.0 == ERROR_CLASS_ALREADY_EXISTS,
            "cannot register the tray window class (error {})",
            error.0
        );
    }

    // Keep this as a hidden top-level window rather than `HWND_MESSAGE`:
    // message-only windows do not receive the broadcast that announces an
    // Explorer/notification-area restart. The tool-window style keeps this
    // zero-sized owner out of the taskbar and Alt-Tab; it is never shown.
    // SAFETY: the class is registered and the module handle is live.
    let hwnd = unsafe {
        CreateWindowExW(
            WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW,
            WINDOW_CLASS,
            w!("QingLink 托盘"),
            WS_OVERLAPPED,
            0,
            0,
            0,
            0,
            None,
            None,
            Some(instance.into()),
            None,
        )
    };
    let hwnd = hwnd.context("cannot create the tray message window")?;

    Ok(hwnd)
}

/// Window procedure for the tray message window.
extern "system" fn window_procedure(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        message if message == taskbar_created_message() => {
            dispatch(TrayEvent::TaskbarCreated);
            LRESULT(0)
        }
        WM_TIMER if wparam.0 == REGISTRATION_TIMER_ID => {
            dispatch(TrayEvent::RetryRegistration);
            LRESULT(0)
        }
        WM_TRAY_CALLBACK => {
            // NOTIFYICON_VERSION_4 delivers the event in the low word of
            // `lparam` and the coordinates in `wparam`.
            let event = (lparam.0 as u32) & 0xffff;
            match event {
                WM_LBUTTONUP => dispatch(TrayEvent::Action(TrayAction::OpenUi)),
                WM_RBUTTONUP => {
                    show_menu(hwnd, wparam);
                }
                _ => {}
            }
            LRESULT(0)
        }
        WM_COMMAND => {
            // The low word carries the command id; the high word the
            // notification code, which this menu does not use.
            let id = (wparam.0 as u32) & MENU_ID_MASK;
            match action_for(id) {
                Some(TrayAction::Exit) => dispatch(TrayEvent::Action(TrayAction::Exit)),
                Some(action) => dispatch(TrayEvent::Action(action)),
                None => {}
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            // The window is going away, which only happens during shutdown: the
            // message loop lives in `main`, so there is nothing to post to.
            LRESULT(0)
        }
        _ => {
            // SAFETY: forwards every other message to the default procedure,
            // which owns all unhandled behaviour.
            unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
        }
    }
}

/// Send one event to the application, which owns the real state.
fn dispatch(event: TrayEvent) {
    WINDOW_STATE.with(|slot| {
        if let Some(state) = slot.borrow().as_ref() {
            let _ = state.events.send(event);
        }
    });
}

/// Show the popup menu at the icon, as the shell reports its position.
fn show_menu(hwnd: HWND, wparam: WPARAM) {
    let anchor = menu_anchor(wparam);
    WINDOW_STATE.with(|slot| {
        let state = slot.borrow();
        let Some(state) = state.as_ref() else {
            return;
        };
        // The menu belongs to a window that is not the foreground window, so the
        // shell dismisses it on the first click elsewhere unless the owner is
        // brought forward first. This pair is the documented workaround.
        // SAFETY: `hwnd` is our live window and the menu handle belongs to it.
        unsafe {
            let _ = SetForegroundWindow(hwnd);
            let _ = TrackPopupMenu(
                state.menu,
                TPM_LEFTALIGN | TPM_BOTTOMALIGN | TPM_RIGHTBUTTON,
                anchor.x,
                anchor.y,
                Some(0),
                hwnd,
                None,
            );
            // Required by the shell after TrackPopupMenu so a subsequent click
            // does not dismiss a stale foreground menu owner.
            let _ = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0));
        }
    });
}

/// Where the popup menu should appear.
///
/// With NOTIFYICON_VERSION_4, `wparam` carries the icon's screen coordinates in
/// its two 16-bit halves. Falling back to the cursor keeps the menu usable when
/// that message version was refused.
fn menu_anchor(wparam: WPARAM) -> POINT {
    let x = (wparam.0 & 0xffff) as u16 as i16 as i32;
    let y = ((wparam.0 >> 16) & 0xffff) as u16 as i16 as i32;
    if x != 0 || y != 0 {
        return POINT { x, y };
    }
    let mut point = POINT::default();
    // SAFETY: writes the cursor position into a live local.
    if unsafe { GetCursorPos(&mut point) }.is_ok() {
        return point;
    }
    POINT { x: 0, y: 0 }
}
