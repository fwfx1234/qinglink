//! 按 HID usage 与修饰键位图注入按键。
//!
//! 本地路径把 HID usage 转成 Set 1 扫描码后经 `SendInput` 发送；设置了
//! `QINGLINK_SESSION_AGENT` 时改为经会话代理注入。

use anyhow::{Result, ensure};

/// 按 HID usage 与修饰键位图注入按键，自动转换为扫描码后发送。
///
/// # Errors
/// 设置了 `QINGLINK_SESSION_AGENT` 时：附加输入桌面失败、`usage` 无法映射为扫描码
/// （“unsupported HID usage”）或 `SendInput` 实际插入的事件数与预期不一致时返回
/// 错误；否则经会话代理注入，代理请求失败时返回错误。
#[cfg(target_os = "windows")]
pub fn send_hid_key(usage: u8, modifiers: u8, pressed: bool) -> Result<()> {
    if std::env::var_os("QINGLINK_SESSION_AGENT").is_some() {
        use windows::Win32::UI::Input::KeyboardAndMouse::{INPUT, SendInput};
        let _ = super::attach_input_desktop()?;
        let (scan, extended) = super::hid::usage_to_scancode(usage)
            .ok_or_else(|| anyhow::anyhow!("unsupported HID usage"))?;
        let modifiers_table = [
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
            for (bit, modifier_scan, modifier_extended) in modifiers_table {
                if modifiers & bit != 0 {
                    inputs.push(scan_input(modifier_scan, true, modifier_extended));
                }
            }
            inputs.push(scan_input(scan, true, extended));
        } else {
            inputs.push(scan_input(scan, false, extended));
            for (bit, modifier_scan, modifier_extended) in modifiers_table.into_iter().rev() {
                if modifiers & bit != 0 {
                    inputs.push(scan_input(modifier_scan, false, modifier_extended));
                }
            }
        }
        let inserted = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
        ensure!(
            inserted == inputs.len() as u32,
            "SendInput HID key event failed"
        );
        return Ok(());
    }
    tracing::debug!(
        usage,
        modifiers,
        pressed,
        "sending HID key event via agent pipe"
    );
    super::agent_hid_key(usage, modifiers, pressed)
}

#[cfg(not(target_os = "windows"))]
pub fn send_hid_key(_: u8, _: u8, _: bool) -> Result<()> {
    anyhow::bail!("secure desktop input is only supported on Windows")
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
