//! 键鼠输入注入：鼠标移动、按键、滚轮、文本与 HID 按键事件。

use super::platform_impl;

/// 以相对位移移动鼠标指针。
///
/// # Errors
///
/// 当相对位移超过协议上限（`MAX_MOUSE_MOVE_DELTA`）、安全桌面注入失败，或
/// `SendInput` 与输入服务回退均失败时返回 `Err`；非 Windows 平台恒返回不支持错误。
pub fn mouse_move(dx: i32, dy: i32) -> anyhow::Result<()> {
    platform_impl::mouse_move(dx, dy)
}

/// 将鼠标指针移动到屏幕绝对坐标。
///
/// # Errors
///
/// 当读不到虚拟屏幕分辨率、安全桌面注入失败、安全桌面状态查询失败，或
/// `SendInput` 注入失败时返回 `Err`；非 Windows 平台恒返回不支持错误。
pub fn mouse_move_absolute(x: u32, y: u32) -> anyhow::Result<()> {
    platform_impl::mouse_move_absolute(x, y)
}

/// 执行一次鼠标按键点击。
///
/// # Errors
///
/// 当按键名不是 `"left"`/`"right"`、安全桌面注入失败，或 `SendInput` 与输入
/// 服务回退均失败时返回 `Err`；非 Windows 平台恒返回不支持错误。
pub fn mouse_button(button: &str) -> anyhow::Result<()> {
    platform_impl::mouse_button(button)
}

/// 按下或抬起指定的鼠标按键。
///
/// # Errors
///
/// 当按键名与按下/抬起状态组合不受支持、安全桌面注入失败，或 `SendInput` 与
/// 输入服务回退均失败时返回 `Err`；非 Windows 平台恒返回不支持错误。
pub fn mouse_button_state(button: &str, pressed: bool) -> anyhow::Result<()> {
    platform_impl::mouse_button_state(button, pressed)
}

/// 滚动鼠标滚轮，`delta` 为正表示向上滚动。
///
/// # Errors
///
/// 当安全桌面注入失败，或 `SendInput` 与输入服务回退均失败时返回 `Err`；
/// 非 Windows 平台恒返回不支持错误。
pub fn mouse_wheel(delta: i32) -> anyhow::Result<()> {
    platform_impl::mouse_wheel(delta)
}

/// 输入一段文本。
///
/// # Errors
///
/// 当文本为空或超过 64 KiB、安全桌面注入失败，或 `SendInput` 未能注入全部
/// 字符时返回 `Err`；非 Windows 平台恒返回不支持错误。
pub fn send_text(text: &str) -> anyhow::Result<()> {
    platform_impl::send_text(text)
}

/// 模拟按下并释放一次指定按键。
///
/// # Errors
///
/// 当按键名不在支持的按键表内、安全桌面注入失败，或 HID 注入与输入服务回退
/// 均失败时返回 `Err`；非 Windows 平台恒返回不支持错误。
pub fn key_press(key: &str) -> anyhow::Result<()> {
    platform_impl::key_press(key)
}

/// 按 HID 用法码与修饰键位图发送按键按下或抬起事件。
///
/// # Errors
///
/// 当 HID usage 不在 `0x04..=0x73` 范围内、安全桌面注入失败、usage 没有对应的
/// Windows 扫描码，或 HID 注入与输入服务回退均失败时返回 `Err`；非 Windows
/// 平台恒返回不支持错误。
pub fn key_event(usage: u8, modifiers: u8, pressed: bool) -> anyhow::Result<()> {
    platform_impl::key_event(usage, modifiers, pressed)
}

/// 读取物理鼠标左右中键的当前按下状态。
///
/// 不返回 `Result`：这条信息只用于界面提示与拖拽去重，读不到时按"全部抬起"处理，
/// 不值得让调用方每次都处理错误分支。非 Windows 平台恒为全部抬起。
pub fn mouse_buttons_state() -> qinglink_protocol::MouseButtonsState {
    platform_impl::mouse_buttons_state()
}
