//! 只跟踪已成功送达的按住输入，以便暂停时确定性地清理。
use anyhow::Result;
use std::collections::{BTreeMap, BTreeSet};

/// 记录当前处于按下状态、需要被释放的按键与鼠标键。
#[derive(Default, Debug)]
pub struct HeldInput {
    keys: BTreeMap<u8, u8>,
    buttons: BTreeSet<String>,
}

impl HeldInput {
    /// 记录一次按键状态变化：按下时记住键码与修饰键，抬起时移除。
    pub fn key(&mut self, usage: u8, modifiers: u8, pressed: bool) {
        if pressed {
            self.keys.insert(usage, modifiers);
        } else {
            self.keys.remove(&usage);
        }
    }
    /// 记录一次鼠标键状态变化：按下时记住按键名，抬起时移除。
    pub fn button(&mut self, button: &str, pressed: bool) {
        if pressed {
            self.buttons.insert(button.into());
        } else {
            self.buttons.remove(button);
        }
    }
    /// 释放所有仍处于按下状态的输入，成功后清空记录。
    ///
    /// # Errors
    /// 会话代理不可用或拒绝请求、取不到活动会话，或回放抬起事件
    /// （`key_event` / `mouse_button_state`）失败时返回 `Err`；返回 `Err` 时保留
    /// 已记录的按下状态，便于重试。
    pub fn release(&mut self) -> Result<()> {
        if self.keys.is_empty() && self.buttons.is_empty() {
            return Ok(());
        }
        #[cfg(target_os = "windows")]
        if qinglink_windows::secure_desktop::running_in_session_zero() {
            use qinglink_protocol::service_ipc::ServiceResponse;
            use std::sync::atomic::AtomicBool;
            use std::time::Duration;
            let session =
                unsafe { windows::Win32::System::RemoteDesktop::WTSGetActiveConsoleSessionId() };
            anyhow::ensure!(
                session != 0 && session != u32::MAX,
                "no input agent session for cleanup"
            );
            let keys: Vec<_> = self
                .keys
                .iter()
                .map(|(&key, &modifiers)| (key, modifiers))
                .collect();
            let response: ServiceResponse = qinglink_windows::ipc::request(
                &format!(r"\\.\pipe\QingLinkSessionAgent-{session}"),
                &serde_json::json!({"version":2,"request_id":uuid::Uuid::new_v4().to_string(),
                    "command":"release_inputs","keys":keys,"buttons":self.buttons}),
                Duration::from_secs(3),
                &AtomicBool::new(false),
            )?;
            anyhow::ensure!(response.ok, "input cleanup failed: {}", response.message);
            self.keys.clear();
            self.buttons.clear();
            return Ok(());
        }
        for (&usage, &modifiers) in &self.keys {
            crate::platform::key_event(usage, modifiers, false)?;
        }
        for button in &self.buttons {
            crate::platform::mouse_button_state(button, false)?;
        }
        self.keys.clear();
        self.buttons.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tracks_balanced_state_without_injecting_input() {
        let mut state = HeldInput::default();
        state.key(4, 2, true);
        state.key(4, 0, false);
        state.button("left", true);
        state.button("left", false);
        assert!(state.keys.is_empty());
        assert!(state.buttons.is_empty());
        state.key(5, 1, true);
        state.key(5, 3, true);
        assert_eq!(state.keys.len(), 1);
        assert_eq!(state.keys[&5], 3);
    }
}
