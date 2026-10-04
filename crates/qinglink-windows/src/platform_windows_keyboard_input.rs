//! 文本与按键注入：Unicode 逐字符文本、按名称按键、按 HID usage 注入扫描码事件。

use std::mem;

use super::input_routing::require_hid_input;
use super::lock_input_pipe::LockInputPipeRequest;
use super::session_lock::is_secure_desktop;

/// 以 Unicode 方式逐字符注入文本，不经过剪贴板。
///
/// # Errors
/// 文本为空或长度超过 64 KiB（“文字长度无效”）；`is_secure_desktop()` 探测失败；
/// 安全桌面路径下透传 `crate::secure_desktop::send_text` 的错误；或 `SendInput`
/// 插入的 Unicode 事件数与预期不一致
/// （“SendInput 发送文字失败（可能被系统或 UAC 拦截）”）时返回错误。
pub fn send_text(text: &str) -> anyhow::Result<()> {
    anyhow::ensure!(!text.is_empty() && text.len() <= 64 * 1024, "文字长度无效");
    // The network server runs as LocalSystem (session 0). Direct SendInput
    // from that session cannot reach the user's desktop, so route text through
    // the per-session agent just like mouse and HID input.
    if crate::secure_desktop::running_in_session_zero() || is_secure_desktop()? {
        return crate::secure_desktop::send_text(text);
    }
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE, SendInput,
        VIRTUAL_KEY,
    };
    // 使用 KEYEVENTF_UNICODE 逐字符注入，不经过剪贴板：
    // 不会破坏用户剪贴板内容，也没有 Ctrl+V 与剪贴板刷新的竞态。
    let mut inputs: Vec<INPUT> = Vec::with_capacity(text.encode_utf16().count() * 2);
    for unit in text.encode_utf16() {
        inputs.push(INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VIRTUAL_KEY(0),
                    wScan: unit,
                    dwFlags: KEYEVENTF_UNICODE,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        });
        inputs.push(INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VIRTUAL_KEY(0),
                    wScan: unit,
                    dwFlags: KEYEVENTF_UNICODE | KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        });
    }
    let inserted = unsafe { SendInput(&inputs, mem::size_of::<INPUT>() as i32) };
    anyhow::ensure!(
        inserted == inputs.len() as u32,
        "SendInput 发送文字失败（可能被系统或 UAC 拦截）"
    );
    Ok(())
}

/// 按名称模拟一次按键（按下后抬起），名称不区分大小写且需在支持的按键表内。
///
/// # Errors
/// `key` 不在支持的按键表内（“不支持的按键: {key}”）；否则透传 [`key_event`] 的错误
/// （不支持的 HID usage、`is_secure_desktop()` 探测失败、安全桌面注入失败，或回退到
/// 输入服务时管道不可用/读写或解析失败、服务返回 `ok == false`）。
pub fn key_press(key: &str) -> anyhow::Result<()> {
    let usage = match key {
        "a" => 0x04,
        "b" => 0x05,
        "c" => 0x06,
        "d" => 0x07,
        "e" => 0x08,
        "f" => 0x09,
        "g" => 0x0a,
        "h" => 0x0b,
        "i" => 0x0c,
        "j" => 0x0d,
        "k" => 0x0e,
        "l" => 0x0f,
        "m" => 0x10,
        "n" => 0x11,
        "o" => 0x12,
        "p" => 0x13,
        "q" => 0x14,
        "r" => 0x15,
        "s" => 0x16,
        "t" => 0x17,
        "u" => 0x18,
        "v" => 0x19,
        "w" => 0x1a,
        "x" => 0x1b,
        "y" => 0x1c,
        "z" => 0x1d,
        "1" => 0x1e,
        "2" => 0x1f,
        "3" => 0x20,
        "4" => 0x21,
        "5" => 0x22,
        "6" => 0x23,
        "7" => 0x24,
        "8" => 0x25,
        "9" => 0x26,
        "0" => 0x27,
        "enter" => 0x28,
        "escape" => 0x29,
        "backspace" => 0x2a,
        "tab" => 0x2b,
        "space" => 0x2c,
        "delete" => 0x4c,
        "right" => 0x4f,
        "left" => 0x50,
        "down" => 0x51,
        "up" => 0x52,
        "home" => 0x4a,
        "end" => 0x4d,
        "pageup" => 0x4b,
        "pagedown" => 0x4e,
        _ => anyhow::bail!("不支持的按键: {}", key),
    };
    key_event(usage, 0, true)?;
    key_event(usage, 0, false)
}

/// 按 HID usage 与修饰键位图注入一次按键事件，`pressed` 区分按下与抬起。
///
/// # Errors
/// `usage` 不在 `0x04..=0x73` 范围内（“不支持的 HID usage”）；`is_secure_desktop()`
/// 探测失败；安全桌面路径下透传 `crate::secure_desktop::send_hid_key` 的错误；否则先
/// 尝试 `SendInput`（HID usage 无对应扫描码或插入事件数不符时视为失败）并回退到输入
/// 服务，服务的管道不可用/读写或解析失败、或返回 `ok == false`
/// （“{code}: {message}”）时返回错误。
pub fn key_event(usage: u8, modifiers: u8, pressed: bool) -> anyhow::Result<()> {
    anyhow::ensure!((0x04..=0x73).contains(&usage), "不支持的 HID usage");
    let request = LockInputPipeRequest {
        version: 1,
        command: "key_event",
        pin: None,
        dx: None,
        dy: None,
        button: None,
        pressed: Some(pressed),
        delta: None,
        usage: Some(usage),
        modifiers: Some(modifiers),
    };
    if crate::secure_desktop::running_in_session_zero() || is_secure_desktop()? {
        return crate::secure_desktop::send_hid_key(usage, modifiers, pressed);
    }
    match send_hid_usage_with_send_input(usage, modifiers, pressed) {
        Ok(()) => Ok(()),
        Err(_) => require_hid_input(&request),
    }
}

