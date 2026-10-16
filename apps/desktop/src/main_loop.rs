//! 主循环：HostCommand、托盘事件、配对弹窗、状态同步与消息泵。
//!
//! 循环体必须留在 main 线程：托盘窗口只允许同线程创建/销毁，Windows 消息队列
//! 属于这个线程，`SetThreadExecutionState` 也是 per-thread 状态。改动前请先读
//! `docs/architecture.md` 第 4 节；Web 层不能直接碰这些状态，只能走 `HostCommand`。

use std::sync::Arc;
use std::time::{Duration, Instant};

use qinglink_windows::single_instance::SingleInstance;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::admin_ui::{open_browser, web_url};
use crate::bootstrap::Desktop;
use crate::exit::{self, ExitRequest};
use crate::input_install;
use crate::pairing::{self, PairingPrompt, PromptEvent};
use crate::service::ServiceClient;
use crate::tray::{TrayAction, TrayController, TrayEvent, TrayRetryResult, TrayStatus};
use crate::web;
use crate::web::state::{HostCommand, ServerEvent, SharedSettings, WebState};

/// Main-loop period: drives the Windows message pump and status sync.
const TICK: Duration = Duration::from_millis(50);

/// How often the tray tooltip is compared against the service state.
const TOOLTIP_INTERVAL: Duration = Duration::from_millis(500);

/// 主循环借用的运行态。
struct Loop<'a> {
    host_rx: &'a mut UnboundedReceiver<HostCommand>,
    tray_rx: &'a mut UnboundedReceiver<TrayEvent>,
    tray: &'a mut Option<TrayController>,
    prompt: &'a mut Option<PairingPrompt>,
    prompt_rx: &'a mut UnboundedReceiver<PromptEvent>,
    instance: &'a SingleInstance,
    settings: &'a Arc<SharedSettings>,
    web_state: &'a Arc<WebState>,
    client: &'a ServiceClient,
    /// 是否已经打开过浏览器兜底，避免重复弹窗。
    browser_fallback_opened: &'a mut bool,
    /// 下一次同步托盘 tooltip 的时刻。
    next_tooltip: Instant,
    /// 主循环结束的原因；由 HostCommand 或托盘事件写入。
    exit_request: Option<ExitRequest>,
}

/// 跑主循环，返回退出原因。
///
/// 循环只在 HostCommand 或托盘事件要求退出时结束；退出前先通知浏览器并给在途
/// 响应留出时间，细节见 [`crate::exit`]。
pub fn run(desktop: &mut Desktop) -> ExitRequest {
    let Desktop {
        host_rx,
        tray_rx,
        tray,
        prompt,
        prompt_rx,
        instance,
        settings,
        web_state,
        client,
        browser_fallback_opened,
        ..
    } = desktop;
    let mut state = Loop {
        host_rx,
        tray_rx,
        tray,
        prompt,
        prompt_rx,
        instance,
        settings,
        web_state,
        client,
        browser_fallback_opened,
        next_tooltip: Instant::now(),
        exit_request: None,
    };

    loop {
        state.drain_host_commands();
        state.drain_tray_events();
        state.drain_prompt_events();

        if let Some(request) = state.exit_request {
            exit::prepare(request, state.web_state);
            return request;
        }

        state.sync_tray_status();
        state.sync_pairing_prompt();
        state.surface_activation();
        pump_messages();
        std::thread::sleep(TICK);
    }
}

