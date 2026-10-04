//! 安全桌面附加与服务侧的 SendInput 操作。

use anyhow::Result;
use std::cell::RefCell;
use std::sync::atomic::{AtomicU8, Ordering};
use windows::Win32::Foundation::GENERIC_WRITE;

static MOUSE_BUTTONS: AtomicU8 = AtomicU8::new(0);

struct OwnedDesktop(windows::Win32::System::StationsAndDesktops::HDESK);
impl Drop for OwnedDesktop {
    fn drop(&mut self) {
        // SAFETY: this wrapper owns exactly one desktop handle.
        unsafe {
            windows::Win32::System::StationsAndDesktops::CloseDesktop(self.0).ok();
        }
    }
}

thread_local! {
    // SetThreadDesktop requires the selected desktop handle to remain open.
    static ATTACHED_DESKTOP: RefCell<Option<OwnedDesktop>> = const { RefCell::new(None) };
}

pub(super) fn inject_mouse_move(dx: i32, dy: i32) -> Result<()> {
    anyhow::ensure!(
        dx.unsigned_abs() <= 500 && dy.unsigned_abs() <= 500,
        "鼠标位移过大"
    );
    send_mouse_report(dx, dy, 0)
}

pub(super) fn inject_mouse_move_absolute(x: i32, y: i32) -> Result<()> {
    send_mouse_report_absolute(x, y)
}

pub(super) fn key_to_usage(key: &str) -> Option<u8> {
    let key = key.trim().to_ascii_lowercase();
    if key.len() == 1 {
        let byte = key.as_bytes()[0];
        if byte.is_ascii_lowercase() {
            return Some(byte - b'a' + 0x04);
        }
        if byte.is_ascii_digit() {
            return Some(if byte == b'0' {
                0x27
            } else {
                byte - b'1' + 0x1e
            });
        }
    }
    Some(match key.as_str() {
        "backspace" => 0x2a,
        "enter" => 0x28,
        "escape" => 0x29,
        "tab" => 0x2b,
        "space" => 0x2c,
        "delete" => 0x4c,
        "home" => 0x4a,
        "end" => 0x4d,
        "pageup" => 0x4b,
        "pagedown" => 0x4e,
        "up" => 0x52,
        "down" => 0x51,
        "left" => 0x50,
        "right" => 0x4f,
        "f1" => 0x3a,
        "f2" => 0x3b,
        "f3" => 0x3c,
        "f4" => 0x3d,
        "f5" => 0x3e,
        "f6" => 0x3f,
        "f7" => 0x40,
        "f8" => 0x41,
        "f9" => 0x42,
        "f10" => 0x43,
        "f11" => 0x44,
        "f12" => 0x45,
        _ => return None,
    })
}

pub(super) fn inject_key_event(usage: u8, modifiers: u8, pressed: bool) -> Result<()> {
    anyhow::ensure!((0x04..=0x73).contains(&usage), "不支持的 HID usage");
    use windows::Win32::UI::Input::KeyboardAndMouse::{INPUT, SendInput};
    attach_input_desktop()?;
    let desktop = get_thread_desktop_name().unwrap_or_else(|| "unknown".to_string());
    let (scan, extended) = qinglink_windows::secure_desktop::usage_to_scancode(usage)
        .ok_or_else(|| anyhow::anyhow!("不支持的 HID usage"))?;
    let modifier_scans = [
        (0x01, 0x1d, false),
        (0x02, 0x2a, false),
        (0x04, 0x38, false),
        (0x08, 0x5b, true),
        (0x10, 0x1d, true),
        (0x20, 0x36, false),
        (0x40, 0x38, true),
        (0x80, 0x5c, true),
    ];
    let mut inputs = Vec::with_capacity(9);
    if pressed {
        for (bit, scan, extended) in modifier_scans {
            if modifiers & bit != 0 {
                inputs.push(scan_input(scan, true, extended));
            }
        }
        inputs.push(scan_input(scan, true, extended));
    } else {
        inputs.push(scan_input(scan, false, extended));
        for (bit, scan, extended) in modifier_scans.into_iter().rev() {
            if modifiers & bit != 0 {
                inputs.push(scan_input(scan, false, extended));
            }
        }
    }
    tracing::debug!(%desktop, usage, scan, extended, modifiers, pressed, "injecting HID key event");
    let inserted = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
    anyhow::ensure!(inserted == inputs.len() as u32, "SendInput 键盘事件失败");
    Ok(())
}

