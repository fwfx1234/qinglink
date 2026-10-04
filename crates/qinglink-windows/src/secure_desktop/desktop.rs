//! 桌面归属与直接输入注入。
//!
//! 本模块只保留桌面归属与附加：`OwnedDesktop` 的持有与释放、线程桌面切换，以及
//! 配对桌面的轻量探测。文本、HID、鼠标与窗口注入分别位于 `text`、`key`、`mouse`、
//! `window`，安全注意序列位于 `sas`。

use anyhow::{Context, Result};

#[cfg(target_os = "windows")]
use std::cell::RefCell;

#[cfg(target_os = "windows")]
struct OwnedDesktop(windows::Win32::System::StationsAndDesktops::HDESK);

#[cfg(target_os = "windows")]
impl Drop for OwnedDesktop {
    fn drop(&mut self) {
        unsafe {
            windows::Win32::System::StationsAndDesktops::CloseDesktop(self.0).ok();
        }
    }
}

#[cfg(target_os = "windows")]
thread_local! {
    // The thread remains attached to this desktop; retain one owner and replace
    // it only after a new SetThreadDesktop call has succeeded.
    static ATTACHED_DESKTOP: RefCell<Option<OwnedDesktop>> = const { RefCell::new(None) };
}

#[cfg(target_os = "windows")]
fn desktop_name(desktop: windows::Win32::System::StationsAndDesktops::HDESK) -> Result<String> {
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::StationsAndDesktops::{GetUserObjectInformationW, UOI_NAME};
    let mut buffer = [0u16; 128];
    let mut needed = 0u32;
    unsafe {
        GetUserObjectInformationW(
            HANDLE(desktop.0),
            UOI_NAME,
            Some(buffer.as_mut_ptr().cast()),
            (buffer.len() * 2) as u32,
            Some(&mut needed),
        )?;
    }
    let end = buffer
        .iter()
        .position(|unit| *unit == 0)
        .unwrap_or(buffer.len());
    Ok(String::from_utf16_lossy(&buffer[..end]))
}

#[cfg(target_os = "windows")]
pub(crate) fn attach_input_desktop() -> Result<String> {
    use windows::Win32::Foundation::GENERIC_WRITE;
    use windows::Win32::System::StationsAndDesktops::{
        DESKTOP_ACCESS_FLAGS, DESKTOP_CREATEMENU, DESKTOP_CREATEWINDOW, DESKTOP_ENUMERATE,
        DESKTOP_HOOKCONTROL, DESKTOP_READOBJECTS, DESKTOP_SWITCHDESKTOP, DESKTOP_WRITEOBJECTS,
        GetThreadDesktop, OpenInputDesktop, SetThreadDesktop,
    };
    use windows::Win32::System::Threading::GetCurrentThreadId;
    let access = DESKTOP_ACCESS_FLAGS(
        DESKTOP_CREATEWINDOW.0
            | DESKTOP_CREATEMENU.0
            | DESKTOP_HOOKCONTROL.0
            | DESKTOP_ENUMERATE.0
            | DESKTOP_READOBJECTS.0
            | DESKTOP_SWITCHDESKTOP.0
            | DESKTOP_WRITEOBJECTS.0
            | GENERIC_WRITE.0,
    );
    unsafe {
        let input = OwnedDesktop(
            OpenInputDesktop(Default::default(), false, access)
                .context("OpenInputDesktop failed")?,
        );
        let name = desktop_name(input.0)?;
        let current = GetThreadDesktop(GetCurrentThreadId())?;
        let current_name = desktop_name(current).unwrap_or_default();
        if current_name.eq_ignore_ascii_case(&name) {
            tracing::debug!(desktop = %name, "already attached to input desktop");
            drop(input);
        } else {
            tracing::info!(from = %current_name, to = %name, "switching thread desktop for secure input");
            SetThreadDesktop(input.0).context("SetThreadDesktop failed")?;
            ATTACHED_DESKTOP.with(|slot| {
                slot.replace(Some(input));
            });
        }
        Ok(name)
    }
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn attach_input_desktop() -> Result<String> {
    anyhow::bail!("secure desktop input is only supported on Windows")
}

/// Checks the desktop used for pairing without probing or injecting input.
pub fn pairing_desktop_available() -> bool {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::System::RemoteDesktop::ProcessIdToSessionId;
        use windows::Win32::System::RemoteDesktop::WTSGetActiveConsoleSessionId;
        use windows::Win32::System::StationsAndDesktops::{DESKTOP_READOBJECTS, OpenInputDesktop};
        use windows::Win32::System::Threading::GetCurrentProcessId;

        let mut current_session = u32::MAX;
        if unsafe { ProcessIdToSessionId(GetCurrentProcessId(), &mut current_session) }.is_err() {
            return false;
        }
        let active_session = unsafe { WTSGetActiveConsoleSessionId() };
        if current_session == 0
            || active_session == 0
            || active_session == u32::MAX
            || current_session != active_session
        {
            return false;
        }
        let desktop =
            match unsafe { OpenInputDesktop(Default::default(), false, DESKTOP_READOBJECTS) } {
                Ok(desktop) => OwnedDesktop(desktop),
                Err(_) => return false,
            };
        match desktop_name(desktop.0) {
            Ok(name) => name.eq_ignore_ascii_case("Default"),
            Err(_) => false,
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}
