//! 安全桌面的鼠标注入：相对/绝对移动、按键状态与滚轮。
//!
//! 未设置 `QINGLINK_SESSION_AGENT` 时统一转发给会话代理，其余情况直接在附加后的
//! 输入桌面上调用 `SendInput`。

use anyhow::{Context, Result, ensure};

/// 按相对位移注入鼠标移动，必要时经会话代理转发。
///
/// # Errors
/// 未设置 `QINGLINK_SESSION_AGENT` 时代理请求失败（错误带上下文
/// `SESSION_AGENT_UNAVAILABLE: mouse input requires the session agent`）；附加输入
/// 桌面失败；`SendInput` 实际插入的事件数不等于 1。
#[cfg(target_os = "windows")]
pub fn send_mouse_move(dx: i32, dy: i32) -> Result<()> {
    if std::env::var_os("QINGLINK_SESSION_AGENT").is_none() {
        return super::agent_request("mouse_move", None, Some((dx, dy)), None, None)
            .context("SESSION_AGENT_UNAVAILABLE: mouse input requires the session agent");
    }
    let _ = super::attach_input_desktop()?;
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_MOVE, MOUSEINPUT, SendInput,
    };
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
    let inserted = unsafe { SendInput(&[input], std::mem::size_of::<INPUT>() as i32) };
    ensure!(inserted == 1, "SendInput mouse move failed");
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn send_mouse_move(_: i32, _: i32) -> Result<()> {
    anyhow::bail!("secure desktop input is only supported on Windows")
}

/// 将鼠标指针绝对定位到虚拟桌面坐标 (x, y)，必要时经会话代理转发。
///
/// # Errors
/// 虚拟屏幕宽或高不为正（“cannot read virtual-screen metrics”）；未设置
/// `QINGLINK_SESSION_AGENT` 时代理请求失败（错误带上下文
/// `SESSION_AGENT_UNAVAILABLE: absolute mouse input requires the session agent`）；
/// 附加输入桌面失败；`SendInput` 实际插入的事件数不等于 1。
#[cfg(target_os = "windows")]
pub fn send_mouse_move_absolute(x: u32, y: u32) -> Result<()> {
    use windows::Win32::UI::WindowsAndMessaging::{
        GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN,
        SM_YVIRTUALSCREEN,
    };
    let left = unsafe { GetSystemMetrics(SM_XVIRTUALSCREEN) };
    let top = unsafe { GetSystemMetrics(SM_YVIRTUALSCREEN) };
    let width = unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) };
    let height = unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) };
    ensure!(
        width > 0 && height > 0,
        "cannot read virtual-screen metrics"
    );
    let abs_x =
        ((((x as i64 - left as i64) * 65535) / (width as i64 - 1).max(1)).clamp(0, 65535)) as i32;
    let abs_y =
        ((((y as i64 - top as i64) * 65535) / (height as i64 - 1).max(1)).clamp(0, 65535)) as i32;
    if std::env::var_os("QINGLINK_SESSION_AGENT").is_none() {
        return super::agent_request(
            "mouse_move_absolute",
            None,
            Some((abs_x, abs_y)),
            None,
            None,
        )
        .context("SESSION_AGENT_UNAVAILABLE: absolute mouse input requires the session agent");
    }
    let _ = super::attach_input_desktop()?;
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_MOVE,
        MOUSEEVENTF_VIRTUALDESK, MOUSEINPUT, SendInput,
    };
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
    let inserted = unsafe { SendInput(&[input], std::mem::size_of::<INPUT>() as i32) };
    ensure!(inserted == 1, "SendInput absolute mouse move failed");
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn send_mouse_move_absolute(_: u32, _: u32) -> Result<()> {
    anyhow::bail!("secure desktop input is only supported on Windows")
}

/// 按下或抬起指定鼠标按键（`left`/`right`），必要时经会话代理转发。
///
/// # Errors
/// 未设置 `QINGLINK_SESSION_AGENT` 时代理请求失败（错误带上下文
/// `SESSION_AGENT_UNAVAILABLE: mouse input requires the session agent`）；附加输入
/// 桌面失败；`button` 不是 `left`/`right` 时返回“unsupported mouse button”；
/// `SendInput` 实际插入的事件数不等于 1。
#[cfg(target_os = "windows")]
pub fn send_mouse_button(button: &str, pressed: bool) -> Result<()> {
    if std::env::var_os("QINGLINK_SESSION_AGENT").is_none() {
        return super::agent_request(
            "mouse_button_state",
            None,
            None,
            Some(button),
            Some(pressed),
        )
        .context("SESSION_AGENT_UNAVAILABLE: mouse input requires the session agent");
    }
    let _ = super::attach_input_desktop()?;
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP,
        MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEINPUT, SendInput,
    };
    let flag = match (button, pressed) {
        ("left", true) => MOUSEEVENTF_LEFTDOWN,
        ("left", false) => MOUSEEVENTF_LEFTUP,
        ("right", true) => MOUSEEVENTF_RIGHTDOWN,
        ("right", false) => MOUSEEVENTF_RIGHTUP,
        _ => anyhow::bail!("unsupported mouse button"),
    };
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
    let inserted = unsafe { SendInput(&[input], std::mem::size_of::<INPUT>() as i32) };
    ensure!(inserted == 1, "SendInput mouse button failed");
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn send_mouse_button(_: &str, _: bool) -> Result<()> {
    anyhow::bail!("secure desktop input is only supported on Windows")
}

/// 注入鼠标滚轮滚动，`delta` 为正表示向上滚动。
///
/// # Errors
/// 未设置 `QINGLINK_SESSION_AGENT` 时代理请求失败（错误带上下文
/// `SESSION_AGENT_UNAVAILABLE: mouse input requires the session agent`）；附加输入
/// 桌面失败；`SendInput` 实际插入的事件数不等于 1。
#[cfg(target_os = "windows")]
pub fn send_mouse_wheel(delta: i32) -> Result<()> {
    if std::env::var_os("QINGLINK_SESSION_AGENT").is_none() {
        return super::agent_request("mouse_wheel", None, Some((delta, 0)), None, None)
            .context("SESSION_AGENT_UNAVAILABLE: mouse input requires the session agent");
    }
    let _ = super::attach_input_desktop()?;
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_WHEEL, MOUSEINPUT, SendInput,
    };
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
    let inserted = unsafe { SendInput(&[input], std::mem::size_of::<INPUT>() as i32) };
    ensure!(inserted == 1, "SendInput mouse wheel failed");
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn send_mouse_wheel(_: i32) -> Result<()> {
    anyhow::bail!("secure desktop input is only supported on Windows")
}