fn send_hid_usage_with_send_input(usage: u8, modifiers: u8, pressed: bool) -> anyhow::Result<()> {
    use windows::Win32::UI::Input::KeyboardAndMouse::{INPUT, SendInput};
    let (scan, extended) = hid_usage_to_scan_code(usage)
        .ok_or_else(|| anyhow::anyhow!("HID usage 没有对应的 Windows 扫描码"))?;
    let mut inputs = Vec::new();
    // USB HID modifier bits.  Right-hand modifiers use the extended scan
    // code prefix; without it Windows interprets them as the left key.
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
    if pressed {
        for (bit, modifier_scan, modifier_extended) in modifier_scans {
            if modifiers & bit != 0 {
                inputs.push(scan_code_input(modifier_scan, true, modifier_extended));
            }
        }
        inputs.push(scan_code_input(scan, true, extended));
    } else {
        inputs.push(scan_code_input(scan, false, extended));
        for (bit, modifier_scan, modifier_extended) in modifier_scans.into_iter().rev() {
            if modifiers & bit != 0 {
                inputs.push(scan_code_input(modifier_scan, false, modifier_extended));
            }
        }
    }
    let inserted = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
    anyhow::ensure!(inserted == inputs.len() as u32, "SendInput HID 按键失败");
    Ok(())
}

fn scan_code_input(
    scan: u16,
    pressed: bool,
    extended: bool,
) -> windows::Win32::UI::Input::KeyboardAndMouse::INPUT {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP,
        KEYEVENTF_SCANCODE,
    };
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: Default::default(),
                wScan: scan,
                dwFlags: KEYEVENTF_SCANCODE
                    | if extended {
                        KEYEVENTF_EXTENDEDKEY
                    } else {
                        Default::default()
                    }
                    | if pressed {
                        Default::default()
                    } else {
                        KEYEVENTF_KEYUP
                    },
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn hid_usage_to_scan_code(usage: u8) -> Option<(u16, bool)> {
    const TABLE: &[(u8, u16)] = &[
        (0x04, 0x1e),
        (0x05, 0x30),
        (0x06, 0x2e),
        (0x07, 0x20),
        (0x08, 0x12),
        (0x09, 0x21),
        (0x0a, 0x22),
        (0x0b, 0x23),
        (0x0c, 0x17),
        (0x0d, 0x24),
        (0x0e, 0x25),
        (0x0f, 0x26),
        (0x10, 0x32),
        (0x11, 0x31),
        (0x12, 0x18),
        (0x13, 0x19),
        (0x14, 0x10),
        (0x15, 0x13),
        (0x16, 0x1f),
        (0x17, 0x14),
        (0x18, 0x16),
        (0x19, 0x2f),
        (0x1a, 0x11),
        (0x1b, 0x2d),
        (0x1c, 0x15),
        (0x1d, 0x2c),
        (0x1e, 0x02),
        (0x1f, 0x03),
        (0x20, 0x04),
        (0x21, 0x05),
        (0x22, 0x06),
        (0x23, 0x07),
        (0x24, 0x08),
        (0x25, 0x09),
        (0x26, 0x0a),
        (0x27, 0x0b),
        (0x28, 0x1c),
        (0x29, 0x01),
        (0x2a, 0x0e),
        (0x2b, 0x0f),
        (0x2c, 0x39),
        (0x2d, 0x0c),
        (0x2e, 0x0d),
        (0x2f, 0x1a),
        (0x30, 0x1b),
        (0x31, 0x2b),
        (0x33, 0x27),
        (0x34, 0x28),
        (0x35, 0x29),
        (0x36, 0x33),
        (0x37, 0x34),
        (0x38, 0x35),
        (0x39, 0x3a),
        (0x3a, 0x3b),
        (0x3b, 0x3c),
        (0x3c, 0x3d),
        (0x3d, 0x3e),
        (0x3e, 0x3f),
        (0x3f, 0x40),
        (0x40, 0x41),
        (0x41, 0x42),
        (0x42, 0x43),
        (0x43, 0x44),
        (0x44, 0x57),
        (0x45, 0x58),
        (0x4c, 0x53),
        (0x4f, 0x4d),
        (0x50, 0x4b),
        (0x51, 0x50),
        (0x52, 0x48),
        (0x4a, 0x47),
        (0x4d, 0x4f),
        (0x4b, 0x49),
        (0x4e, 0x51),
    ];
    let scan = TABLE
        .iter()
        .find_map(|(candidate, scan)| (*candidate == usage).then_some(*scan))?;
    Some((scan, matches!(usage, 0x4f..=0x52)))
}
