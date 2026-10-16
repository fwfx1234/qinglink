//! 配对提示窗口本体：建窗、设文本、显示/隐藏，以及窗口过程。
//!
//! 与托盘窗口不同，这个窗口是**可见**的：它存在的意义就是让没开浏览器的用户也能拿到
//! 配对码。窗口由系统绘制（STATIC / BUTTON 子控件 + `WM_CTLCOLORSTATIC` 上色），
//! 自绘 `WM_PAINT` 会把高 DPI、主题与字体适配变成另一摊需要长期维护的东西。
//!
//! 本模块只做"窗口机制"：什么时候显示、什么时候关闭全部由 [`super::PairingPrompt`]
//! 与主循环决定。
//!
//! 所有函数都必须在创建窗口的那个线程（main 线程）上调用：窗口消息只投递给创建它的
//! 线程，线程局部状态也是按这个前提设计的（与 `tray::window` 同一模式，见
//! `docs/architecture.md` 第 4 节）。

use std::cell::RefCell;

use anyhow::{Context, Result};
use tokio::sync::mpsc::UnboundedSender;
use windows::Win32::Foundation::{
    COLORREF, GetLastError, HINSTANCE, HWND, LPARAM, LRESULT, WPARAM,
};
use windows::Win32::Graphics::Gdi::{
    COLOR_BTNFACE, COLOR_HIGHLIGHT, COLOR_WINDOWTEXT, GetSysColor, GetSysColorBrush, HDC,
    SetBkMode, SetTextColor, TRANSPARENT,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetDlgCtrlID, IDC_ARROW, LoadCursorW,
    RegisterClassW, SW_HIDE, SW_SHOWNORMAL, SetForegroundWindow, SetWindowTextW, ShowWindow,
    WM_CLOSE, WM_COMMAND, WM_CTLCOLORSTATIC, WNDCLASSW, WS_CAPTION, WS_CLIPCHILDREN,
    WS_EX_APPWINDOW, WS_EX_TOPMOST, WS_OVERLAPPED, WS_SYSMENU,
};
use windows::core::{PCWSTR, w};

use super::PromptEvent;
use super::fonts::{self, Fonts};
use super::layout::{self, CANCEL_ID, Controls, PIN_ID};

/// 窗口类名，带产品前缀以避免与系统类或其它程序冲突。
const WINDOW_CLASS: PCWSTR = w!("QingLink.Pairing.Prompt");

/// 标题栏文本。
const WINDOW_TITLE: PCWSTR = w!("QingLink 配对");

/// `RegisterClassW` 对重复注册返回的错误码。
const ERROR_CLASS_ALREADY_EXISTS: u32 = 1410;

/// 窗口过程需要的状态，随线程保存。
///
/// `WndProc` 没有闭包捕获，因此建窗时把需要的东西发布到这里；它只会在创建窗口的
/// 线程上运行，这份状态也就天然属于 main 线程。
struct WindowState {
    events: UnboundedSender<PromptEvent>,
    controls: Controls,
    /// 当前展示的请求；点「取消配对」时随事件回传给主循环。
    request_id: Option<String>,
}

thread_local! {
    static WINDOW_STATE: RefCell<Option<WindowState>> = const { RefCell::new(None) };
}

fn publish_state(state: WindowState) {
    WINDOW_STATE.with(|slot| *slot.borrow_mut() = Some(state));
}

fn clear_state() {
    WINDOW_STATE.with(|slot| *slot.borrow_mut() = None);
}

/// 一个配对提示窗口；默认隐藏，[`Self::show`] 时出现在用户面前。
pub(super) struct PromptWindow {
    hwnd: HWND,
    controls: Controls,
    fonts: Fonts,
}

impl std::fmt::Debug for PromptWindow {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PromptWindow")
            .field("request_id", &current_request_id())
            .finish_non_exhaustive()
    }
}

