//! 注册表只读 FFI 封装：读取 HKLM 下的字符串与 DWORD 值。

use super::to_wide_z;

pub(super) fn read_winlogon_value(value_name: &str) -> Option<String> {
    read_hklm_string(
        r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Winlogon",
        value_name,
    )
}

pub(super) fn read_hklm_string(subkey: &str, value_name: &str) -> Option<String> {
    use windows::Win32::System::Registry::{
        HKEY, HKEY_LOCAL_MACHINE, KEY_READ, REG_SZ, RegCloseKey, RegOpenKeyExW, RegQueryValueExW,
    };

    unsafe {
        let subkey = to_wide_z(subkey);
        let mut hkey = HKEY::default();
        if RegOpenKeyExW(
            HKEY_LOCAL_MACHINE,
            windows::core::PCWSTR(subkey.as_ptr()),
            Some(0),
            KEY_READ,
            &mut hkey,
        )
        .is_err()
        {
            return None;
        }

        let name = to_wide_z(value_name);
        let mut kind = REG_SZ;
        let mut buf = vec![0u16; 512];
        let mut len = (buf.len() * 2) as u32;
        let result = RegQueryValueExW(
            hkey,
            windows::core::PCWSTR(name.as_ptr()),
            None,
            Some(&mut kind),
            Some(buf.as_mut_ptr() as *mut u8),
            Some(&mut len),
        );
        let _ = RegCloseKey(hkey);
        if result.is_err() || kind != REG_SZ || len < 2 {
            return None;
        }

        let chars = (len as usize).saturating_sub(2) / 2;
        Some(String::from_utf16_lossy(&buf[..chars]))
    }
}

pub(super) fn read_hklm_dword(subkey: &str, value_name: &str) -> Option<u32> {
    use windows::Win32::System::Registry::{
        HKEY, HKEY_LOCAL_MACHINE, KEY_READ, REG_DWORD, RegCloseKey, RegOpenKeyExW, RegQueryValueExW,
    };

    unsafe {
        let subkey = to_wide_z(subkey);
        let mut hkey = HKEY::default();
        if RegOpenKeyExW(
            HKEY_LOCAL_MACHINE,
            windows::core::PCWSTR(subkey.as_ptr()),
            Some(0),
            KEY_READ,
            &mut hkey,
        )
        .is_err()
        {
            return None;
        }

        let name = to_wide_z(value_name);
        let mut kind = REG_DWORD;
        let mut value = 0u32;
        let mut len = std::mem::size_of::<u32>() as u32;
        let result = RegQueryValueExW(
            hkey,
            windows::core::PCWSTR(name.as_ptr()),
            None,
            Some(&mut kind),
            Some(&mut value as *mut u32 as *mut u8),
            Some(&mut len),
        );
        let _ = RegCloseKey(hkey);
        if result.is_err() || kind != REG_DWORD || len != std::mem::size_of::<u32>() as u32 {
            return None;
        }

        Some(value)
    }
}
