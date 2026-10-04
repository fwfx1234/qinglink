//! 安全桌面的文本注入：可打印 ASCII 的按键映射路径与通用 Unicode 路径。
//!
//! `send_secure_ascii` 只接受可打印 ASCII，`send_text` 走 Unicode 注入路径；
//! 两者的校验、代理回退与速率限制语义保持与原实现一致。

use anyhow::{Context, Result, ensure};

/// 向安全桌面注入可打印 ASCII 文本（例如锁屏 PIN），按当前键盘布局映射按键。
///
/// # Errors
/// 以下情况返回错误：文本未通过校验（为空、超过 256 字节或含非可打印 ASCII
/// 字符）；未设置 `QINGLINK_SESSION_AGENT` 时代理请求失败（错误带上下文
/// `SESSION_AGENT_UNAVAILABLE: secure ASCII requires the session agent`）；
/// 附加/切换输入桌面失败（`OpenInputDesktop`/`SetThreadDesktop` 报错）；某个字符
/// 在当前键盘布局下无法映射（`VkKeyScanExW` 返回 -1）；`SendInput` 实际插入的
/// 事件数与预期不一致。
pub fn send_secure_ascii(text: &str) -> Result<()> {
    super::validate_secure_ascii(text)?;
    tracing::info!(length = text.len(), "sending secure ASCII input");
    if std::env::var_os("QINGLINK_SESSION_AGENT").is_none() {
        return super::agent_request("secure_ascii", Some(text), None, None, None)
            .context("SESSION_AGENT_UNAVAILABLE: secure ASCII requires the session agent");
    }
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            GetKeyboardLayout, INPUT, SendInput, VkKeyScanExW,
        };
        let _ = super::attach_input_desktop()?;
        let layout = unsafe { GetKeyboardLayout(0) };
        let mut inputs = Vec::with_capacity(text.len() * 8);
        for byte in text.bytes() {
            let mapped = unsafe { VkKeyScanExW(byte as u16, layout) };
            ensure!(
                mapped != -1,
                "character is unavailable in the active keyboard layout"
            );
            let vk = (mapped as u16 & 0xff) as u16;
            let modifiers = (mapped as u16 >> 8) as u8;
            for (bit, modifier) in [(1u8, 0x10u16), (2, 0x11), (4, 0x12)] {
                if modifiers & bit != 0 {
                    inputs.push(key_input(modifier, false));
                }
            }
            inputs.push(key_input(vk, false));
            inputs.push(key_input(vk, true));
            for (bit, modifier) in [(4u8, 0x12u16), (2, 0x11), (1, 0x10)] {
                if modifiers & bit != 0 {
                    inputs.push(key_input(modifier, true));
                }
            }
        }
        let inserted = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
        ensure!(inserted == inputs.len() as u32, "secure ASCII input failed");
        return Ok(());

        fn key_input(vk: u16, key_up: bool) -> windows::Win32::UI::Input::KeyboardAndMouse::INPUT {
            use windows::Win32::UI::Input::KeyboardAndMouse::{
                INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VIRTUAL_KEY,
            };
            INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VIRTUAL_KEY(vk),
                        wScan: 0,
                        dwFlags: if key_up {
                            KEYEVENTF_KEYUP
                        } else {
                            Default::default()
                        },
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            }
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        anyhow::bail!("secure desktop input is only supported on Windows")
    }
}

/// 以 Unicode 方式向当前桌面注入任意文本，不经过剪贴板。
///
/// # Errors
/// 文本为空或长度超过 64 KiB 时返回“文字长度无效”；未设置
/// `QINGLINK_SESSION_AGENT` 时代理请求失败（错误带上下文
/// `SESSION_AGENT_UNAVAILABLE: text input requires the session agent`）；附加输入
/// 桌面失败；Windows 下 `SendInput` 实际插入的 Unicode 事件数不足。非 Windows
/// 目标恒返回“secure desktop input is only supported on Windows”。
pub fn send_text(text: &str) -> Result<()> {
    ensure!(!text.is_empty() && text.len() <= 64 * 1024, "文字长度无效");
    if std::env::var_os("QINGLINK_SESSION_AGENT").is_none() {
        return super::agent_request("send_text", Some(text), None, None, None)
            .context("SESSION_AGENT_UNAVAILABLE: text input requires the session agent");
    }
    let _ = super::attach_input_desktop()?;
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE,
            SendInput, VIRTUAL_KEY,
        };
        let mut inputs = Vec::with_capacity(text.encode_utf16().count() * 2);
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
        let inserted = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
        ensure!(inserted == inputs.len() as u32, "SendInput 发送文字失败");
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        anyhow::bail!("secure desktop input is only supported on Windows")
    }
}
