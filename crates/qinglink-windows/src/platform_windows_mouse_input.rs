//! 鼠标注入：相对/绝对位移、单击、按下与抬起、滚轮，失败时回退到输入服务。

use std::mem;

use super::input_routing::{ensure_remote_input_desktop, require_hid_input};
use super::lock_input_pipe::LockInputPipeRequest;
use super::session_lock::is_secure_desktop;

/// 按相对位移移动鼠标指针；位移超限或注入失败时回退到输入服务。
///
/// # Errors
/// `dx`/`dy` 的绝对值超过 `qinglink_protocol::MAX_MOUSE_MOVE_DELTA`
/// （“鼠标位移过大”）；`is_secure_desktop()` 探测失败；安全桌面路径下透传
/// `crate::secure_desktop::send_mouse_move` 的错误；`SendInput` 插入事件数不为 1 时
/// 回退到输入服务，服务的管道不可用/读写或解析失败、或返回 `ok == false`
/// （“{code}: {message}”）时返回错误。
pub fn mouse_move(dx: i32, dy: i32) -> anyhow::Result<()> {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_MOVE, MOUSEINPUT, SendInput,
    };
    anyhow::ensure!(
        dx.abs() <= qinglink_protocol::MAX_MOUSE_MOVE_DELTA
            && dy.abs() <= qinglink_protocol::MAX_MOUSE_MOVE_DELTA,
        "鼠标位移过大"
    );
    let request = LockInputPipeRequest {
        version: 1,
        command: "mouse_move",
        pin: None,
        dx: Some(dx),
        dy: Some(dy),
        button: None,
        pressed: None,
        delta: None,
        usage: None,
        modifiers: None,
    };
    if crate::secure_desktop::running_in_session_zero() || is_secure_desktop()? {
        return crate::secure_desktop::send_mouse_move(dx, dy);
    }
    ensure_remote_input_desktop()?;
    let input = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx,
                dy,
                mouseData: 0,
                dwFlags: MOUSEEVENTF_MOVE,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    let inserted = unsafe { SendInput(&[input], mem::size_of::<INPUT>() as i32) };
    if inserted == 1 {
        Ok(())
    } else {
        require_hid_input(&request)
    }
}

/// 将鼠标指针绝对定位到虚拟桌面坐标 (x, y)。
///
/// # Errors
/// 读不到虚拟屏幕分辨率（“无法读取虚拟屏幕分辨率”）；`is_secure_desktop()` 探测
/// 失败；安全桌面路径下透传 `crate::secure_desktop::send_mouse_move_absolute` 的
/// 错误；或 `SendInput` 插入事件数不为 1
/// （“SendInput 绝对定位鼠标移动失败”）时返回错误。
pub fn mouse_move_absolute(x: u32, y: u32) -> anyhow::Result<()> {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_MOVE,
        MOUSEEVENTF_VIRTUALDESK, MOUSEINPUT, SendInput,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN,
        SM_YVIRTUALSCREEN,
    };
    let left = unsafe { GetSystemMetrics(SM_XVIRTUALSCREEN) };
    let top = unsafe { GetSystemMetrics(SM_YVIRTUALSCREEN) };
    let width = unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) };
    let height = unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) };
    anyhow::ensure!(width > 0 && height > 0, "无法读取虚拟屏幕分辨率");
    // The protocol carries physical screen coordinates. Convert them to the
    // 0..65535 virtual-screen range required by MOUSEEVENTF_ABSOLUTE.
    let abs_x =
        ((((x as i64 - left as i64) * 65535) / (width as i64 - 1).max(1)).clamp(0, 65535)) as i32;
    let abs_y =
        ((((y as i64 - top as i64) * 65535) / (height as i64 - 1).max(1)).clamp(0, 65535)) as i32;
    if crate::secure_desktop::running_in_session_zero() || is_secure_desktop()? {
        return crate::secure_desktop::send_mouse_move_absolute(x, y);
    }
    ensure_remote_input_desktop()?;
    let input = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: abs_x,
                dy: abs_y,
                mouseData: 0,
                dwFlags: MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_MOVE | MOUSEEVENTF_VIRTUALDESK,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    let inserted = unsafe { SendInput(&[input], mem::size_of::<INPUT>() as i32) };
    anyhow::ensure!(inserted == 1, "SendInput 绝对定位鼠标移动失败");
    Ok(())
}

