use anyhow::{Context, Result};

const VALUE_NAME: &str = "QingLink";

#[cfg(target_os = "windows")]
mod platform {
    use std::{ffi::OsString, os::windows::ffi::OsStrExt};

    use super::*;
    use windows::{
        Win32::System::Registry::{
            HKEY, HKEY_CURRENT_USER, KEY_READ, KEY_SET_VALUE, REG_OPTION_NON_VOLATILE, REG_SZ,
            RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW,
            RegSetValueExW,
        },
        core::PCWSTR,
    };

    const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

    fn wide(value: &str) -> Vec<u16> {
        OsString::from(value).encode_wide().chain([0]).collect()
    }

    fn command() -> Result<String> {
        let executable = std::env::current_exe().context("cannot locate QingLink executable")?;
        Ok(format!("\"{}\" --startup --silent", executable.display()))
    }

    fn open(access: windows::Win32::System::Registry::REG_SAM_FLAGS) -> Option<HKEY> {
        let path = wide(RUN_KEY);
        let mut key = HKEY::default();
        let result = unsafe {
            RegOpenKeyExW(
                HKEY_CURRENT_USER,
                PCWSTR(path.as_ptr()),
                Some(0),
                access,
                &mut key,
            )
        };
        result.is_ok().then_some(key)
    }

    pub fn enable() -> Result<()> {
        let path = wide(RUN_KEY);
        let name = wide(VALUE_NAME);
        let value = wide(&command()?);
        let mut key = HKEY::default();
        unsafe {
            RegCreateKeyExW(
                HKEY_CURRENT_USER,
                PCWSTR(path.as_ptr()),
                Some(0),
                PCWSTR::null(),
                REG_OPTION_NON_VOLATILE,
                KEY_SET_VALUE,
                None,
                &mut key,
                None,
            )
            .ok()
            .context("cannot open Windows autostart registry key")?;
            let bytes = std::slice::from_raw_parts(value.as_ptr().cast::<u8>(), value.len() * 2);
            let result = RegSetValueExW(key, PCWSTR(name.as_ptr()), Some(0), REG_SZ, Some(bytes));
            let _ = RegCloseKey(key);
            result.ok().context("cannot enable QingLink autostart")?;
        }
        Ok(())
    }

    pub fn disable() -> Result<()> {
        let Some(key) = open(KEY_SET_VALUE) else {
            return Ok(());
        };
        let name = wide(VALUE_NAME);
        unsafe {
            let result = RegDeleteValueW(key, PCWSTR(name.as_ptr()));
            let _ = RegCloseKey(key);
            if result.is_err() {
                // Missing values already represent the requested state.
                return Ok(());
            }
        }
        Ok(())
    }

    pub fn is_enabled() -> Result<bool> {
        let Some(key) = open(KEY_READ) else {
            return Ok(false);
        };
        let name = wide(VALUE_NAME);
        let mut value = vec![0_u16; 1024];
        let mut byte_len = (value.len() * 2) as u32;
        let mut kind = REG_SZ;
        let result = unsafe {
            RegQueryValueExW(
                key,
                PCWSTR(name.as_ptr()),
                None,
                Some(&mut kind),
                Some(value.as_mut_ptr().cast::<u8>()),
                Some(&mut byte_len),
            )
        };
        let _ = unsafe { RegCloseKey(key) };
        if result.is_err() {
            return Ok(false);
        }
        let units = (byte_len as usize / 2).saturating_sub(1);
        Ok(String::from_utf16_lossy(&value[..units]) == command()?)
    }
}

#[cfg(not(target_os = "windows"))]
mod platform {
    use super::*;

    pub fn enable() -> Result<()> {
        anyhow::bail!("QingLink autostart is only supported on Windows")
    }
    pub fn disable() -> Result<()> {
        Ok(())
    }
    pub fn is_enabled() -> Result<bool> {
        Ok(false)
    }
}

pub use platform::{disable, enable, is_enabled};
