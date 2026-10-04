//! 服务对象 DACL（SDDL）收紧。
//!
//! 安全语义：SDDL 字符串与 `WRITE_DAC` 访问位不得改动——服务对象只对
//! INTERACTIVE、Administrators 与 LocalSystem 放行，且由转换 API 分配的描述符
//! 必须用 `LocalFree` 释放。

use anyhow::{Result, ensure};

#[cfg(target_os = "windows")]
use std::ptr;

#[cfg(target_os = "windows")]
use super::ffi::{
    ConvertStringSecurityDescriptorToSecurityDescriptorW, DACL_SECURITY_INFORMATION, LocalFree,
    SetServiceObjectSecurity, WRITE_DAC, last_error, wide,
};
#[cfg(target_os = "windows")]
use super::query::service;

#[cfg(target_os = "windows")]
pub(super) fn set_permissions(name: &str) -> Result<()> {
    let service = service(name, WRITE_DAC)?;
    let descriptor = wide(
        "D:(A;;CCLCSWRPWPLOCRRC;;;IU)(A;;CCDCLCSWRPWPDTLOCRSDRCWDWO;;;BA)(A;;CCLCSWRPWPLOCRRC;;;SY)",
    );
    let mut security_descriptor = ptr::null_mut();
    let mut descriptor_size = 0;
    // SAFETY: output pointers refer to writable local variables and descriptor is valid UTF-16.
    let converted = unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            descriptor.as_ptr(),
            1,
            &mut security_descriptor,
            &mut descriptor_size,
        )
    };
    ensure!(
        converted != 0,
        "{}",
        last_error("无法解析 Windows 服务 ACL")
    );
    // SAFETY: descriptor was allocated by LocalAlloc through the conversion API.
    let result = unsafe {
        SetServiceObjectSecurity(service.0, DACL_SECURITY_INFORMATION, security_descriptor)
    };
    // SAFETY: the conversion API requires LocalFree for its output allocation.
    unsafe { LocalFree(security_descriptor) };
    ensure!(result != 0, "{}", last_error("无法设置 Windows 服务 ACL"));
    Ok(())
}

pub(crate) fn configure_service_permissions(name: &str) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        set_permissions(name)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = name;
        bail!("Windows 服务仅支持 Windows")
    }
}