impl Loop<'_> {
    /// Host commands arrive from HTTP handlers that need the main thread.
    fn drain_host_commands(&mut self) {
        while let Ok(command) = self.host_rx.try_recv() {
            match command {
                HostCommand::OpenUi { reply } => {
                    let result = open_browser().map(|()| serde_json::json!({ "url": web_url() }));
                    let _ = reply.send(result.map_err(|error| {
                        web::error::ApiError::internal(format!("无法打开管理界面：{error}"))
                    }));
                }
                HostCommand::SetKeepAwake { enabled, reply } => {
                    let reply_value = match qinglink_windows::power::set_keep_awake(enabled) {
                        Ok(()) => {
                            self.settings.set(
                                self.settings.autostart(),
                                enabled,
                                self.settings.debug_log(),
                            );
                            if let Some(tray) = self.tray.as_ref() {
                                tray.set_keep_awake(enabled);
                            }
                            tracing::info!(enabled, "保持唤醒状态已更新");
                            Ok(serde_json::json!({ "keep_awake": enabled }))
                        }
                        Err(error) => Err(web::error::ApiError::internal(format!(
                            "设置保持唤醒失败：{error}"
                        ))),
                    };
                    let _ = reply.send(reply_value);
                }
                HostCommand::Restart { reply } => {
                    let _ = reply.send(Ok(serde_json::json!({ "accepted": true })));
                    self.exit_request = Some(ExitRequest::Restart);
                }
                HostCommand::Exit { reply } => {
                    let _ = reply.send(Ok(serde_json::json!({ "accepted": true })));
                    self.exit_request = Some(ExitRequest::Quit);
                }
            }
        }
    }

    /// Tray actions and shell lifecycle events.
    fn drain_tray_events(&mut self) {
        while let Ok(event) = self.tray_rx.try_recv() {
            match event {
                TrayEvent::RetryRegistration => self.retry_registration(),
                TrayEvent::TaskbarCreated => self.taskbar_created(),
                TrayEvent::Action(action) => self.run_tray_action(action),
            }
        }
    }

    /// 重试注册；预算耗尽或仍被永久拒绝时改用浏览器兜底。
    fn retry_registration(&mut self) {
        let Some(controller) = self.tray.as_mut() else {
            return;
        };
        let result = controller.retry_registration(false);
        self.web_state.set_tray_report(controller.registration());
        // `Blocked` is reached when a shell that refused the icon
        // for an integrity mismatch is restarted and still does
        // not sit at our level; the fallback applies just as it
        // does when the retry budget runs out.
        if matches!(
            result,
            TrayRetryResult::Blocked | TrayRetryResult::Exhausted
        ) && !*self.browser_fallback_opened
        {
            *self.browser_fallback_opened = true;
            tracing::error!(?result, "托盘仍不可用（重试已停止），改为自动打开浏览器");
            if let Err(error) = open_browser() {
                tracing::error!(%error, "托盘不可用且浏览器兜底失败，请手动访问管理界面");
            }
        }
    }

    /// Explorer 重启后重新注册图标，并解除"永久拒绝"状态。
    fn taskbar_created(&mut self) {
        let Some(controller) = self.tray.as_mut() else {
            return;
        };
        let result = controller.retry_registration(true);
        self.web_state.set_tray_report(controller.registration());
        tracing::info!(?result, "检测到通知区域重建，重新注册托盘图标");
    }

    /// 配对提示弹窗：手机发起配对时出现，配对完成 / 超时 / 取消 / 锁屏 / 服务断开
    /// 时消失。窗口不自己判断结局，只跟随 `pending_pairing` 这一份事实。
    fn sync_pairing_prompt(&mut self) {
        let Some(prompt) = self.prompt.as_mut() else {
            return;
        };
        prompt.sync(self.client.pending_pairing());
    }

    /// 弹窗事件：取消请求要落到服务上，但只能在后台线程里等答复。
    fn drain_prompt_events(&mut self) {
        while let Ok(event) = self.prompt_rx.try_recv() {
            match event {
                PromptEvent::CancelRequested { request_id } => {
                    self.start_pairing_cancel(request_id)
                }
                PromptEvent::Dismissed { request_id } => {
                    if let Some(prompt) = self.prompt.as_mut() {
                        prompt.dismiss(&request_id);
                    }
                }
                PromptEvent::CancelFinished {
                    request_id,
                    outcome,
                } => self.finish_pairing_cancel(&request_id, outcome),
            }
        }
    }

    /// 把取消请求交给服务；答复由同一个事件通道送回主循环。
    ///
    /// 不能在主循环里等：Windows 消息泵就在这条线程上，一次网络往返会把托盘和弹窗
    /// 一起冻住。实际调用因此跑在专用线程上，主循环继续按 50ms 节拍转。
    fn start_pairing_cancel(&mut self, request_id: String) {
        let Some(prompt) = self.prompt.as_ref() else {
            return;
        };
        if let Err(error) = pairing::spawn_cancel(self.client.clone(), prompt.sender(), request_id)
        {
            tracing::error!(%error, "无法启动配对取消线程");
            if let Some(prompt) = self.prompt.as_mut() {
                prompt.cancel_failed();
            }
        }
    }

    /// 取消的结论：成功就收起弹窗，失败就把按钮交还给用户并给出原因。
    fn finish_pairing_cancel(&mut self, request_id: &str, outcome: Result<(), String>) {
        match outcome {
            Ok(()) => {
                if let Some(prompt) = self.prompt.as_mut() {
                    prompt.cancel_succeeded(request_id);
                }
                tracing::info!(request_id, "已取消配对请求");
                self.web_state.publish(ServerEvent::Toast {
                    level: "info",
                    message: "已取消配对请求".into(),
                });
            }
            Err(message) => {
                if let Some(prompt) = self.prompt.as_mut() {
                    prompt.cancel_failed();
                }
                tracing::warn!(request_id, %message, "取消配对请求失败");
                self.web_state.publish(ServerEvent::Toast {
                    level: "error",
                    message: format!("取消配对失败：{message}"),
                });
            }
        }
    }

    /// 托盘菜单动作。
    fn run_tray_action(&mut self, action: TrayAction) {
        match action {
            TrayAction::OpenUi => {
                if let Err(error) = open_browser() {
                    self.web_state.publish(ServerEvent::Toast {
                        level: "error",
                        message: format!("无法打开管理界面：{error}"),
                    });
                }
            }
            TrayAction::RestartService => restart_service(self.web_state),
            TrayAction::ToggleKeepAwake => {
                let enabled = !self.settings.keep_awake();
                match qinglink_windows::power::set_keep_awake(enabled) {
                    Ok(()) => {
                        self.settings.set(
                            self.settings.autostart(),
                            enabled,
                            self.settings.debug_log(),
                        );
                        if let Some(tray) = self.tray.as_ref() {
                            tray.set_keep_awake(enabled);
                        }
                        // The browser's settings page shares this state.
                        self.web_state
                            .publish(ServerEvent::Settings(Box::new(self.settings.view())));
                        // 开关跨重启保留；持久化丢给后台线程，main 线程不等同步 IO。
                        let preferences = self.web_state.preferences.clone();
                        std::thread::spawn(move || {
                            if let Err(error) = preferences.set_keep_awake(enabled) {
                                tracing::warn!(%error, enabled, "保存保持唤醒设置失败");
                            }
                        });
                        tracing::info!(enabled, "托盘切换保持唤醒");
                    }
                    Err(error) => tracing::error!(%error, "设置保持唤醒失败"),
                }
            }
            TrayAction::Exit => {
                tracing::info!("托盘请求退出");
                self.exit_request = Some(ExitRequest::Quit);
            }
        }
    }

    /// Status sync for the tray tooltip, rate-limited: native updates are
    /// comparatively expensive and the tooltip only needs to be current.
    fn sync_tray_status(&mut self) {
        if Instant::now() >= self.next_tooltip {
            if let Some(tray) = self.tray.as_ref() {
                let status = self.client.status();
                tray.set_status(&TrayStatus {
                    service_running: status.connected && status.network_ready,
                    service_error: status.last_error,
                });
            }
            self.next_tooltip = Instant::now() + TOOLTIP_INTERVAL;
        }
    }

    /// Another launch asked us to surface the interface.
    fn surface_activation(&self) {
        if self.instance.take_activation() {
            tracing::info!("其他实例请求激活，打开浏览器");
            if let Err(error) = open_browser() {
                self.web_state.publish(ServerEvent::Toast {
                    level: "error",
                    message: format!("无法打开管理界面：{error}"),
                });
            }
        }
    }
}

