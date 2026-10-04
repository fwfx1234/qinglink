//! 配对与远程输入的共享安全状态。
//!
//! 桌面代理是权威方：它在认证了服务 IPC 对端之后调用
//! [`ControlState::observe_desktop`]。HTTP 服务器只持有短期租约并执行最后的平台
//! 会话检查；在判断配对是否可用时，它绝不探测或注入输入。

use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

const DESKTOP_LEASE: Duration = Duration::from_secs(2);

#[derive(Clone, Debug)]
struct DesktopLease {
    session: u32,
    unlocked: bool,
    observed_at: Instant,
}

#[derive(Debug)]
struct ControlInner {
    desktop: Option<DesktopLease>,
    network_error: Option<String>,
}

/// Process-wide pause and desktop-observation state.
#[derive(Debug)]
pub struct ControlState {
    paused: AtomicBool,
    inner: Mutex<ControlInner>,
    /// Serializes input with pause/desktop cleanup operations.
    pub input_gate: Mutex<()>,
}

impl Default for ControlState {
    fn default() -> Self {
        Self {
            paused: AtomicBool::new(false),
            inner: Mutex::new(ControlInner {
                desktop: None,
                network_error: None,
            }),
            input_gate: Mutex::new(()),
        }
    }
}

impl ControlState {
    /// 服务当前是否处于暂停状态。
    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::Acquire)
    }

    /// 设置暂停标志；持久化与监听器收敛由调用方负责。
    pub fn set_paused(&self, paused: bool) {
        self.paused.store(paused, Ordering::Release);
    }

    /// 记录或清除当前的网络错误信息。
    pub fn set_network_error(&self, error: Option<String>) {
        if let Ok(mut state) = self.inner.lock() {
            state.network_error = error;
        }
    }

    /// 返回最近一次记录的监听器错误。
    pub fn network_error(&self) -> Option<String> {
        self.inner
            .lock()
            .ok()
            .and_then(|state| state.network_error.clone())
    }

    /// Records an authenticated desktop observation from service IPC.
    pub fn observe_desktop(&self, session: u32, unlocked: bool) {
        if let Ok(mut state) = self.inner.lock() {
            state.desktop = Some(DesktopLease {
                session,
                unlocked,
                observed_at: Instant::now(),
            });
        }
    }

    /// 清除桌面租约，使配对立即变得不可用。
    pub fn clear_pairing_desktop(&self) {
        if let Ok(mut state) = self.inner.lock() {
            state.desktop = None;
        }
    }

    /// Returns true only for a recent, unlocked authenticated desktop lease.
    ///
    /// Non-Windows builds intentionally fail closed.  On Windows the active
    /// console session and WTS state are independently checked here.  This
    /// check does not call secure-desktop status/probe APIs or SendInput.
    pub fn pairing_available(&self) -> bool {
        if self.is_paused() {
            return false;
        }
        let lease = match self.inner.lock() {
            Ok(state) => state.desktop.clone(),
            Err(_) => return false,
        };
        let Some(lease) = lease else { return false };
        if !lease.unlocked || lease.observed_at.elapsed() > DESKTOP_LEASE {
            return false;
        }

        #[cfg(target_os = "windows")]
        {
            windows_desktop_available(lease.session)
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = lease;
            false
        }
    }
}

#[cfg(target_os = "windows")]
fn windows_desktop_available(session: u32) -> bool {
    use windows::Win32::System::RemoteDesktop::{
        WTS_SESSIONSTATE_UNLOCK, WTSActive, WTSFreeMemory, WTSGetActiveConsoleSessionId,
        WTSINFOEXW, WTSQuerySessionInformationW, WTSSessionInfoEx,
    };

    if session == 0 || session == u32::MAX || unsafe { WTSGetActiveConsoleSessionId() } != session {
        return false;
    }

    let mut buffer = windows::core::PWSTR::null();
    let mut bytes = 0u32;
    if unsafe {
        WTSQuerySessionInformationW(None, session, WTSSessionInfoEx, &mut buffer, &mut bytes)
    }
    .is_err()
        || buffer.is_null()
    {
        return false;
    }

    let result = (|| {
        if (bytes as usize) < std::mem::size_of::<WTSINFOEXW>() {
            return false;
        }
        let info = unsafe { &*(buffer.0 as *const WTSINFOEXW) };
        if info.Level != 1 {
            return false;
        }
        let (session_state, session_flags) = unsafe {
            (
                info.Data.WTSInfoExLevel1.SessionState,
                info.Data.WTSInfoExLevel1.SessionFlags,
            )
        };
        if session_state != WTSActive {
            return false;
        }
        session_flags as u32 == WTS_SESSIONSTATE_UNLOCK
    })();
    unsafe { WTSFreeMemory(buffer.0.cast()) };
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn default_is_paused_false_and_fail_closed_without_lease() {
        let state = ControlState::default();
        assert!(!state.is_paused());
        assert!(!state.pairing_available());
    }

    #[test]
    fn pause_and_network_error_are_independent_state() {
        let state = ControlState::default();
        state.set_network_error(Some("offline".into()));
        assert_eq!(state.network_error().as_deref(), Some("offline"));
        state.set_paused(true);
        assert!(state.is_paused());
        state.set_paused(false);
        assert!(!state.is_paused());
        state.set_network_error(None);
        assert_eq!(state.network_error(), None);
    }

    #[test]
    fn clear_removes_observed_lease() {
        let state = ControlState::default();
        state.observe_desktop(7, true);
        state.clear_pairing_desktop();
        assert!(!state.pairing_available());
    }

    #[test]
    fn observation_is_replaced_and_input_gate_is_serializable() {
        let state = std::sync::Arc::new(ControlState::default());
        state.observe_desktop(1, false);
        state.observe_desktop(2, true);
        let guard = state.input_gate.lock().expect("gate");
        let state2 = std::sync::Arc::clone(&state);
        let joined = thread::spawn(move || {
            state2.observe_desktop(3, true);
        });
        // The observation lock is separate from input_gate: IPC observation is
        // safe while an input operation is serialized by the caller.
        joined.join().expect("observation");
        drop(guard);
        state.clear_pairing_desktop();
        assert!(!state.pairing_available());
    }
}
