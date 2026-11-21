//! 焦点事件中继：会话代理的钩子线程置位跨会话事件，这里阻塞等待并广播给客户端。
//!
//! **不轮询。** 空闲时线程阻塞在 `WaitForSingleObject` 上：既不占 CPU，也不产生管道流量，
//! 焦点变化到手机收到只差一次唤醒加一次广播。
//!
//! 事件由代理创建侧置位、这里创建并等待（`Global\QingLinkFocusInput`，自动复位）：
//! 代理在"焦点第一次落进可写输入框"时才置位一次，所以这里收到就该推，不需要再去重。

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::thread;

use crate::protocol::requests::WsEvent;
use crate::server::EventSender;

/// 等待超时：只为周期性检查 `running`，让进程退出时线程能收尾。
#[cfg(target_os = "windows")]
const WAIT_SLICE_MILLIS: u32 = 500;

/// 焦点事件中继器。
#[derive(Debug)]
pub struct FocusMonitor {
    /// 事件发送端。
    event_tx: Option<EventSender>,
    /// 线程运行标志。
    running: Arc<AtomicBool>,
    /// 中继线程句柄。
    watcher: Option<thread::JoinHandle<()>>,
}

impl Default for FocusMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl FocusMonitor {
    /// 创建中继器；此时尚未启动后台线程。
    pub fn new() -> Self {
        Self {
            event_tx: None,
            running: Arc::new(AtomicBool::new(false)),
            watcher: None,
        }
    }

    /// 设置事件发送端。
    pub fn set_event_sender(&mut self, tx: EventSender) {
        self.event_tx = Some(tx);
    }

    /// 启动中继线程；重复调用不会创建新线程。
    pub fn start(&mut self) {
        if self.running.load(Ordering::Relaxed) {
            return;
        }
        self.running.store(true, Ordering::Relaxed);
        let running = Arc::clone(&self.running);
        let event_tx = self.event_tx.clone();
        self.watcher = match thread::Builder::new()
            .name("focus-input-monitor".into())
            .spawn(move || monitor_thread_main(&running, &event_tx))
        {
            Ok(watcher) => Some(watcher),
            Err(error) => {
                self.running.store(false, Ordering::Relaxed);
                tracing::error!(%error, "failed to spawn focus input monitor");
                None
            }
        };
    }
}

/// 等待代理的置位，每醒来一次广播一条 `focus_input`。
fn monitor_thread_main(running: &Arc<AtomicBool>, event_tx: &Option<EventSender>) {
    #[cfg(target_os = "windows")]
    {
        wait_for_notifications(running, event_tx);
    }
    #[cfg(not(target_os = "windows"))]
    {
        // 非 Windows 没有 UIA 焦点概念，线程起来后原地等到退出即可。
        while running.load(Ordering::Relaxed) {
            thread::sleep(std::time::Duration::from_millis(WAIT_SLICE_MILLIS.into()));
        }
        let _ = event_tx;
    }
}

/// Windows 实现：创建跨会话事件并阻塞等待。
#[cfg(target_os = "windows")]
fn wait_for_notifications(running: &Arc<AtomicBool>, event_tx: &Option<EventSender>) {
    use windows::Win32::Foundation::{CloseHandle, WAIT_OBJECT_0, WAIT_TIMEOUT};
    use windows::Win32::System::Threading::{CreateEventW, WaitForSingleObject};
    use windows::core::PCWSTR;

    // 自动复位事件：置位唤醒一次就归零，多次通知合并成一次"重新确认焦点"。
    let name =
        windows::core::HSTRING::from(qinglink_windows::platform_windows::FOCUS_NOTIFY_EVENT_NAME);
    let Ok(handle) = (unsafe { CreateEventW(None, false, false, PCWSTR(name.as_ptr())) }) else {
        tracing::error!("failed to create focus notify event");
        return;
    };

    while running.load(Ordering::Relaxed) {
        let result = unsafe { WaitForSingleObject(handle, WAIT_SLICE_MILLIS) };
        if result == WAIT_OBJECT_0 {
            if let Some(tx) = event_tx {
                let _ = tx.send(WsEvent::FocusInput);
            }
        } else if result != WAIT_TIMEOUT {
            tracing::warn!("focus notify wait failed");
            break;
        }
    }

    unsafe {
        let _ = CloseHandle(handle);
    }
}