/// Restart the Windows service from the tray.
///
/// Elevation is required, so the request is handed to the secure-input
/// subsystem, which owns the UAC helper and the install-state reporting. The
/// result reaches the interface through install-state events.
fn restart_service(web_state: &Arc<WebState>) {
    tracing::info!("托盘请求重启服务");
    let shared = Arc::clone(&web_state.install_state);
    {
        let Ok(mut state) = shared.lock() else {
            return;
        };
        if state.busy {
            tracing::info!("已有服务操作在执行，忽略重复请求");
            return;
        }
        state.busy = true;
        state.result_message = Some("正在重启服务…".into());
    }
    web_state.publish_install_state();

    let web_state = Arc::clone(web_state);
    let shared = Arc::clone(&web_state.install_state);
    std::thread::spawn(move || {
        let result = input_install::restart_service_elevated();
        input_install::refresh(&shared);
        if let Ok(mut state) = shared.lock() {
            state.busy = false;
            state.result_message = Some(match result {
                Ok(message) => message,
                Err(error) => format!("重启服务失败：{error:#}"),
            });
        }
        let view = web_state.install_view();
        let _ = web_state
            .events
            .send(ServerEvent::InstallState(Box::new(view)));
    });
}

/// Drain the current thread's Windows message queue.
///
/// The tray icon lives on a hidden top-level window, so its callbacks (click,
/// menu command, shell restart) and registration timer only arrive while this
/// thread pumps messages. It must run on every iteration of the main loop.
fn pump_messages() {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::UI::WindowsAndMessaging::{
            DispatchMessageW, MSG, PM_REMOVE, PeekMessageW, TranslateMessage,
        };
        let mut message = MSG::default();
        // SAFETY: the queue belongs to this thread; `MSG` is fully initialized
        // and the retrieved message is dispatched before `message` is reused.
        unsafe {
            while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
    }
}
