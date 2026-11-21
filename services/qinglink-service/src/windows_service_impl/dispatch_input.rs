//! 鼠标与键盘注入命令的分发。
//!
//! 从 `dispatch` 拆出：那边还要处理服务级命令、窗口命令与代理命令，再加上键鼠占位会
//! 越过单文件 500 行的硬上限（AGENTS.md §2.1）。这里只负责把管道请求翻译成 `input` 调用。

use anyhow::Result;

use super::dispatch::{Request, respond_result, write_response};
use super::{input, pipe::Reply};

/// 鼠标移动、按键与滚轮注入。
pub(super) fn dispatch_mouse_command(stream: &mut Reply, request: &Request) -> Result<()> {
    match request.command.as_str() {
        "mouse_move" => respond_result(
            stream,
            input::inject_mouse_move(request.dx.unwrap_or(0), request.dy.unwrap_or(0)),
            "INPUT_FAILED",
            "SERVICE_OK",
            "鼠标移动已发送",
        ),
        "mouse_move_absolute" => respond_result(
            stream,
            input::inject_mouse_move_absolute(request.dx.unwrap_or(0), request.dy.unwrap_or(0)),
            "INPUT_FAILED",
            "SERVICE_OK",
            "绝对定位鼠标移动已发送",
        ),
        "mouse_button" => respond_result(
            stream,
            input::inject_mouse_button(request.button.as_deref().unwrap_or("")),
            "INPUT_FAILED",
            "SERVICE_OK",
            "鼠标按键已发送",
        ),
        "mouse_button_state" => respond_result(
            stream,
            input::inject_mouse_button_state(
                request.button.as_deref().unwrap_or(""),
                request.pressed.unwrap_or(false),
            ),
            "INPUT_FAILED",
            "SERVICE_OK",
            "鼠标按键状态已发送",
        ),
        "mouse_wheel" => respond_result(
            stream,
            input::inject_mouse_wheel(request.delta.unwrap_or(0)),
            "INPUT_FAILED",
            "SERVICE_OK",
            "鼠标滚轮已发送",
        ),
        _ => write_response(stream, false, "UNKNOWN_COMMAND", "未知输入命令"),
    }
}

/// 按键按下与 HID 按键事件注入。
pub(super) fn dispatch_key_command(stream: &mut Reply, request: &Request) -> Result<()> {
    match request.command.as_str() {
        "key_press" => match input::key_to_usage(request.button.as_deref().unwrap_or("")) {
            Some(usage) => respond_result(
                stream,
                input::inject_key_event(usage, 0, true)
                    .and_then(|()| input::inject_key_event(usage, 0, false)),
                "INPUT_FAILED",
                "SERVICE_OK",
                "按键已发送",
            ),
            None => write_response(stream, false, "INVALID_KEY", "不支持的按键"),
        },
        "key_event" => respond_result(
            stream,
            input::inject_key_event(
                request.usage.unwrap_or(0),
                request.modifiers.unwrap_or(0),
                request.pressed.unwrap_or(false),
            ),
            "INPUT_FAILED",
            "SERVICE_OK",
            "HID 按键已发送",
        ),
        _ => write_response(stream, false, "UNKNOWN_COMMAND", "未知输入命令"),
    }
}

/// 在安全桌面上按数字键输入 PIN，最后补一次回车。
pub(super) fn submit_pin(stream: &mut Reply, pin: Option<&str>) -> Result<()> {
    let Some(pin) = pin else {
        return write_response(stream, false, "INVALID_PIN_FORMAT", "PIN 不能为空");
    };
    anyhow::ensure!(
        (4..=32).contains(&pin.len()) && pin.bytes().all(|byte| byte.is_ascii_digit()),
        "PIN 必须是 4-32 位数字"
    );
    let result = pin
        .chars()
        .try_for_each(|digit| {
            let usage = if digit == '0' {
                0x27
            } else {
                0x1d + digit.to_digit(10).unwrap_or(0) as u8
            };
            input::inject_key_event(usage, 0, true)?;
            input::inject_key_event(usage, 0, false)
        })
        .and_then(|()| input::inject_key_event(0x28, 0, true))
        .and_then(|()| input::inject_key_event(0x28, 0, false));
    respond_result(
        stream,
        result,
        "INPUT_FAILED",
        "SERVICE_OK",
        "PIN 已通过 HID 发送",
    )
}
