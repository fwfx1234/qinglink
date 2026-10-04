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
        // Only `--startup` is written: every argument here must exist in
        // `StartupOptions::parse`, because an unknown one makes the process exit
        // with an error before anything starts. An earlier revision also wrote
        // `--silent`, which the parser rejects, so login startup died with
        // "未知启动参数: --silent" and the application never came up.
        Ok(format!("\"{}\" --startup", executable.display()))
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

    /// 在当前用户的 Run 注册表项下写入自启动命令，使 QingLink 随登录启动。
    ///
    /// # Errors
    /// 取不到当前可执行文件路径（“cannot locate QingLink executable”）、
    /// `RegCreateKeyExW` 打开/创建 Run 项失败（“cannot open Windows autostart
    /// registry key”）或 `RegSetValueExW` 写入失败（“cannot enable QingLink
    /// autostart”）时返回错误。
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

    /// 删除 Run 注册表项中的自启动命令；键值本就不存在时视为成功。
    ///
    /// # Errors
    /// 当前实现不返回 `Err`：Run 项打不开、`RegDeleteValueW` 失败（含键值不存在）
    /// 都被视为已达到目标状态并返回 `Ok(())`。返回值保留 `Result` 以维持接口稳定。
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

    /// 读取 Run 注册表项，并判断其中的自启动命令是否与当前可执行文件一致。
    ///
    /// # Errors
    /// 仅在取不到当前可执行文件路径（`command()` 返回“cannot locate QingLink
    /// executable”）时返回错误。Run 项打不开或 `RegQueryValueExW` 查询失败都返回
    /// `Ok(false)`。
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

    /// 非 Windows 平台的占位实现，始终返回不支持错误。
    pub fn enable() -> Result<()> {
        anyhow::bail!("QingLink autostart is only supported on Windows")
    }
    /// 非 Windows 平台的占位实现，无操作直接成功。
    pub fn disable() -> Result<()> {
        Ok(())
    }
    /// 非 Windows 平台的占位实现，始终报告未启用自启动。
    pub fn is_enabled() -> Result<bool> {
        Ok(false)
    }
}

pub use platform::{disable, enable, is_enabled};
