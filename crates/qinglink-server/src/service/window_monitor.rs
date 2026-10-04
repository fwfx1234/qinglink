//! 窗口监控：用 WinEvent 钩子（必要时回退轮询）跟踪前台窗口与游戏模式。

use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use std::thread;
use std::time::Duration;

use crate::platform;
use crate::protocol::requests::WsEvent;
use crate::server::EventSender;

/// 窗口监控器：后台线程感知前台窗口变化，并维护游戏模式标记。
#[derive(Debug)]
pub struct WindowMonitor {
    /// 当前是否处于全屏游戏模式。
    is_gaming: Arc<AtomicBool>,
    /// 监控线程句柄，析构时用于等待线程退出。
    watcher: Option<thread::JoinHandle<()>>,
    /// 事件发送端，用于推送 `ForegroundChanged`。
    event_tx: Option<EventSender>,
    /// 线程运行标志，置为 false 即请求退出。
    running: Arc<AtomicBool>,
}

impl Default for WindowMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowMonitor {
    /// 创建监控器；此时尚未启动后台线程。
    pub fn new() -> Self {
        Self {
            is_gaming: Arc::new(AtomicBool::new(false)),
            watcher: None,
            event_tx: None,
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    /// 设置用于推送前台窗口变化事件的发送端。
    pub fn set_event_sender(&mut self, tx: EventSender) {
        self.event_tx = Some(tx);
    }

    /// 启动监控线程；已启动时重复调用不会创建新线程。
    pub fn start(&mut self) {
        if self.running.load(Ordering::Relaxed) {
            return;
        }
        self.running.store(true, Ordering::Relaxed);

        let is_gaming = Arc::clone(&self.is_gaming);
        let event_tx = self.event_tx.clone();
        let running = Arc::clone(&self.running);

        self.watcher = match thread::Builder::new()
            .name("window-monitor".into())
            .spawn(move || monitor_thread_main(&running, &is_gaming, &event_tx))
        {
            Ok(watcher) => Some(watcher),
            Err(error) => {
                self.running.store(false, Ordering::Relaxed);
                tracing::error!(%error, "failed to spawn window monitor");
                None
            }
        };
    }

    /// 当前前台是否为全屏游戏（游戏模式下会降低扫描频率）。
    pub fn is_gaming(&self) -> bool {
        self.is_gaming.load(Ordering::Relaxed)
    }
}

/// 监控线程主体：优先装 WinEvent 钩子，Session 0、建事件/装钩失败或进入游戏模式时回退轮询。
fn monitor_thread_main(
    running: &Arc<AtomicBool>,
    is_gaming: &Arc<AtomicBool>,
    event_tx: &Option<EventSender>,
) {
    lower_current_thread_priority();

    // WinEvent hooks are scoped to the calling window station.
    // The network service runs in Session 0, so its hook would
    // never observe the interactive user's foreground changes.
    // Poll the session agent's window catalog in that case.
    if qinglink_windows::secure_desktop::running_in_session_zero() {
        poll_interactive_fallback(running, is_gaming, event_tx);
        return;
    }

    let shutdown_event = match create_shutdown_event() {
        Some(handle) => handle,
        None => {
            tracing::warn!("[窗口监控] CreateEventW 失败，回退到轮询模式");
            poll_fallback(running, is_gaming, event_tx);
            return;
        }
    };

    // 用于接收钩子回调数据的共享状态
    let hook_hwnd = Arc::new(AtomicUsize::new(usize::MAX));

    // 安装事件钩子
    let hook = platform::set_foreground_hook(foreground_hook_callback(
        running,
        is_gaming,
        Arc::clone(&hook_hwnd),
    ));
    let hook = match hook {
        Ok(hook) => {
            tracing::info!("[窗口监控] 事件钩子已安装");
            hook
        }
        Err(e) => {
            tracing::warn!("[窗口监控] 事件钩子安装失败: {}，回退到轮询模式", e);
            close_handle(shutdown_event);
            poll_fallback(running, is_gaming, event_tx);
            return;
        }
    };

    run_hook_loop(running, is_gaming, event_tx, &hook_hwnd, shutdown_event);

    platform::unhook_foreground(hook);
    close_handle(shutdown_event);
    tracing::info!("[窗口监控] 事件钩子已卸载");

    // 回退到轮询模式（游戏模式或钩子卸载后）
    poll_fallback(running, is_gaming, event_tx);
}

/// 把监控线程优先级降到最低，避免与前台应用争抢 CPU。
fn lower_current_thread_priority() {
    unsafe {
        let handle = windows::Win32::System::Threading::GetCurrentThread();
        let _ = windows::Win32::System::Threading::SetThreadPriority(
            handle,
            windows::Win32::System::Threading::THREAD_PRIORITY_LOWEST,
        );
    }
}

/// 创建手动重置的 shutdown 事件；创建失败时返回 `None`，由调用方回退轮询。
fn create_shutdown_event() -> Option<windows::Win32::Foundation::HANDLE> {
    // 创建 shutdown 事件
    use windows::core::PCWSTR;
    let shutdown_event = unsafe {
        windows::Win32::System::Threading::CreateEventW(
            None,
            true,  // manual reset
            false, // initial state: nonsignaled
            PCWSTR::null(),
        )
    };
    match shutdown_event {
        Ok(handle) if !handle.is_invalid() => Some(handle),
        _ => None,
    }
}

/// 构造钩子回调：更新游戏模式标记，并记录非游戏态的前台窗口句柄。
fn foreground_hook_callback(
    running: &Arc<AtomicBool>,
    is_gaming: &Arc<AtomicBool>,
    hook_hwnd: Arc<AtomicUsize>,
) -> Box<dyn Fn(isize) + Send + Sync> {
    let running_hook = Arc::clone(running);
    let is_gaming_hook = Arc::clone(is_gaming);

    Box::new(move |hwnd| {
        if !running_hook.load(Ordering::Relaxed) {
            return;
        }
        let gaming = platform::is_foreground_fullscreen();
        is_gaming_hook.store(gaming, Ordering::Relaxed);
        if gaming {
            return;
        }
        hook_hwnd.store(hwnd as usize, Ordering::Relaxed);
    })
}

/// 主循环：使用 MsgWaitForMultipleObjects 等待事件或消息，直到退出或进入游戏模式。
fn run_hook_loop(
    running: &Arc<AtomicBool>,
    is_gaming: &Arc<AtomicBool>,
    event_tx: &Option<EventSender>,
    hook_hwnd: &AtomicUsize,
    shutdown_event: windows::Win32::Foundation::HANDLE,
) {
    while running.load(Ordering::Relaxed) {
        if is_gaming.load(Ordering::Relaxed) {
            break;
        }

        // 检查是否有窗口变化事件
        let hwnd = hook_hwnd.load(Ordering::Relaxed);
        if hwnd != usize::MAX {
            hook_hwnd.store(usize::MAX, Ordering::Relaxed);
            send_foreground_changed(event_tx, hwnd);
        }

        if pump_messages_until_shutdown(shutdown_event) {
            running.store(false, Ordering::Relaxed);
            break;
        }
    }
}

/// 等待 shutdown 事件或窗口消息（200ms 超时）。
///
/// 返回 `true` 表示 shutdown 事件已触发。
fn pump_messages_until_shutdown(shutdown_event: windows::Win32::Foundation::HANDLE) -> bool {
    unsafe {
        let result = windows::Win32::UI::WindowsAndMessaging::MsgWaitForMultipleObjects(
            Some(&[shutdown_event]),
            false, // wait for any
            200,   // timeout ms
            windows::Win32::UI::WindowsAndMessaging::QS_ALLEVENTS,
        );
        // WAIT_OBJECT_0 表示 shutdown 事件被触发
        if result == windows::Win32::Foundation::WAIT_OBJECT_0 {
            return true;
        }
        // 如果有消息，PeekMessage 分派它们（让钩子回调有机会执行）
        let mut msg: windows::Win32::UI::WindowsAndMessaging::MSG = std::mem::zeroed();
        while windows::Win32::UI::WindowsAndMessaging::PeekMessageW(
            &mut msg,
            None,
            0,
            0,
            windows::Win32::UI::WindowsAndMessaging::PM_REMOVE,
        )
        .as_bool()
        {
            let _ = windows::Win32::UI::WindowsAndMessaging::TranslateMessage(&msg);
            let _ = windows::Win32::UI::WindowsAndMessaging::DispatchMessageW(&msg);
        }
        false
    }
}

/// 读取前台窗口信息并推送 `ForegroundChanged`；`hwnd` 取钩子回调记录的句柄。
fn send_foreground_changed(event_tx: &Option<EventSender>, hwnd: usize) {
    let Some(tx) = event_tx.as_ref() else {
        return;
    };
    let Ok(info) = platform::get_foreground_window_info() else {
        return;
    };
    let _ = tx.send(WsEvent::ForegroundChanged {
        data: crate::protocol::requests::ForegroundInfo {
            pid: info.pid,
            hwnd,
            title: info.title,
            path: info.path,
        },
    });
}

/// 关闭 Win32 句柄，忽略失败结果。
fn close_handle(handle: windows::Win32::Foundation::HANDLE) {
    unsafe {
        let _ = windows::Win32::Foundation::CloseHandle(handle);
    }
}

fn poll_fallback(
    running: &Arc<AtomicBool>,
    is_gaming: &Arc<AtomicBool>,
    event_tx: &Option<EventSender>,
) {
    let mut last_hwnd: Option<usize> = None;
    let mut debounce: Option<std::time::Instant> = None;

    while running.load(Ordering::Relaxed) {
        let interval = if is_gaming.load(Ordering::Relaxed) {
            Duration::from_secs(5)
        } else {
            Duration::from_millis(500)
        };

        thread::sleep(interval);

        // Full-screen foreground work is treated conservatively as game mode.
        // It clears as soon as that workload leaves the foreground, preventing
        // the old one-way `set_gaming(true)` state from blocking scans forever.
        is_gaming.store(platform::is_foreground_fullscreen(), Ordering::Relaxed);

        if let Ok(info) = platform::get_foreground_window_info() {
            let current_hwnd = info.hwnd;
            if last_hwnd != Some(current_hwnd) {
                last_hwnd = Some(current_hwnd);
                debounce = Some(std::time::Instant::now());
            } else if let Some(t) = debounce
                && t.elapsed() > Duration::from_millis(200)
            {
                if let Some(tx) = event_tx.as_ref() {
                    let _ = tx.send(WsEvent::ForegroundChanged {
                        data: crate::protocol::requests::ForegroundInfo {
                            pid: info.pid,
                            hwnd: info.hwnd,
                            title: info.title,
                            path: info.path,
                        },
                    });
                }
                debounce = None;
            }
        }
    }
}

fn poll_interactive_fallback(
    running: &Arc<AtomicBool>,
    is_gaming: &Arc<AtomicBool>,
    event_tx: &Option<EventSender>,
) {
    let mut last_hwnd: Option<usize> = None;
    let mut debounce: Option<std::time::Instant> = None;

    while running.load(Ordering::Relaxed) {
        thread::sleep(Duration::from_millis(300));
        let foreground = crate::platform::enum_windows_list()
            .into_iter()
            .find(|window| window.is_foreground);
        let Some(window) = foreground else {
            is_gaming.store(false, Ordering::Relaxed);
            continue;
        };

        if last_hwnd != Some(window.id) {
            last_hwnd = Some(window.id);
            debounce = Some(std::time::Instant::now());
        } else if let Some(t) = debounce
            && t.elapsed() > Duration::from_millis(200)
        {
            if let Some(tx) = event_tx.as_ref() {
                let _ = tx.send(WsEvent::ForegroundChanged {
                    data: crate::protocol::requests::ForegroundInfo {
                        pid: window.pid,
                        hwnd: window.id,
                        title: window.title,
                        path: window.exe_path,
                    },
                });
            }
            debounce = None;
        }
    }
}

impl Drop for WindowMonitor {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(handle) = self.watcher.take() {
            let _ = handle.join();
        }
    }
}