pub(super) fn inject_mouse_button(button: &str) -> Result<()> {
    anyhow::ensure!(matches!(button, "left" | "right"), "不支持的鼠标按键");
    inject_mouse_button_state(button, true)?;
    inject_mouse_button_state(button, false)
}

pub(super) fn inject_mouse_button_state(button: &str, pressed: bool) -> Result<()> {
    anyhow::ensure!(matches!(button, "left" | "right"), "不支持的鼠标按键");
    send_mouse_button_event(button, pressed)?;
    let mask = if button == "left" { 1 } else { 2 };
    if pressed {
        MOUSE_BUTTONS.fetch_or(mask, Ordering::SeqCst);
    } else {
        MOUSE_BUTTONS.fetch_and(!mask, Ordering::SeqCst);
    }
    Ok(())
}

pub(super) fn inject_mouse_wheel(delta: i32) -> Result<()> {
    send_mouse_report(0, 0, delta.clamp(-20, 20) as i8)
}

pub(super) fn attach_input_desktop() -> Result<()> {
    attach_owned_input_desktop("attach_input_desktop").map(|_| ())
}

pub(super) fn attach_owned_input_desktop(log_prefix: &str) -> Result<String> {
    use windows::Win32::System::StationsAndDesktops::{
        DESKTOP_ACCESS_FLAGS, DESKTOP_CREATEMENU, DESKTOP_CREATEWINDOW, DESKTOP_ENUMERATE,
        DESKTOP_HOOKCONTROL, DESKTOP_READOBJECTS, DESKTOP_SWITCHDESKTOP, DESKTOP_WRITEOBJECTS,
        OpenInputDesktop, SetThreadDesktop,
    };
    let access = DESKTOP_ACCESS_FLAGS(
        DESKTOP_CREATEMENU.0
            | DESKTOP_CREATEWINDOW.0
            | DESKTOP_ENUMERATE.0
            | DESKTOP_HOOKCONTROL.0
            | DESKTOP_READOBJECTS.0
            | DESKTOP_WRITEOBJECTS.0
            | DESKTOP_SWITCHDESKTOP.0
            | GENERIC_WRITE.0,
    );
    let current_name = get_thread_desktop_name();
    let input_name = get_input_desktop_name();
    if let (Some(current), Some(input)) = (&current_name, &input_name) {
        if current.eq_ignore_ascii_case(input) {
            return Ok(input.clone());
        }
        tracing::info!(from = %current, to = %input, "{log_prefix}: switching thread desktop");
    }
    // The temporary owner is dropped on failure, while success transfers it
    // into the thread-local slot only after SetThreadDesktop succeeds.
    let desktop = OwnedDesktop(unsafe { OpenInputDesktop(Default::default(), false, access)? });
    let name = input_name.unwrap_or_else(|| "unknown".to_string());
    unsafe {
        SetThreadDesktop(desktop.0)?;
    }
    ATTACHED_DESKTOP.with(|slot| slot.replace(Some(desktop)));
    Ok(name)
}

fn get_thread_desktop_name() -> Option<String> {
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::StationsAndDesktops::GetThreadDesktop;
    use windows::Win32::System::Threading::GetCurrentThreadId;
    // SAFETY: the current thread and returned desktop are valid for this call.
    unsafe {
        let desktop = GetThreadDesktop(GetCurrentThreadId()).ok()?;
        desktop_name(HANDLE(desktop.0))
    }
}

