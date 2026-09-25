use gpui::{
    ElementId, FontWeight, InteractiveElement, ParentElement, Stateful, StatefulInteractiveElement,
    Styled, Window, div, px,
};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

use qinglink_ui::theme;

use crate::tray::set_main_window_handle;

pub fn section(title: impl Into<String>, subtitle: impl Into<String>) -> gpui::Div {
    div()
        .flex()
        .flex_col()
        .gap(px(12.0))
        .child(
            div()
                .text_size(px(18.0))
                .font_weight(FontWeight::BOLD)
                .child(title.into()),
        )
        .child(muted(subtitle))
}

pub fn native_window_handle(window: &Window) -> Option<isize> {
    match HasWindowHandle::window_handle(window).ok()?.as_raw() {
        RawWindowHandle::Win32(handle) => Some(handle.hwnd.get()),
        _ => None,
    }
}

pub fn hide_window(window: &Window) {
    if let Some(handle) = native_window_handle(window) {
        qinglink_windows::window::hide_native_window(handle);
    }
}

pub fn show_window(window: &Window) {
    if let Some(handle) = native_window_handle(window) {
        qinglink_windows::window::show_native_window(handle);
    }
    window.activate_window();
}

pub fn muted(text: impl Into<String>) -> gpui::Div {
    div()
        .text_size(px(13.0))
        .text_color(gpui::rgb(theme::TEXT_MUTED))
        .child(text.into())
}

pub fn action_button(
    label: impl Into<String>,
    id: impl Into<ElementId>,
    handler: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> Stateful<gpui::Div> {
    div()
        .id(id)
        .h(px(32.0))
        .px(px(12.0))
        .flex()
        .items_center()
        .rounded(px(5.0))
        .bg(gpui::rgb(theme::SUCCESS))
        .cursor_pointer()
        .text_color(gpui::rgb(0xffffff))
        .on_click(handler)
        .child(label.into())
}

pub fn disabled_button(label: impl Into<String>) -> gpui::Div {
    div()
        .h(px(32.0))
        .px(px(12.0))
        .flex()
        .items_center()
        .rounded(px(5.0))
        .bg(gpui::rgb(0xe5e7eb))
        .text_color(gpui::rgb(theme::TEXT_MUTED))
        .child(label.into())
}

pub fn status_row(label: &'static str, status: &'static str) -> gpui::Div {
    div()
        .h(px(34.0))
        .flex()
        .items_center()
        .justify_between()
        .child(label)
        .child(muted(status))
}

pub fn setting_row(
    label: &'static str,
    action: &'static str,
    id: &'static str,
    handler: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> gpui::Div {
    div()
        .h(px(52.0))
        .flex()
        .items_center()
        .justify_between()
        .border_b_1()
        .border_color(gpui::rgb(0xe5e7eb))
        .child(label)
        .child(action_button(action, id, handler))
}

pub fn store_main_window_handle(window: &Window) {
    if let Some(handle) = native_window_handle(window) {
        set_main_window_handle(handle);
    }
}
