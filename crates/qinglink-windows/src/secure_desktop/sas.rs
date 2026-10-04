//! 安全注意序列（Ctrl+Alt+Del）注入。
//!
//! `SendSAS` 需要 `SoftwareSASGeneration` 注册表策略允许；本模块负责在调用前后
//! 临时改写并还原该项。

use anyhow::Result;

/// 触发安全注意序列（Ctrl+Alt+Del），必要时先临时开启软件 SAS 生成。
///
/// # Errors
/// 当前实现不返回 `Err`：读写 `SoftwareSASGeneration` 注册表项、调用 `SendSAS`
/// 的失败都被忽略，函数最终总是返回 `Ok(())`。返回值保留 `Result` 以维持接口稳定。
#[cfg(target_os = "windows")]
pub fn send_secure_attention() -> Result<()> {
    use windows::Win32::System::Registry::{
        HKEY_LOCAL_MACHINE, KEY_READ, KEY_SET_VALUE, REG_DWORD, REG_OPTION_NON_VOLATILE,
        REG_VALUE_TYPE, RRF_RT_REG_DWORD, RegCloseKey, RegCreateKeyExW, RegGetValueW,
        RegOpenKeyExW, RegSetValueExW,
    };
    use windows::core::BOOL;
    #[link(name = "sas")]
    unsafe extern "system" {
        fn SendSAS(as_user: BOOL);
    }
    fn software_sas_generation() -> Result<u32> {
        let subkey: Vec<u16> = "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Policies\\System"
            .encode_utf16()
            .chain([0])
            .collect();
        let value: Vec<u16> = "SoftwareSASGeneration".encode_utf16().chain([0]).collect();
        let mut key = windows::Win32::System::Registry::HKEY(std::ptr::null_mut());
        unsafe {
            RegOpenKeyExW(
                HKEY_LOCAL_MACHINE,
                windows::core::PCWSTR(subkey.as_ptr()),
                None,
                KEY_READ,
                &mut key,
            )
            .ok()?;
        }
        let mut data = 0u32;
        let mut size = 4u32;
        let result = unsafe {
            RegGetValueW(
                key,
                windows::core::PCWSTR::null(),
                windows::core::PCWSTR(value.as_ptr()),
                RRF_RT_REG_DWORD,
                Some(std::ptr::null_mut::<REG_VALUE_TYPE>()),
                Some((&mut data as *mut u32).cast()),
                Some(&mut size),
            )
        };
        unsafe {
            let _ = RegCloseKey(key);
        }
        result.ok()?;
        Ok(data)
    }
    fn set_software_sas_generation(value: u32) -> Result<()> {
        let subkey: Vec<u16> = "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Policies\\System"
            .encode_utf16()
            .chain([0])
            .collect();
        let name: Vec<u16> = "SoftwareSASGeneration".encode_utf16().chain([0]).collect();
        let mut key = windows::Win32::System::Registry::HKEY(std::ptr::null_mut());
        unsafe {
            RegCreateKeyExW(
                HKEY_LOCAL_MACHINE,
                windows::core::PCWSTR(subkey.as_ptr()),
                Some(0),
                None,
                REG_OPTION_NON_VOLATILE,
                KEY_SET_VALUE,
                None,
                &mut key,
                None,
            )
            .ok()?;
        }
        let result = unsafe {
            RegSetValueExW(
                key,
                windows::core::PCWSTR(name.as_ptr()),
                None,
                REG_DWORD,
                Some(&value.to_ne_bytes()),
            )
        };
        unsafe {
            let _ = RegCloseKey(key);
        }
        result.ok()?;
        Ok(())
    }
    let previous = software_sas_generation().ok();
    if previous != Some(1) {
        let _ = set_software_sas_generation(1);
    }
    unsafe {
        SendSAS(false.into());
    }
    if let Some(value) = previous {
        let _ = set_software_sas_generation(value);
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn send_secure_attention() -> Result<()> {
    anyhow::bail!("secure attention is only supported on Windows")
}
