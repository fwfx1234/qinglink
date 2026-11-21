//! 焦点事件缓存：会话代理里唯一碰 UI Automation 的地方。
//!
//! 代理处理管道请求是**单线程串行**的（`session_agent.rs` 的 `serve_pipe` 一次只接一个
//! 连接），所以焦点读取绝不能放在命令里：应用卡顿时 UIA 会阻塞几十到几百毫秒，后面的
//! 鼠标移动与按键注入全排在它后面，表现就是触控板一顿一顿。
//!
//! 这里完全不轮询：
//!
//! ```text
//! EVENT_OBJECT_FOCUS ──钩子回调只置位──▶ 工作线程读一次 UIA ──上升沿──▶ SetEvent 通知服务
//! ```
//!
//! 空闲时钩子线程阻塞在消息循环上、工作线程阻塞在 `WaitForSingleObject` 上，
//! 既不占 CPU，也不产生管道流量。

use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use std::thread;

use windows::Win32::Foundation::{HANDLE, WAIT_OBJECT_0};
use windows::Win32::System::Threading::{
    CreateEventW, EVENT_MODIFY_STATE, OpenEventW, SetEvent, WaitForSingleObject,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetMessageW, MSG, TranslateMessage,
};
use windows::core::PCWSTR;

use qinglink_windows::platform_windows;

/// 焦点当前是否落在可写输入框里，由工作线程在每次焦点变化后更新。
static ACTIVE: AtomicBool = AtomicBool::new(false);
/// 钩子线程 → 工作线程的唤醒事件句柄（本进程内，按 `isize` 存以跨线程传递）。
static WAKE: AtomicIsize = AtomicIsize::new(0);
/// 通知服务进程的跨会话事件句柄；打不开时留 0，下次焦点变化再试。
static NOTIFY: AtomicIsize = AtomicIsize::new(0);

/// 焦点当前是否在输入框里。只读内存，微秒级返回。
pub(super) fn active() -> bool {
    ACTIVE.load(Ordering::Acquire)
}

/// 启动钩子线程与工作线程；重复调用没有副作用。
pub(super) fn spawn() {
    if WAKE.load(Ordering::Acquire) != 0 {
        return;
    }
    let Ok(wake) = (unsafe { CreateEventW(None, false, false, PCWSTR::null()) }) else {
        tracing::error!("failed to create focus wake event");
        return;
    };
    WAKE.store(wake.0 as isize, Ordering::Release);
    let raw_wake = wake.0 as isize;

    if let Err(error) = thread::Builder::new()
        .name("focus-hook".into())
        .spawn(hook_thread_main)
    {
        tracing::error!(%error, "failed to spawn focus hook thread");
    }
    if let Err(error) = thread::Builder::new()
        .name("focus-worker".into())
        .spawn(move || worker_thread_main(raw_wake))
    {
        tracing::error!(%error, "failed to spawn focus worker thread");
    }
}

/// 装钩子并跑消息循环：`WINEVENT_OUTOFCONTEXT` 的回调通过这个线程的消息队列派发。
fn hook_thread_main() {
    let callback = Box::new(|| {
        let raw = WAKE.load(Ordering::Acquire);
        if raw != 0 {
            // 回调里只置位，不读 UIA——目标应用卡死时两边互相等待会把钩子线程拖住。
            unsafe {
                let _ = SetEvent(HANDLE(raw as *mut _));
            }
        }
    });
    if let Err(error) = platform_windows::set_focus_hook(callback) {
        tracing::error!(%error, "failed to install focus hook");
        return;
    }

    let mut message = MSG::default();
    loop {
        let result = unsafe { GetMessageW(&mut message, None, 0, 0) };
        if result.0 <= 0 {
            break;
        }
        unsafe {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
}

/// 等钩子置位，每次醒来读一次焦点；只在"没有焦点 → 有焦点"时通知服务。
fn worker_thread_main(raw_wake: isize) {
    let wake = HANDLE(raw_wake as *mut _);
    loop {
        if unsafe { WaitForSingleObject(wake, 1000) } != WAIT_OBJECT_0 {
            continue;
        }
        let current = platform_windows::has_focus_input();
        let previous = ACTIVE.swap(current, Ordering::AcqRel);
        if current && !previous {
            notify_service();
        }
    }
}

/// 置位跨会话事件，让服务进程醒来推送 `focus_input`。
fn notify_service() {
    let Some(handle) = notify_handle() else {
        return;
    };
    unsafe {
        let _ = SetEvent(handle);
    }
}

/// 打开（必要时缓存）服务创建的通知事件。
///
/// 服务还没起来时 `OpenEventW` 会失败，这里直接跳过：下一个焦点变化会再试一次。
fn notify_handle() -> Option<HANDLE> {
    let cached = NOTIFY.load(Ordering::Acquire);
    if cached != 0 {
        return Some(HANDLE(cached as *mut _));
    }
    let name = windows::core::HSTRING::from(platform_windows::FOCUS_NOTIFY_EVENT_NAME);
    let handle = unsafe { OpenEventW(EVENT_MODIFY_STATE, false, PCWSTR(name.as_ptr())) }.ok()?;
    NOTIFY.store(handle.0 as isize, Ordering::Release);
    Some(handle)
}
