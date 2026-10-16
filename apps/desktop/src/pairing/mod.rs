//! 配对提示弹窗：手机发起配对时，在桌面端弹出一个独立窗口显示配对码。
//!
//! 为什么需要它：管理界面跑在浏览器里，而桌面端是登录后常驻托盘的程序。浏览器没
//! 打开时，配对码没有任何地方可看，配对也就无法完成。这个窗口是该进程里唯一会主动
//! 出现在用户面前的窗口。
//!
//! 状态的**唯一来源是服务快照**（`ServiceClient::pending_pairing`）：手机完成配对、
//! 请求超时、请求被取消、桌面被锁屏、服务断开，都会让快照里不再有待确认请求，窗口
//! 于是在同一个轮询里收敛为隐藏——不必为每种结局各写一条关闭路径。
//!
//! 模块分工：
//!
//! * 本文件：[`PairingPrompt`] 的状态机（何时显示、何时收起）与事件类型；
//! * [`window`]：Win32 窗口本体与窗口过程；
//! * [`layout`]：窗口几何、DPI 换算与子控件创建；
//! * [`fonts`]：由系统消息字体派生的两套字体；
//! * [`cancel`]：在后台线程里把取消请求送到服务；
//! * [`text`]：弹窗文案。

mod cancel;
mod fonts;
mod layout;
mod text;
mod window;

use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

use qinglink_protocol::service_ipc::PendingPairing;

use self::window::PromptWindow;
pub(crate) use cancel::spawn as spawn_cancel;

/// 窗口过程交给主循环的事件。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PromptEvent {
    /// 用户点了「取消配对」；主循环负责把它送到服务。
    CancelRequested {
        /// 要取消的配对请求标识。
        request_id: String,
    },
    /// 用户关掉了窗口：只收起窗口，配对请求仍然有效。
    Dismissed {
        /// 被收起的请求标识。
        request_id: String,
    },
    /// 后台取消线程的结论；`Err` 里是给用户看的原因。
    CancelFinished {
        /// 这次取消针对的请求标识。
        request_id: String,
        /// 成功为 `Ok(())`，失败带服务返回的原因。
        outcome: Result<(), String>,
    },
}

/// 配对提示弹窗的控制器。
///
/// 只保存"当前展示了什么"，不持有服务客户端：取消请求由主循环发起，窗口本身不碰
/// 任何 IO。所有方法都必须在 main 线程调用。
pub struct PairingPrompt {
    window: PromptWindow,
    events: UnboundedSender<PromptEvent>,
    /// 正在展示的请求；`None` 表示窗口当前是隐藏的。
    shown: Option<PendingPairing>,
    /// 被用户关掉或已经请求取消的请求：在它真正从快照里消失之前不再自动弹回。
    ///
    /// 取消是异步的，而快照最多滞后 500ms；没有这个压住标记，窗口会在取消成功后
    /// 被那条过期快照重新弹出来。
    suppressed: Option<String>,
    /// 上一次写进窗口的剩余秒数，避免每 50ms 重设一次文本。
    last_remaining: Option<i64>,
}

/// 只打印请求标识：`PendingPairing` 里有配对码，绝不能进日志。
impl std::fmt::Debug for PairingPrompt {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PairingPrompt")
            .field(
                "shown",
                &self.shown.as_ref().map(|shown| shown.request_id.as_str()),
            )
            .field("suppressed", &self.suppressed)
            .finish_non_exhaustive()
    }
}

impl PairingPrompt {
    /// 创建隐藏的提示窗口并返回它的事件通道。
    ///
    /// # Errors
    /// 窗口类注册、字体或窗口创建失败时返回错误。调用方应当降级为"配对码只在管理
    /// 界面里显示"，而不是让启动失败。
    pub fn create() -> anyhow::Result<(Self, UnboundedReceiver<PromptEvent>)> {
        let (events, receiver) = unbounded_channel();
        let window = PromptWindow::create(events.clone())?;
        Ok((
            Self {
                window,
                events,
                shown: None,
                suppressed: None,
                last_remaining: None,
            },
            receiver,
        ))
    }

    /// 事件通道的克隆，供后台取消线程回传结论。
    pub fn sender(&self) -> UnboundedSender<PromptEvent> {
        self.events.clone()
    }

    /// 按服务快照收敛窗口。
    ///
    /// `pending` 为 `None` 覆盖了全部结束情形：配对成功、超时、被取消、锁屏、服务
    /// 断开。同一条请求重复进来时只刷新倒计时。
    pub fn sync(&mut self, pending: Option<PendingPairing>) {
        let Some(request) = pending else {
            self.suppressed = None;
            if self.shown.take().is_some() {
                self.last_remaining = None;
                self.window.hide();
            }
            return;
        };

        if self.suppressed.as_deref() == Some(request.request_id.as_str()) {
            return;
        }

        let remaining = text::remaining_seconds(request.expires_at_unix);
        let is_new = self
            .shown
            .as_ref()
            .map(|shown| shown.request_id != request.request_id)
            .unwrap_or(true);
        if is_new {
            self.window.show(
                &request.request_id,
                &text::device_line(&request.device_name),
                &request.pin,
                &text::countdown_label(remaining),
            );
            self.last_remaining = Some(remaining);
        } else if self.last_remaining != Some(remaining) {
            self.window.set_countdown(&text::countdown_label(remaining));
            self.last_remaining = Some(remaining);
        }
        self.shown = Some(request);
    }

    /// 用户关掉了窗口：同一个请求消失前不再自动弹出。
    pub fn dismiss(&mut self, request_id: &str) {
        if self.shown.as_ref().map(|shown| shown.request_id.as_str()) == Some(request_id) {
            self.suppressed = Some(request_id.to_string());
            self.shown = None;
            self.last_remaining = None;
            self.window.hide();
        }
    }

    /// 取消成功：收起窗口，并压住迟到快照里的同一条请求。
    pub fn cancel_succeeded(&mut self, request_id: &str) {
        self.suppressed = Some(request_id.to_string());
        self.shown = None;
        self.last_remaining = None;
        self.window.hide();
    }

    /// 取消失败：把按钮还给用户，原因由调用方提示。
    pub fn cancel_failed(&mut self) {
        self.window.set_cancel_enabled(true);
    }
}
