use gpui::prelude::FluentBuilder;
use gpui::{AnyElement, Context, FontWeight, IntoElement, ParentElement, Styled, div, px};
use qinglink_ui::theme;

use crate::{
    input_install::{InputOperation, start_elevated},
    view::helpers::{action_button, disabled_button, muted, section, setting_row, status_row},
};

use super::RemoteDashboard;

pub fn overview(this: &mut RemoteDashboard, cx: &mut Context<RemoteDashboard>) -> AnyElement {
    let status = this
        .service_status
        .lock()
        .map(|value| value.clone())
        .unwrap_or_default();
    let now = time::OffsetDateTime::now_utc().unix_timestamp();
    let pairing = this
        .pairing
        .lock()
        .ok()
        .and_then(|value| value.clone())
        .filter(|request| request.expires_at_unix > now);
    let status_text = if status.running {
        format!(
            "运行中 · {}",
            status.address.unwrap_or_else(|| "0.0.0.0:3721".into())
        )
    } else if this.paused {
        "已暂停".into()
    } else if let Some(ref error) = status.error {
        format!("错误 · {error}")
    } else {
        "启动中".into()
    };
    if let Some(ref tray) = this.tray {
        tray.set_status(this.paused, status.running, status.error.as_deref());
    }
    let input_status = this.input_install.lock().ok().map(|s| s.status.clone());
    let needs_repair = input_status
        .as_ref()
        .map(|s| s.package_available && (!s.available || !s.service_running))
        .unwrap_or(false);
    let mut content = section("Service Status", status_text)
        .child(div().h(px(1.0)).bg(gpui::rgb(0xe5e7eb)))
        .child(
            div()
                .text_color(gpui::rgb(theme::TEXT_MUTED))
                .child(if pairing.is_some() {
                    "配对请求窗口已打开"
                } else {
                    "等待局域网内移动设备配对"
                }),
        );
    if needs_repair {
        content = content.child(
            div()
                .h(px(44.0))
                .flex()
                .items_center()
                .justify_between()
                .child(muted("安全桌面输入服务错误"))
                .child(action_button(
                    "修复服务",
                    "repair-service-overview",
                    cx.listener(|this, _, _, cx| {
                        start_elevated(&this.input_install, InputOperation::Install);
                        this.show_message("正在请求管理员权限以修复服务...".into());
                        cx.notify();
                    }),
                )),
        );
    }
    content.into_any_element()
}

pub fn devices(this: &mut RemoteDashboard, cx: &mut Context<RemoteDashboard>) -> AnyElement {
    let devices = this.app_state.token_store.list_active();
    let mut content = section("已配对设备", format!("{} 个设备", devices.len()));
    if devices.is_empty() {
        content = content.child(muted("暂无已配对设备"));
    }
    for (index, (prefix, device)) in devices.into_iter().enumerate() {
        let device_name = device.device_name.clone();
        content = content.child(
            div()
                .h(px(48.0))
                .flex()
                .items_center()
                .justify_between()
                .border_b_1()
                .border_color(gpui::rgb(0xe5e7eb))
                .child(format!("{} · {}…", device.device_name, prefix))
                .child(action_button(
                    "撤销",
                    ("revoke", index),
                    cx.listener(move |this, _, _, cx| {
                        if this.app_state.token_store.revoke_by_name(&device_name) {
                            this.show_message(format!("已撤销 {device_name}"));
                        }
                        cx.notify();
                    }),
                )),
        );
    }
    content.into_any_element()
}

pub fn catalog(this: &RemoteDashboard) -> AnyElement {
    let apps = this
        .app_state
        .app_db
        .lock()
        .ok()
        .and_then(|database| database.get_visible_apps().ok())
        .unwrap_or_default();
    let mut content = section("应用目录", format!("{} 个可启动项", apps.len()));
    for app in apps.into_iter().take(12) {
        content = content.child(
            div()
                .h(px(42.0))
                .flex()
                .items_center()
                .justify_between()
                .border_b_1()
                .border_color(gpui::rgb(0xe5e7eb))
                .child(app.name)
                .child(muted(app.source)),
        );
    }
    content.into_any_element()
}