/// 在当前位置完成一次鼠标点击，`button` 支持 "left" 与 "right"。
///
/// # Errors
/// `button` 不是 `left`/`right`（“不支持的鼠标按键”）；`is_secure_desktop()` 探测
/// 失败；安全桌面路径下透传 `crate::secure_desktop::send_mouse_button` 的错误；
/// `SendInput` 插入事件数不为 2 时回退到输入服务，服务的管道不可用/读写或解析失败、
/// 或返回 `ok == false`（“{code}: {message}”）时返回错误。
pub fn mouse_button(button: &str) -> anyhow::Result<()> {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP,
        MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEINPUT, SendInput,
    };
    let (down, up) = match button {
        "left" => (MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP),
        "right" => (MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP),
        _ => anyhow::bail!("不支持的鼠标按键"),
    };
    let request = LockInputPipeRequest {
        version: 1,
        command: "mouse_button",
        pin: None,
        dx: None,
        dy: None,
        button: Some(button),
        pressed: None,
        delta: None,
        usage: None,
        modifiers: None,
    };
    if crate::secure_desktop::running_in_session_zero() || is_secure_desktop()? {
        return crate::secure_desktop::send_mouse_button(button, true)
            .and_then(|_| crate::secure_desktop::send_mouse_button(button, false));
    }
    ensure_remote_input_desktop()?;
    let inputs = [
        INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: 0,
                    dy: 0,
                    mouseData: 0,
                    dwFlags: down,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
        INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: 0,
                    dy: 0,
                    mouseData: 0,
                    dwFlags: up,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
    ];
    let inserted = unsafe { SendInput(&inputs, mem::size_of::<INPUT>() as i32) };
    if inserted == 2 {
        Ok(())
    } else {
        require_hid_input(&request)
    }
}

/// 仅按下或仅抬起鼠标按键，用于按住拖选等需要持续按住的操作。
///
/// # Errors
/// `button` 不是 `left`/`right`（“不支持的鼠标按键状态”）；`is_secure_desktop()`
/// 探测失败；安全桌面路径下透传 `crate::secure_desktop::send_mouse_button` 的错误；
/// `SendInput` 插入事件数不为 1 时回退到输入服务，服务的管道不可用/读写或解析失败、
/// 或返回 `ok == false`（“{code}: {message}”）时返回错误。
pub fn mouse_button_state(button: &str, pressed: bool) -> anyhow::Result<()> {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP,
        MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEINPUT, SendInput,
    };
    let flag = match (button, pressed) {
        ("left", true) => MOUSEEVENTF_LEFTDOWN,
        ("left", false) => MOUSEEVENTF_LEFTUP,
        ("right", true) => MOUSEEVENTF_RIGHTDOWN,
        ("right", false) => MOUSEEVENTF_RIGHTUP,
        _ => anyhow::bail!("不支持的鼠标按键状态"),
    };
    let request = LockInputPipeRequest {
        version: 1,
        command: "mouse_button_state",
        pin: None,
        dx: None,
        dy: None,
        button: Some(button),
        pressed: Some(pressed),
        delta: None,
        usage: None,
        modifiers: None,
    };
    if crate::secure_desktop::running_in_session_zero() || is_secure_desktop()? {
        return crate::secure_desktop::send_mouse_button(button, pressed);
    }
    ensure_remote_input_desktop()?;
    let input = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: 0,
                dy: 0,
                mouseData: 0,
                dwFlags: flag,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    let inserted = unsafe { SendInput(&[input], mem::size_of::<INPUT>() as i32) };
    if inserted == 1 {
        Ok(())
    } else {
        require_hid_input(&request)
    }
}

/// 滚动鼠标滚轮，`delta` 为正表示向上滚动。
///
/// # Errors
/// `is_secure_desktop()` 探测失败；安全桌面路径下透传
/// `crate::secure_desktop::send_mouse_wheel` 的错误；`SendInput` 插入事件数不为 1 时
/// 回退到输入服务，服务的管道不可用/读写或解析失败、或返回 `ok == false`
/// （“{code}: {message}”）时返回错误。`delta` 超过 ±20 会被截断，不报错。
pub fn mouse_wheel(delta: i32) -> anyhow::Result<()> {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_WHEEL, MOUSEINPUT, SendInput,
    };
    let request = LockInputPipeRequest {
        version: 1,
        command: "mouse_wheel",
        pin: None,
        dx: None,
        dy: None,
        button: None,
        pressed: None,
        delta: Some(delta),
        usage: None,
        modifiers: None,
    };
    if crate::secure_desktop::running_in_session_zero() || is_secure_desktop()? {
        return crate::secure_desktop::send_mouse_wheel(delta);
    }
    ensure_remote_input_desktop()?;
    let input = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: 0,
                dy: 0,
                mouseData: (delta.clamp(-20, 20) * 120) as u32,
                dwFlags: MOUSEEVENTF_WHEEL,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    let inserted = unsafe { SendInput(&[input], mem::size_of::<INPUT>() as i32) };
    if inserted == 1 {
        Ok(())
    } else {
        require_hid_input(&request)
    }
}