impl PromptWindow {
    /// 建窗（隐藏）并把点击事件接到 `events` 上。
    ///
    /// # Errors
    /// 窗口类注册、字体创建或窗口/子控件创建失败时返回错误。调用方应当降级为
    /// "只在管理界面显示配对码"，而不是让进程启动失败。
    pub(super) fn create(events: UnboundedSender<PromptEvent>) -> Result<Self> {
        // SAFETY: 无需释放的模块句柄；`WNDCLASSW` 与 `CreateWindowExW` 都只读取它。
        let instance =
            unsafe { GetModuleHandleW(None) }.context("cannot resolve the module handle")?;
        let instance = HINSTANCE::from(instance);
        register_class(instance)?;

        let dpi = layout::system_dpi();
        let style = WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_CLIPCHILDREN;
        // 置顶是刻意的：配对码只有 60 秒有效期，藏在别的窗口后面等于没有出现。
        let ex_style = WS_EX_TOPMOST | WS_EX_APPWINDOW;
        let (width, height) = layout::window_size(style, ex_style, dpi)?;
        let (x, y) = layout::centered_position(width, height);

        // SAFETY: 类已注册；所有参数都是本函数构造的有效值，父窗口为空表示顶层窗口。
        let hwnd = unsafe {
            CreateWindowExW(
                ex_style,
                WINDOW_CLASS,
                WINDOW_TITLE,
                style,
                x,
                y,
                width,
                height,
                None,
                None,
                Some(instance),
                None,
            )
        }
        .context("cannot create the pairing prompt window")?;

        // 从这里起，任何失败都要把已经建出来的资源收干净。
        let fonts = match fonts::create() {
            Ok(fonts) => fonts,
            Err(error) => {
                // SAFETY: 窗口刚创建、还没有子控件，属于本线程。
                let _ = unsafe { DestroyWindow(hwnd) };
                return Err(error);
            }
        };
        let controls = match layout::create_controls(hwnd, instance, &fonts, dpi) {
            Ok(controls) => controls,
            Err(error) => {
                destroy_resources(hwnd, fonts);
                return Err(error);
            }
        };

        publish_state(WindowState {
            events,
            controls,
            request_id: None,
        });
        Ok(Self {
            hwnd,
            controls,
            fonts,
        })
    }

    /// 展示一条配对请求，并把窗口带到前台。
    pub(super) fn show(&self, request_id: &str, device: &str, pin: &str, countdown: &str) {
        set_window_text(self.controls.device, device);
        set_window_text(self.controls.pin, pin);
        set_window_text(self.controls.countdown, countdown);
        self.set_cancel_enabled(true);
        set_request_id(Some(request_id));
        // SAFETY: 窗口属于本线程；`SetForegroundWindow` 的失败只是"没能抢到前台"，
        // 顶层窗口仍然可见，不需要额外处理。
        unsafe {
            let _ = ShowWindow(self.hwnd, SW_SHOWNORMAL);
            let _ = SetForegroundWindow(self.hwnd);
        }
    }

    /// 隐藏窗口；窗口本身保留，下一次请求直接复用。
    pub(super) fn hide(&self) {
        set_request_id(None);
        // SAFETY: 窗口属于本线程。
        unsafe {
            let _ = ShowWindow(self.hwnd, SW_HIDE);
        }
    }

    /// 只刷新倒计时行，避免整窗重设文本。
    pub(super) fn set_countdown(&self, label: &str) {
        set_window_text(self.controls.countdown, label);
    }

    /// 取消请求在途时禁用按钮，答复回来后再交还给用户。
    pub(super) fn set_cancel_enabled(&self, enabled: bool) {
        let label = if enabled {
            "取消配对"
        } else {
            "正在取消…"
        };
        set_window_text(self.controls.cancel, label);
        // SAFETY: 控件属于本线程；按钮被禁用后不会再发 `WM_COMMAND`。
        unsafe {
            let _ = EnableWindow(self.controls.cancel, enabled);
        }
    }
}

impl Drop for PromptWindow {
    fn drop(&mut self) {
        clear_state();
        destroy_resources(self.hwnd, self.fonts);
    }
}

/// 窗口过程：把用户动作变成事件，其余交给系统。
extern "system" fn window_procedure(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_COMMAND if (wparam.0 as u32) & 0xffff == CANCEL_ID as u32 => {
            on_cancel_clicked();
            LRESULT(0)
        }
        WM_CLOSE => {
            // 关掉窗口不等于取消配对：请求仍然有效，手机还能继续输入配对码。
            dispatch_dismissed();
            // SAFETY: 窗口属于本线程；隐藏而不是销毁，下一次请求直接复用。
            unsafe {
                let _ = ShowWindow(hwnd, SW_HIDE);
            }
            LRESULT(0)
        }
        WM_CTLCOLORSTATIC => static_colors(wparam, lparam),
        _ => {
            // SAFETY: 其余消息按契约交给默认过程。
            unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
        }
    }
}