fn get_input_desktop_name() -> Option<String> {
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::StationsAndDesktops::{
        DESKTOP_ACCESS_FLAGS, DESKTOP_READOBJECTS, OpenInputDesktop,
    };
    let access = DESKTOP_ACCESS_FLAGS(DESKTOP_READOBJECTS.0);
    // OwnedDesktop closes the temporary OpenInputDesktop result on every path.
    let desktop =
        OwnedDesktop(unsafe { OpenInputDesktop(Default::default(), false, access).ok()? });
    desktop_name(HANDLE(desktop.0.0))
}

fn desktop_name(desktop: windows::Win32::Foundation::HANDLE) -> Option<String> {
    use windows::Win32::System::StationsAndDesktops::{GetUserObjectInformationW, UOI_NAME};
    let mut buffer = [0u16; 128];
    let mut needed = 0u32;
    // SAFETY: buffer is writable and the handle is valid for this query.
    unsafe {
        GetUserObjectInformationW(
            desktop,
            UOI_NAME,
            Some(buffer.as_mut_ptr().cast()),
            (buffer.len() * 2) as u32,
            Some(&mut needed),
        )
        .ok()?;
    }
    let end = buffer
        .iter()
        .position(|unit| *unit == 0)
        .unwrap_or(buffer.len());
    Some(String::from_utf16_lossy(&buffer[..end]))
}

fn send_mouse_button_event(button: &str, pressed: bool) -> Result<()> {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP,
        MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEINPUT, SendInput,
    };
    attach_input_desktop()?;
    let flags = match (button, pressed) {
        ("left", true) => MOUSEEVENTF_LEFTDOWN,
        ("left", false) => MOUSEEVENTF_LEFTUP,
        ("right", true) => MOUSEEVENTF_RIGHTDOWN,
        ("right", false) => MOUSEEVENTF_RIGHTUP,
        _ => anyhow::bail!("不支持的鼠标按键状态"),
    };
    let input = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: 0,
                dy: 0,
                mouseData: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    let inserted = unsafe { SendInput(&[input], std::mem::size_of::<INPUT>() as i32) };
    anyhow::ensure!(inserted == 1, "SendInput 鼠标按键状态失败");
    Ok(())
}

fn send_mouse_report(dx: i32, dy: i32, wheel: i8) -> Result<()> {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_MOVE, MOUSEEVENTF_WHEEL, MOUSEINPUT, SendInput,
    };
    attach_input_desktop()?;
    let mut input = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: dx.clamp(-500, 500),
                dy: dy.clamp(-500, 500),
                mouseData: 0,
                dwFlags: MOUSEEVENTF_MOVE,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    if wheel != 0 {
        input.Anonymous.mi.mouseData = (wheel as i32 * 120) as u32;
        input.Anonymous.mi.dwFlags = MOUSEEVENTF_WHEEL;
    }
    let inserted = unsafe { SendInput(&[input], std::mem::size_of::<INPUT>() as i32) };
    anyhow::ensure!(inserted == 1, "SendInput 鼠标事件失败");
    Ok(())
}

fn send_mouse_report_absolute(x: i32, y: i32) -> Result<()> {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_MOVE, MOUSEINPUT, SendInput,
    };
    attach_input_desktop()?;
    let input = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: x,
                dy: y,
                mouseData: 0,
                dwFlags: MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_MOVE,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    let inserted = unsafe { SendInput(&[input], std::mem::size_of::<INPUT>() as i32) };
    anyhow::ensure!(inserted == 1, "SendInput 绝对定位鼠标事件失败");
    Ok(())
}

#[cfg(target_os = "windows")]
fn scan_input(
    scan: u16,
    pressed: bool,
    extended: bool,
) -> windows::Win32::UI::Input::KeyboardAndMouse::INPUT {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP,
        KEYEVENTF_SCANCODE, VIRTUAL_KEY,
    };
    let mut flags = KEYEVENTF_SCANCODE;
    if !pressed {
        flags |= KEYEVENTF_KEYUP;
    }
    if extended {
        flags |= KEYEVENTF_EXTENDEDKEY;
    }
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(0),
                wScan: scan,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}
