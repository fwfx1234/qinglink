use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use gpui::{
    AppContext, Bounds, Context, FontWeight, InteractiveElement, IntoElement, ParentElement,
    Render, Styled, Window, div, px, size,
};
use qinglink_server::{AppState, server::pairing::PairingNotification};
use qinglink_ui::theme;

use crate::view::helpers::{action_button, muted};

#[cfg(target_os = "windows")]
use gpui::Context as _;

pub struct PairingPopup {
    pairing: Arc<Mutex<Option<PairingNotification>>>,
    app_state: AppState,
    should_close: bool,
}

impl PairingPopup {
    pub fn new(
        pairing: Arc<Mutex<Option<PairingNotification>>>,
        app_state: AppState,
        _device_id: String,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let close_pairing = Arc::clone(&pairing);
        let close_state = app_state.clone();
        window.on_window_should_close(cx, move |window, _cx| {
            if let Ok(mut snapshot) = close_pairing.lock() {
                if let Some(ref request) = *snapshot {
                    if let Ok(mut pairing_state) = close_state.pairing.lock() {
                        pairing_state.cancel(request.request_id);
                    }
                }
                *snapshot = None;
            }
            window.remove_window();
            true
        });
        let pairing_monitor = Arc::clone(&pairing);
        cx.spawn(async move |popup, async_cx| {
            loop {
                async_cx
                    .background_executor()
                    .timer(Duration::from_millis(200))
                    .await;
                if popup
                    .update(async_cx, |popup, cx| {
                        let now = time::OffsetDateTime::now_utc().unix_timestamp();
                        let request = pairing_monitor
                            .lock()
                            .ok()
                            .and_then(|v| v.clone())
                            .filter(|r| r.expires_at_unix > now);
                        if request.is_none() {
                            popup.should_close = true;
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
        #[cfg(target_os = "windows")]
        unsafe {
            use windows::Win32::UI::Input::KeyboardAndMouse::{
                INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, SendInput, VK_MENU,
            };
            let inputs = [
                INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: VK_MENU,
                            wScan: 0,
                            dwFlags: Default::default(),
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                },
                INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: VK_MENU,
                            wScan: 0,
                            dwFlags: KEYEVENTF_KEYUP,
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                },
            ];
            let _ = SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
        }
        window.activate_window();
        Self {
            pairing,
            app_state,
            should_close: false,
        }
    }
}

impl Render for PairingPopup {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.should_close {
            self.should_close = false;
            if let Ok(mut snapshot) = self.pairing.lock() {
                *snapshot = None;
            }
            window.remove_window();
            return div().size_full().into_any_element();
        }
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        let request = self
            .pairing
            .lock()
            .ok()
            .and_then(|value| value.clone())
            .filter(|r| r.expires_at_unix > now);
        let Some(request) = request else {
            return div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .bg(gpui::rgb(theme::SURFACE))
                .child("等待配对请求...")
                .into_any_element();
        };
        div()
            .size_full()
            .p(px(24.0))
            .bg(gpui::rgb(theme::SURFACE))
            .flex()
            .flex_col()
            .gap(px(12.0))
            .child(
                div()
                    .font_weight(FontWeight::BOLD)
                    .text_size(px(18.0))
                    .text_color(gpui::rgb(theme::TEXT))
                    .child("新的配对请求"),
            )
            .child(
                div()
                    .text_color(gpui::rgb(theme::TEXT_SECONDARY))
                    .child(format!("{} 想要连接", request.device_name)),
            )
            .child(
                div()
                    .w_full()
                    .p(px(16.0))
                    .bg(gpui::rgb(0xe8f5e9))
                    .rounded(px(8.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .text_size(px(52.0))
                            .font_weight(FontWeight::BOLD)
                            .text_color(gpui::rgb(0x1b5e20))
                            .child(request.pin),
                    ),
            )
            .child(
                div()
                    .text_size(px(13.0))
                    .text_color(gpui::rgb(theme::TEXT_MUTED))
                    .child(format!(
                        "有效期 {} 秒。请在移动设备上输入此验证码。",
                        request.expires_at_unix - now
                    )),
            )
            .child(action_button(
                "取消配对",
                "cancel-pairing-popup",
                cx.listener(|this, _, _window, _cx| {
                    if let Some(request) = this.pairing.lock().ok().and_then(|value| value.clone())
                        && let Ok(mut pairing) = this.app_state.pairing.lock()
                    {
                        pairing.cancel(request.request_id);
                    }
                    if let Ok(mut snapshot) = this.pairing.lock() {
                        *snapshot = None;
                    }
                    this.should_close = true;
                }),
            ))
            .into_any_element()
    }
}
