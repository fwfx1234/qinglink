use crate::{Result, WindowsCapabilityError};

pub fn ensure_supported() -> Result<()> {
    super::is_supported()
        .then_some(())
        .ok_or(WindowsCapabilityError::Unsupported("window control"))
}

#[cfg(target_os = "windows")]
pub fn hide_native_window(handle: isize) {
    use windows::Win32::{
        Foundation::HWND,
        UI::WindowsAndMessaging::{SW_HIDE, ShowWindow},
    };
    unsafe {
        let _ = ShowWindow(HWND(handle as *mut _), SW_HIDE);
    }
}

#[cfg(target_os = "windows")]
pub fn show_native_window(handle: isize) {
    use windows::Win32::{
        Foundation::HWND,
        UI::WindowsAndMessaging::{SW_RESTORE, SetForegroundWindow, ShowWindow},
    };
    let hwnd = HWND(handle as *mut _);
    unsafe {
        let _ = ShowWindow(hwnd, SW_RESTORE);
        let _ = SetForegroundWindow(hwnd);
    }
}

#[cfg(target_os = "windows")]
pub fn find_qinglink_window() -> Option<isize> {
    use windows::Win32::UI::WindowsAndMessaging::FindWindowW;
    let title: Vec<u16> = crate::single_instance::WINDOW_TITLE
        .encode_utf16()
        .chain([0])
        .collect();
    let hwnd = unsafe { FindWindowW(None, windows::core::PCWSTR(title.as_ptr())) }.ok()?;
    if hwnd.is_invalid() {
        None
    } else {
        Some(hwnd.0 as isize)
    }
}

#[cfg(not(target_os = "windows"))]
pub fn find_qinglink_window() -> Option<isize> {
    None
}
