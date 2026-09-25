use anyhow::{Context, Result};
use std::time::Duration;

#[cfg(target_os = "windows")]
const ACTIVATE_MESSAGE_ID: u32 = 0x0401;

#[cfg(target_os = "windows")]
pub(crate) const WINDOW_TITLE: &str = "轻联 QingLink";

#[cfg(target_os = "windows")]
pub struct SingleInstance {
    handle: windows::Win32::Foundation::HANDLE,
}

#[cfg(target_os = "windows")]
impl SingleInstance {
    pub fn acquire() -> Result<Option<Self>> {
        use windows::{
            Win32::{
                Foundation::{ERROR_ALREADY_EXISTS, GetLastError},
                System::Threading::CreateMutexW,
            },
            core::w,
        };
        let handle = unsafe { CreateMutexW(None, false, w!("Local\\QingLink.Desktop.Instance")) }
            .context("cannot create QingLink instance mutex")?;
        if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
            let _ = unsafe { windows::Win32::Foundation::CloseHandle(handle) };
            return Ok(None);
        }
        Ok(Some(Self { handle }))
    }
}

#[cfg(target_os = "windows")]
impl Drop for SingleInstance {
    fn drop(&mut self) {
        let _ = unsafe { windows::Win32::Foundation::CloseHandle(self.handle) };
    }
}

#[cfg(target_os = "windows")]
pub fn activate_existing() -> Result<()> {
    use windows::Win32::{
        Foundation::{LPARAM, WPARAM},
        UI::WindowsAndMessaging::{
            FindWindowW, PostMessageW, SW_RESTORE, SetForegroundWindow, ShowWindow,
        },
    };

    let title: Vec<u16> = WINDOW_TITLE.encode_utf16().chain([0]).collect();

    if let Ok(hwnd) = unsafe { FindWindowW(None, windows::core::PCWSTR(title.as_ptr())) } {
        if !hwnd.is_invalid() {
            unsafe {
                let _ = ShowWindow(hwnd, SW_RESTORE);
                let _ = SetForegroundWindow(hwnd);
                let _ = PostMessageW(Some(hwnd), ACTIVATE_MESSAGE_ID, WPARAM(0), LPARAM(0));
            }
            return Ok(());
        }
    }

    std::thread::sleep(Duration::from_millis(500));

    if let Ok(hwnd) = unsafe { FindWindowW(None, windows::core::PCWSTR(title.as_ptr())) } {
        if !hwnd.is_invalid() {
            unsafe {
                let _ = ShowWindow(hwnd, SW_RESTORE);
                let _ = SetForegroundWindow(hwnd);
                let _ = PostMessageW(Some(hwnd), ACTIVATE_MESSAGE_ID, WPARAM(0), LPARAM(0));
            }
        }
    }

    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn activate_existing() -> Result<()> {
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub struct SingleInstance;

#[cfg(not(target_os = "windows"))]
impl SingleInstance {
    pub fn acquire() -> Result<Option<Self>> {
        Ok(Some(Self))
    }
}

#[cfg(target_os = "windows")]
pub mod activation {
    pub const MESSAGE_ID: u32 = super::ACTIVATE_MESSAGE_ID;
}
