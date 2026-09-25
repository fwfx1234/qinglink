// 使用 Win32 API 的 Windows 特定实现

use std::io::{Read, Write};
use std::{mem, sync::Arc};

use anyhow::Context;
use base64::Engine as _;
use windows::Win32::Foundation::{CloseHandle, HANDLE, HWND, LPARAM, WPARAM};
use windows::Win32::Graphics::Dwm::{DWMWA_CLOAKED, DwmGetWindowAttribute};
use windows::Win32::Graphics::Gdi::{
    CDS_FULLSCREEN, ChangeDisplaySettingsExW, DEVMODEW, DISP_CHANGE_SUCCESSFUL,
    DISPLAY_DEVICE_ATTACHED_TO_DESKTOP, DISPLAY_DEVICE_PRIMARY_DEVICE, DISPLAY_DEVICEW,
    DM_DISPLAYFREQUENCY, DM_PELSHEIGHT, DM_PELSWIDTH, ENUM_CURRENT_SETTINGS,
    ENUM_DISPLAY_SETTINGS_FLAGS, ENUM_DISPLAY_SETTINGS_MODE, EnumDisplayDevicesW,
    EnumDisplaySettingsExW, GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO,
    MonitorFromWindow,
};
use windows::Win32::System::ProcessStatus::GetModuleFileNameExW;
use windows::Win32::System::Threading::{
    AttachThreadInput, GetCurrentThreadId, OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ,
};
use windows::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, EnumWindows, GW_OWNER, GWL_EXSTYLE, GetForegroundWindow, GetWindow,
    GetWindowLongW, GetWindowTextW, GetWindowThreadProcessId, HWND_NOTOPMOST, HWND_TOPMOST,
    IsWindow, IsWindowVisible, MSG, PM_NOREMOVE, PeekMessageW, PostMessageW, SW_RESTORE,
    SW_SHOWMAXIMIZED, SW_SHOWMINIMIZED, SW_SHOWNORMAL, SWP_NOSIZE, SWP_NOZORDER, SWP_SHOWWINDOW,
    SetForegroundWindow, SetWindowPos, ShowWindow, WM_CLOSE, WS_EX_APPWINDOW, WS_EX_TOOLWINDOW,
};

use qinglink_protocol::WindowSwitchDirection;
use qinglink_protocol::{
    AppInfo, AudioOutputDevice, AudioOutputsResponse, DisplayInfoResponse, DisplayModeResponse,
    ForegroundResponse, UnlockStatus, VolumeStatus,
};

fn lock_unpoisoned<T>(mutex: &std::sync::Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

// Windows 在生成的 Core Audio 绑定中不暴露默认渲染设备设置器。
// 策略客户端 COM 接口在受支持的 Windows 版本上是稳定的，
// 因此这里仅表示未使用的 vtable 槽位。
windows::core::imp::define_interface!(
    IPolicyConfig,
    IPolicyConfig_Vtbl,
    0xf8679f50_850a_41cf_9c72_430f290290c8
);
windows::core::imp::interface_hierarchy!(IPolicyConfig, windows::core::IUnknown);

#[repr(C)]
pub struct IPolicyConfig_Vtbl {
    base__: windows::core::IUnknown_Vtbl,
    get_mix_format: usize,
    get_device_format: usize,
    reset_device_format: usize,
    set_device_format: usize,
    get_processing_period: usize,
    set_processing_period: usize,
    get_share_mode: usize,
    set_share_mode: usize,
    get_property_value: usize,
    set_property_value: usize,
    set_default_endpoint: unsafe extern "system" fn(
        *mut std::ffi::c_void,
        windows::core::PCWSTR,
        windows::Win32::Media::Audio::ERole,
    ) -> windows::core::HRESULT,
    set_endpoint_visibility: usize,
}

impl IPolicyConfig {
    unsafe fn set_default_endpoint(
        &self,
        device_id: windows::core::PCWSTR,
        role: windows::Win32::Media::Audio::ERole,
    ) -> windows::core::Result<()> {
        unsafe {
            (windows::core::Interface::vtable(self).set_default_endpoint)(
                windows::core::Interface::as_raw(self),
                device_id,
                role,
            )
            .ok()
        }
    }
}

const CLSID_POLICY_CONFIG_CLIENT: windows::core::GUID =
    windows::core::GUID::from_u128(0x870af99c_171d_4f9e_af0d_e63df40c2bc9);

fn to_wide_z(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

fn pwstr_to_string(value: windows::core::PWSTR) -> String {
    unsafe {
        if value.0.is_null() {
            return String::new();
        }
        let mut length = 0usize;
        while *value.0.add(length) != 0 {
            length += 1;
        }
        String::from_utf16_lossy(std::slice::from_raw_parts(value.0, length))
    }
}

fn take_pwstr(value: windows::core::PWSTR) -> String {
    let text = pwstr_to_string(value);
    unsafe {
        windows::Win32::System::Com::CoTaskMemFree(Some(value.0.cast()));
    }
    text
}
