use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use gpui::prelude::FluentBuilder;
use gpui::{
    AnyElement, AppContext, Bounds, Context, FontWeight, InteractiveElement, IntoElement,
    ParentElement, Render, StatefulInteractiveElement, Styled, Window, WindowBackgroundAppearance,
    WindowBounds, WindowDecorations, WindowKind, WindowOptions, div, px, size,
};
use qinglink_server::{AppState, server::pairing::PairingNotification};
use qinglink_ui::theme;
use tokio::sync::watch;

use crate::{
    input_install::shared_state as input_install_state,
    logging::LogLevelSwitch,
    service_runtime::{DesiredServiceState, ServiceStatus},
    tray::{TrayAction, TrayController},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Page {
    Overview,
    Devices,
    Catalog,
    Settings,
}

pub struct RemoteDashboard {
    pub(crate) app_state: AppState,
    pub(crate) pairing: Arc<Mutex<Option<PairingNotification>>>,
    pub(crate) service_status: Arc<Mutex<ServiceStatus>>,
    pub(crate) desired_service: watch::Sender<DesiredServiceState>,
    pub(crate) tray: Option<TrayController>,
    pub(crate) input_install: crate::input_install::SharedInputInstallState,
    pub(crate) paused: bool,
    pub(crate) autostart: bool,
    pub(crate) debug_logging: bool,
    log_switch: LogLevelSwitch,
    page: Page,
    last_activated_request: Option<String>,
    pending_pairing_request: Option<PairingNotification>,
    message: Option<String>,
    message_timeout: u32,
    exiting: Arc<std::sync::atomic::AtomicBool>,
}

impl RemoteDashboard {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        app_state: AppState,
        pairing: Arc<Mutex<Option<PairingNotification>>>,
        service_status: Arc<Mutex<ServiceStatus>>,
        desired_service: watch::Sender<DesiredServiceState>,
        tray: Option<TrayController>,
        autostart: bool,
        show_on_start: bool,
        log_switch: LogLevelSwitch,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Self {
        super::helpers::store_main_window_handle(window);
        let exiting = Arc::new(AtomicBool::new(false));
        let exiting_close = Arc::clone(&exiting);
        if show_on_start {
            super::helpers::show_window(window);
        } else {
            super::helpers::hide_window(window);
        }
        window.on_window_should_close(cx, move |window, _cx| {
            if exiting_close.load(Ordering::Acquire) {
                return true;
            }
            super::helpers::hide_window(window);
            false
        });
        cx.spawn(async move |view, async_cx| {
            loop {
                async_cx
                    .background_executor()
                    .timer(Duration::from_millis(250))
                    .await;
                if view
                    .update(async_cx, |view, cx| {
                        view.flush_pending_pairing(cx);
                        if view.message_timeout > 0 {
                            view.message_timeout -= 1;
                            if view.message_timeout == 0 {
                                view.message = None;
                            }
                        }
                        cx.notify();
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
        let paused = app_state
            .app_db
            .lock()
            .ok()
            .and_then(|database| database.get_setting("service.paused").ok().flatten())
            .is_some_and(|value| value == "true");
        Self {
            app_state,
            pairing,
            service_status,
            desired_service,
            tray,
            debug_logging: log_switch.is_debug(),
            log_switch,
            page: Page::Overview,
            paused,
            autostart,
            last_activated_request: None,
            pending_pairing_request: None,
            message: None,
            message_timeout: 0,
            input_install: input_install_state(),
            exiting,
        }
    }

    pub(crate) fn show_message(&mut self, message: String) {
        self.message = Some(message);
        self.message_timeout = 12;
    }

    fn handle_tray(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let actions: Vec<TrayAction> = self
            .tray
            .as_ref()
            .map(|tray| {
                let mut actions = Vec::new();
                while let Some(action) = tray.next_action() {
                    actions.push(action);
                }
                actions
            })
            .unwrap_or_default();
        for action in actions {
            match action {
                TrayAction::Open => {
                    cx.activate(true);
                    super::helpers::show_window(window);
                }
                TrayAction::TogglePause => self.toggle_service(cx),
                TrayAction::ToggleAutostart => self.toggle_autostart(cx),
                TrayAction::Restart => self.restart_desktop(cx),
                TrayAction::Exit => {
                    self.exiting.store(true, Ordering::Release);
                    cx.quit();
                }
            }
        }
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        if let Ok(pairing) = self.pairing.lock() {
            if let Some(ref request) = *pairing {
                if request.expires_at_unix > now {
                    let request_id = request.request_id.to_string();
                    if self.last_activated_request.as_deref() != Some(request_id.as_str()) {
                        self.last_activated_request = Some(request_id.clone());
                        self.pending_pairing_request = Some(request.clone());
                        if std::env::var_os("QINGLINK_TEST_MODE").is_some() {
                            let pin_file = std::env::temp_dir().join("qinglink-test-pin.txt");
                            let _ = std::fs::write(&pin_file, &request.pin);
                        }
                    }
                }
            }
        }
    }

    fn flush_pending_pairing(&mut self, cx: &mut Context<Self>) {
        let Some(request) = self.pending_pairing_request.take() else {
            return;
        };
        let pairing = Arc::clone(&self.pairing);
        let state = self.app_state.clone();
        let request_device_id = request.device_id.clone();
        let popup_bounds = Bounds::centered(None, size(px(380.0), px(260.0)), cx);
        cx.activate(true);
        let _ = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(popup_bounds)),
                titlebar: Some(gpui::TitlebarOptions {
                    title: Some("轻联配对".into()),
                    ..Default::default()
                }),
                kind: WindowKind::PopUp,
                focus: true,
                is_movable: true,
                is_resizable: false,
                is_minimizable: false,
                show: true,
                window_background: WindowBackgroundAppearance::Blurred,
                window_decorations: Some(WindowDecorations::Client),
                ..Default::default()
            },
            move |window, cx| {
                cx.new(|cx| {
                    super::pairing::PairingPopup::new(pairing, state, request_device_id, window, cx)
                })
            },
        );
    }

    fn restart_desktop(&mut self, cx: &mut Context<Self>) {
        let Ok(executable) = std::env::current_exe() else {
            self.show_message("无法定位桌面可执行文件".into());
            cx.notify();
            return;
        };
        let command = format!(
            "timeout /t 1 /nobreak >nul & \"{}\" --silent",
            executable.display()
        );
        let result = std::process::Command::new("cmd.exe")
            .args(["/C", "start", "", "/b", "cmd.exe", "/C", &command])
            .spawn();
        match result {
            Ok(_) => cx.quit(),
            Err(error) => {
                self.show_message(format!("重启桌面失败: {error}"));
                cx.notify();
            }
        }
    }

    pub(crate) fn toggle_service(&mut self, cx: &mut Context<Self>) {
        self.paused = !self.paused;
        let save_error = self.app_state.app_db.lock().ok().and_then(|database| {
            database
                .set_setting("service.paused", &self.paused.to_string())
                .err()
        });
        if let Some(error) = save_error {
            self.show_message(format!("保存服务状态失败: {error}"));
        }
        let desired = if self.paused {
            DesiredServiceState::Paused
        } else {
            DesiredServiceState::Running
        };
        if self.desired_service.send(desired).is_err() {
            self.show_message("服务控制器不可用".into());
        } else {
            self.show_message(if self.paused {
                "远程控制服务已暂停".into()
            } else {
                "远程控制服务正在恢复".into()
            });
        }
        if let Some(tray) = &self.tray {
            tray.set_paused(self.paused);
        }
        cx.notify();
    }

    pub(crate) fn toggle_autostart(&mut self, cx: &mut Context<Self>) {
        let next = !self.autostart;
        let result = if next {
            qinglink_windows::autostart::enable()
        } else {
            qinglink_windows::autostart::disable()
        };
        match result {
            Ok(()) => {
                self.autostart = next;
                if let Some(tray) = &self.tray {
                    tray.set_autostart(next);
                }
                self.show_message(if next {
                    "已启用自动启动".into()
                } else {
                    "已禁用自动启动".into()
                });
            }
            Err(error) => self.show_message(format!("修改自动启动失败: {error}")),
        }
        cx.notify();
    }

    pub(crate) fn toggle_logging(&mut self, cx: &mut Context<Self>) {
        let next = !self.debug_logging;
        self.debug_logging = next;
        self.log_switch.set_debug(next);
        let save_error = self.app_state.app_db.lock().ok().and_then(|database| {
            database
                .set_setting("logging.debug", &next.to_string())
                .err()
        });
        if let Some(error) = save_error {
            self.show_message(format!("保存日志设置失败: {error}"));
        } else {
            self.show_message(if next {
                "已启用调试日志".into()
            } else {
                "已恢复默认日志级别".into()
            });
        }
        cx.notify();
    }

    fn nav(&self, cx: &mut Context<Self>) -> AnyElement {
        let items = [
            (Page::Overview, "概览"),
            (Page::Devices, "设备"),
            (Page::Catalog, "应用"),
            (Page::Settings, "设置"),
        ];
        let mut navigation = div()
            .w(px(156.0))
            .h_full()
            .p(px(14.0))
            .flex()
            .flex_col()
            .gap(px(6.0));
        for (index, (page, label)) in items.into_iter().enumerate() {
            let selected = self.page == page;
            navigation = navigation.child(
                div()
                    .id(("nav", index))
                    .h(px(38.0))
                    .px(px(12.0))
                    .flex()
                    .items_center()
                    .rounded(px(6.0))
                    .bg(gpui::rgb(if selected {
                        0xe1f5fe
                    } else {
                        theme::BACKGROUND
                    }))
                    .text_color(gpui::rgb(if selected {
                        theme::TEXT
                    } else {
                        theme::TEXT_MUTED
                    }))
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.page = page;
                        this.message = None;
                        cx.notify();
                    }))
                    .child(label),
            );
        }
        navigation.into_any_element()
    }

    fn page_content(&mut self, cx: &mut Context<Self>) -> AnyElement {
        match self.page {
            Page::Overview => super::pages::overview(self, cx),
            Page::Devices => super::pages::devices(self, cx),
            Page::Catalog => super::pages::catalog(self),
            Page::Settings => super::pages::settings(self, cx),
        }
    }
}

impl Render for RemoteDashboard {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.handle_tray(window, cx);
        div()
            .size_full()
            .bg(gpui::rgb(theme::BACKGROUND))
            .text_color(gpui::rgb(theme::TEXT))
            .flex()
            .child(self.nav(cx))
            .child(
                div()
                    .flex_1()
                    .h_full()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .p(px(24.0))
                            .pb(px(0.0))
                            .child(
                                div()
                                    .text_size(px(24.0))
                                    .font_weight(FontWeight::BOLD)
                                    .child("轻联"),
                            )
                            .when_some(self.message.clone(), |element, message| {
                                element.child(
                                    div()
                                        .mt(px(10.0))
                                        .p(px(10.0))
                                        .rounded(px(6.0))
                                        .bg(gpui::rgb(0xd1fae5))
                                        .text_color(gpui::rgb(0x065f46))
                                        .child(message),
                                )
                            }),
                    )
                    .child(
                        div()
                            .flex_1()
                            .h_full()
                            .child(div().p(px(24.0)).pt(px(16.0)).child(self.page_content(cx))),
                    ),
            )
    }
}
