//! 鼠标按键状态监控：把 PC 侧真实的左右中键状态推给手机端。
//!
//! 触控板的长按拖拽需要知道"PC 上鼠标此刻是否已经按住"：按住时不该再补一次按下，
//! 松手时也要知道自己是否需要补一次抬起。异步键状态同时反映物理按压与 `SendInput`
//! 注入，所以这里是唯一事实来源，客户端不再自己记账。

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::thread;
use std::time::Duration;

use crate::platform;
use crate::protocol::requests::WsEvent;
use crate::server::EventSender;

/// 轮询间隔：250ms 足够让"按住"在手机上出现得及时，又不会让会话管道的请求过密。
const POLL_INTERVAL: Duration = Duration::from_millis(250);

/// 即使状态没变也每隔这么多轮补发一次，让后连上来的客户端在一个周期内对齐。
const RESEND_EVERY: u32 = 40;

/// 鼠标按键状态监控器。
#[derive(Debug)]
pub struct InputStateMonitor {
    /// 事件发送端。
    event_tx: Option<EventSender>,
    /// 线程运行标志。
    running: Arc<AtomicBool>,
    /// 监控线程句柄。
    watcher: Option<thread::JoinHandle<()>>,
}

impl Default for InputStateMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl InputStateMonitor {
    /// 创建监控器；此时尚未启动后台线程。
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

    /// 启动监控线程；重复调用不会创建新线程。
    pub fn start(&mut self) {
        if self.running.load(Ordering::Relaxed) {
            return;
        }
        self.running.store(true, Ordering::Relaxed);
        let running = Arc::clone(&self.running);
        let event_tx = self.event_tx.clone();
        self.watcher = match thread::Builder::new()
            .name("input-state-monitor".into())
            .spawn(move || monitor_thread_main(&running, &event_tx))
        {
            Ok(watcher) => Some(watcher),
            Err(error) => {
                self.running.store(false, Ordering::Relaxed);
                tracing::error!(%error, "failed to spawn input state monitor");
                None
            }
        };
    }
}

/// 监控线程主体：只在状态变化（或周期性补发）时广播事件。
fn monitor_thread_main(running: &Arc<AtomicBool>, event_tx: &Option<EventSender>) {
    let mut last = platform::mouse_buttons_state();
    let mut ticks = 0_u32;
    while running.load(Ordering::Relaxed) {
        let current = platform::mouse_buttons_state();
        ticks = ticks.wrapping_add(1);
        if current != last || ticks % RESEND_EVERY == 0 {
            last = current;
            if let Some(tx) = event_tx {
                let _ = tx.send(WsEvent::InputState {
                    left: current.left,
                    right: current.right,
                    middle: current.middle,
                });
            }
        }
        thread::sleep(POLL_INTERVAL);
    }
}