pub fn settings(this: &mut RemoteDashboard, cx: &mut Context<RemoteDashboard>) -> AnyElement {
    let service_label = if this.paused {
        "恢复服务"
    } else {
        "暂停服务"
    };
    let input_state = this
        .input_install
        .lock()
        .map(|value| value.clone())
        .unwrap_or_default();
    let input_status = &input_state.status;
    let service_status = if input_status.service_running {
        "运行中"
    } else if input_status.service_installed {
        "已安装，未运行"
    } else {
        "未安装"
    };
    let install_label = if input_status.service_installed {
        "修复"
    } else {
        "安装"
    };
    let install_control = if input_state.busy || !input_status.package_available {
        disabled_button(if input_state.busy {
            "处理中"
        } else {
            "缺少发布包"
        })
        .into_any_element()
    } else {
        action_button(
            install_label,
            "install-secure-input",
            cx.listener(|this, _, _, cx| {
                start_elevated(&this.input_install, InputOperation::Install);
                cx.notify();
            }),
        )
        .into_any_element()
    };
    let uninstall_control = if input_state.busy || !input_status.service_installed {
        disabled_button("卸载").into_any_element()
    } else {
        action_button(
            "卸载",
            "uninstall-secure-input",
            cx.listener(|this, _, _, cx| {
                start_elevated(&this.input_install, InputOperation::Uninstall);
                cx.notify();
            }),
        )
        .into_any_element()
    };

    let service_control = if input_status.service_installed {
        let start_stop_button = if input_status.service_running {
            action_button(
                "停止",
                "stop-secure-input",
                cx.listener(|this, _, _, cx| {
                    this.show_message("正在停止服务...".into());
                    cx.notify();
                    let state = std::sync::Arc::clone(&this.input_install);
                    std::thread::spawn(move || {
                        let result = crate::input_install::stop_service_elevated();
                        crate::input_install::refresh(&state);
                        let message = match result {
                            Ok(output) => output,
                            Err(e) => format!("停止服务失败: {e}"),
                        };
                        let _ = state.lock().map(|mut s| s.result_message = Some(message));
                    });
                }),
            )
        } else {
            action_button(
                "启动",
                "start-secure-input",
                cx.listener(|this, _, _, cx| {
                    this.show_message("正在请求管理员权限...".into());
                    cx.notify();
                    let state = std::sync::Arc::clone(&this.input_install);
                    std::thread::spawn(move || {
                        let result = crate::input_install::start_service_elevated();
                        crate::input_install::refresh(&state);
                        let message = match result {
                            Ok(output) => output,
                            Err(e) => format!("启动服务失败: {e}"),
                        };
                        let _ = state.lock().map(|mut s| s.result_message = Some(message));
                    });
                }),
            )
        };
        let restart_button = action_button(
            "重启",
            "restart-secure-input",
            cx.listener(|this, _, _, cx| {
                this.show_message("正在请求管理员权限...".into());
                cx.notify();
                let state = std::sync::Arc::clone(&this.input_install);
                std::thread::spawn(move || {
                    let result = crate::input_install::restart_service_elevated();
                    crate::input_install::refresh(&state);
                    let message = match result {
                        Ok(output) => output,
                        Err(e) => format!("重启服务失败: {e}"),
                    };
                    let _ = state.lock().map(|mut s| s.result_message = Some(message));
                });
            }),
        );
        vec![
            start_stop_button.into_any_element(),
            restart_button.into_any_element(),
            uninstall_control,
        ]
    } else {
        vec![install_control]
    };

    section("服务设置", "TCP 3721 · UDP 3723 局域网自动发现")
        .child(setting_row(
            "远程控制服务",
            service_label,
            "toggle-service",
            cx.listener(|this, _, _, cx| this.toggle_service(cx)),
        ))
        .child(setting_row(
            "开机自动启动",
            if this.autostart { "已启用" } else { "已禁用" },
            "toggle-autostart",
            cx.listener(|this, _, _, cx| this.toggle_autostart(cx)),
        ))
        .child(setting_row(
            "调试日志",
            if this.debug_logging { "已启用" } else { "已禁用" },
            "toggle-debug-log",
            cx.listener(|this, _, _, cx| this.toggle_logging(cx)),
        ))
        .child(setting_row(
            "局域网防火墙规则",
            "创建规则",
            "firewall-rule",
            cx.listener(|this, _, _, cx| {
                this.show_message("正在请求管理员权限...".into());
                cx.notify();
                let state = std::sync::Arc::clone(&this.input_install);
                std::thread::spawn(move || {
                    let udp = qinglink_windows::elevation::run_elevated(
                        "powershell",
                        "-Command \"netsh advfirewall firewall add rule name='QingLink Discovery (UDP 3723)' dir=in action=allow protocol=UDP localport=3723 profile=private; netsh advfirewall firewall add rule name='QingLink HTTP (TCP 3721)' dir=in action=allow protocol=TCP localport=3721 profile=private\""
                    );
                    let message = match udp {
                        Ok(()) => "防火墙规则已创建 (UDP 3723 + TCP 3721)".to_string(),
                        Err(e) => format!("创建规则失败: {e}"),
                    };
                    let _ = state.lock().map(|mut s| s.result_message = Some(message));
                });
            }),
        ))
        .child(muted("服务仅监听局域网，无公网端口转发。"))
        .child(div().h(px(1.0)).bg(gpui::rgb(0x29323c)))
        .child(
            div()
                .text_size(px(16.0))
                .font_weight(FontWeight::BOLD)
                .child("安全桌面输入"),
        )
        .child(muted(input_status.detail.clone()))
        .child(status_row("输入服务", service_status))
        .child(
            div()
                .h(px(44.0))
                .flex()
                .items_center()
                .gap(px(8.0))
                .children(service_control)
                .child(action_button(
                    "刷新状态",
                    "refresh-secure-input",
                    cx.listener(|this, _, _, cx| {
                        crate::input_install::refresh(&this.input_install);
                        cx.notify();
                    }),
                )),
        )
        .when_some(input_state.result_message, |element, message| {
            element.child(muted(message))
        })
        .into_any_element()
}
