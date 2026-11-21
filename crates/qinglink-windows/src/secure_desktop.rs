//! Windows 安全桌面输入支持。
//!
//! 平台操作按职责拆分：桌面/会话探测在 `context`，桌面归属与配对探测在 `desktop`，
//! 文本、HID 按键与鼠标注入在 `text`/`key`/`mouse`，窗口操作在 `window`，安全注意
//! 序列在 `sas`，会话代理传输在 `ipc`。重导出把每个符号保留在原来的
//! `secure_desktop::<符号>` 路径上。

use anyhow::{Result, ensure};

mod context;
mod desktop;
mod hid;
mod ipc;
mod key;
mod mouse;
mod sas;
mod text;
mod window;

pub(crate) use context::active_interactive_session;
pub use context::{SecureDesktopStatus, running_in_session_zero, status};
pub(crate) use desktop::attach_input_desktop;
pub use desktop::pairing_desktop_available;
pub use hid::usage_to_scancode;
pub(crate) use ipc::{
    agent_display_list, agent_hid_key, agent_request, agent_request_message,
    agent_set_display_mode, agent_status_request, agent_window_request,
};
pub use key::send_hid_key;
pub use mouse::{send_mouse_button, send_mouse_move, send_mouse_move_absolute, send_mouse_wheel};
pub use sas::send_secure_attention;
pub use text::{send_secure_ascii, send_text};
pub use window::{enum_windows_light, focus_process, focus_window, switch_window, window_action};

fn validate_secure_ascii(text: &str) -> Result<()> {
    ensure!(
        !text.is_empty() && text.len() <= 256,
        "secure text length is invalid"
    );
    ensure!(
        text.bytes().all(|byte| (0x20..=0x7e).contains(&byte)),
        "secure text must be printable ASCII"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secure_ascii_rejects_control_and_non_ascii_data() {
        assert!(validate_secure_ascii("password").is_ok());
        assert!(validate_secure_ascii("bad\nvalue").is_err());
        assert!(validate_secure_ascii("密碼").is_err());
    }

    #[test]
    fn secure_ascii_rejects_empty_and_oversized_data() {
        assert!(validate_secure_ascii("").is_err());
        assert!(validate_secure_ascii(&"a".repeat(257)).is_err());
    }
}