/// 用户点了「取消配对」：禁用按钮并上报请求标识。
fn on_cancel_clicked() {
    WINDOW_STATE.with(|slot| {
        let state = slot.borrow();
        let Some(state) = state.as_ref() else {
            return;
        };
        let Some(request_id) = state.request_id.clone() else {
            return;
        };
        // 先禁用再上报：取消是一次网络往返，重复点击只应产生一个请求。
        // SAFETY: 控件属于本线程。
        unsafe {
            let _ = EnableWindow(state.controls.cancel, false);
        }
        set_window_text(state.controls.cancel, "正在取消…");
        let _ = state
            .events
            .send(PromptEvent::CancelRequested { request_id });
    });
}

/// 用户关掉了窗口；主循环据此压住这条请求，不再自动弹回。
fn dispatch_dismissed() {
    WINDOW_STATE.with(|slot| {
        let state = slot.borrow();
        let Some(state) = state.as_ref() else {
            return;
        };
        let Some(request_id) = state.request_id.clone() else {
            return;
        };
        let _ = state.events.send(PromptEvent::Dismissed { request_id });
    });
}

/// 静态文本的配色：配对码用强调色，其余用系统窗口文本色。
fn static_colors(wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // 这两个参数按 `WM_CTLCOLORSTATIC` 的契约就是 HDC 与控件句柄。
    let hdc = HDC(wparam.0 as *mut core::ffi::c_void);
    let control = HWND(lparam.0 as *mut core::ffi::c_void);
    // SAFETY: 控件句柄由系统给出且有效；GDI 调用只写调用方提供的 HDC。
    unsafe {
        let index = if GetDlgCtrlID(control) == PIN_ID {
            COLOR_HIGHLIGHT
        } else {
            COLOR_WINDOWTEXT
        };
        SetBkMode(hdc, TRANSPARENT);
        SetTextColor(hdc, COLORREF(GetSysColor(index)));
        // 返回窗口背景刷：静态控件用它与窗口融为一体。
        LRESULT(GetSysColorBrush(COLOR_BTNFACE).0 as isize)
    }
}

/// 当前展示的请求标识，供日志与 `Debug` 使用。
fn current_request_id() -> Option<String> {
    WINDOW_STATE.with(|slot| {
        slot.borrow()
            .as_ref()
            .and_then(|state| state.request_id.clone())
    })
}

/// 把当前请求标识写进线程局部状态。
fn set_request_id(request_id: Option<&str>) {
    WINDOW_STATE.with(|slot| {
        if let Some(state) = slot.borrow_mut().as_mut() {
            state.request_id = request_id.map(str::to_string);
        }
    });
}

fn set_window_text(window: HWND, value: &str) {
    let text = wide(value);
    // SAFETY: 缓冲区在整个调用期间存活；窗口属于本线程。
    let _ = unsafe { SetWindowTextW(window, PCWSTR(text.as_ptr())) };
}

/// 把 Rust 字符串转成调用期有效的 NUL 结尾宽字符缓冲区。
fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain([0]).collect()
}

fn register_class(instance: HINSTANCE) -> Result<()> {
    // SAFETY: 系统箭头光标由系统拥有，无需释放。
    let cursor = unsafe { LoadCursorW(None, IDC_ARROW) }.context("cannot load the arrow cursor")?;
    let class = WNDCLASSW {
        lpfnWndProc: Some(window_procedure),
        hInstance: instance,
        hCursor: cursor,
        hbrBackground: unsafe { GetSysColorBrush(COLOR_BTNFACE) },
        lpszClassName: WINDOW_CLASS,
        ..Default::default()
    };
    // SAFETY: `class` 完整初始化；重复注册是可接受的结果，由返回值报告。
    if unsafe { RegisterClassW(&class) } == 0 {
        // SAFETY: 读取上一次 `RegisterClassW` 的线程局部错误码。
        let error = unsafe { GetLastError() };
        anyhow::ensure!(
            error.0 == ERROR_CLASS_ALREADY_EXISTS,
            "cannot register the pairing prompt class (error {})",
            error.0
        );
    }
    Ok(())
}

/// 销毁窗口与字体，供建窗失败与 `Drop` 共用。
fn destroy_resources(hwnd: HWND, fonts: Fonts) {
    // SAFETY: 窗口由本模块创建且属于本线程；子控件随父窗口一起销毁。
    let _ = unsafe { DestroyWindow(hwnd) };
    fonts::destroy_fonts(fonts);
}
