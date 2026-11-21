//! 焦点事件钩子与跨会话通知。
//!
//! `EVENT_OBJECT_FOCUS` 用 `WINEVENT_OUTOFCONTEXT` 注册：回调在**注册钩子的那个线程**
//! 上派发，所以调用方必须在该线程上跑消息循环。回调里只允许做极短的事（置位一个事件），
//! 不要把 UI Automation 放进来——目标应用卡死时，回调和它互相等待会把钩子线程拖住。

use std::sync::{Arc, Mutex};

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Accessibility::{HWINEVENTHOOK, SetWinEventHook, UnhookWinEvent};
use windows::Win32::UI::WindowsAndMessaging::{EVENT_OBJECT_FOCUS, OBJID_CLIENT};

/// 跨会话的焦点通知事件名：代理 `SetEvent`，服务 `WaitForSingleObject`。
///
/// 每个会话各有一份 `\BaseNamedObjects`，跨会话必须放在 `Global\` 命名空间里，
/// 所以这里带 `Global\` 前缀。事件是**自动复位**的：置位后唤醒一个等待者就归零，
/// 多个通知挤在一起只算一次，正好符合"醒来重新确认一次焦点"的语义。
pub const FOCUS_NOTIFY_EVENT_NAME: &str = r"Global\QingLinkFocusInput";

type FocusHookCallback = Arc<dyn Fn() + Send + Sync>;

static FOCUS_HOOK_STATE: Mutex<Option<FocusHookCallback>> = Mutex::new(None);

/// 注册 `EVENT_OBJECT_FOCUS` 钩子；回调在注册线程上派发，调用方需要跑消息循环。
///
/// 只上报"控件区对象获得焦点"（`OBJID_CLIENT`）的事件：菜单、标题栏之类系统对象
/// 拿到焦点时不可能落在输入框里，过滤掉可以少一大半无用唤醒。
///
/// # Errors
///
/// `SetWinEventHook` 返回无效句柄时返回错误，并清理已经登记的回调。
pub fn set_focus_hook(callback: Box<dyn Fn() + Send + Sync>) -> anyhow::Result<HWINEVENTHOOK> {
    if let Ok(mut state) = FOCUS_HOOK_STATE.lock() {
        *state = Some(Arc::from(callback));
    }
    let hook = unsafe {
        SetWinEventHook(
            EVENT_OBJECT_FOCUS,
            EVENT_OBJECT_FOCUS,
            None,
            Some(focus_event_proc),
            0,
            0,
            0, // WINEVENT_OUTOFCONTEXT
        )
    };
    if hook.is_invalid() {
        if let Ok(mut state) = FOCUS_HOOK_STATE.lock() {
            *state = None;
        }
        anyhow::bail!("SetWinEventHook(EVENT_OBJECT_FOCUS) failed");
    }
    Ok(hook)
}

/// 注销焦点钩子并清理回调状态。
pub fn unhook_focus(hook: HWINEVENTHOOK) {
    unsafe {
        let _ = UnhookWinEvent(hook);
    }
    if let Ok(mut state) = FOCUS_HOOK_STATE.lock() {
        *state = None;
    }
}

unsafe extern "system" fn focus_event_proc(
    _hook: HWINEVENTHOOK,
    event: u32,
    _hwnd: HWND,
    id_object: i32,
    _id_child: i32,
    _event_thread: u32,
    _event_time: u32,
) {
    if event != EVENT_OBJECT_FOCUS || id_object != OBJID_CLIENT.0 {
        return;
    }
    // 回调要立刻返回：这里只克隆一次回调指针并调用，真正的 UIA 读取在等待方线程上。
    let callback = FOCUS_HOOK_STATE.lock().ok().and_then(|state| state.clone());
    if let Some(callback) = callback {
        callback();
    }
}
