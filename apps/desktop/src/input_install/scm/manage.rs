//! 服务的创建/更新与删除，即输入服务的安装与卸载流程。
//!
//! Windows 侧原语 [`create_or_configure`] 与 [`delete`] 只负责 SCM 对象本身；
//! ACL 收紧由 [`super::permissions`] 单独完成，调用顺序由安装流程决定。

use std::path::Path;

use anyhow::{Result, ensure};

#[cfg(target_os = "windows")]
use std::ptr;

#[cfg(target_os = "windows")]
use super::control::stop;
#[cfg(target_os = "windows")]
use super::ffi::{
    ChangeServiceConfigW, CreateServiceW, DELETE, DeleteService, SC_MANAGER_CREATE_SERVICE,
    SERVICE_ALL_ACCESS, SERVICE_AUTO_START, SERVICE_CHANGE_CONFIG, SERVICE_ERROR_NORMAL,
    SERVICE_STOPPED, SERVICE_WIN32_OWN_PROCESS, ServiceHandle, last_error, wide,
};
#[cfg(target_os = "windows")]
use super::query::{exists, manager, service, state};

#[cfg(target_os = "windows")]
pub(super) fn create_or_configure(name: &str, binary_path: &Path) -> Result<()> {
    let name_wide = wide(name);
    let display_name = wide("QingLink System Service");
    let binary_path = wide(&format!("\"{}\"", binary_path.display()));
    let local_system = wide("LocalSystem");
    if exists(name) {
        let service = service(name, SERVICE_CHANGE_CONFIG)?;
        // SAFETY: all UTF-16 buffers remain alive for the duration of this call.
        let ok = unsafe {
            ChangeServiceConfigW(
                service.0,
                SERVICE_WIN32_OWN_PROCESS,
                SERVICE_AUTO_START,
                SERVICE_ERROR_NORMAL,
                binary_path.as_ptr(),
                ptr::null(),
                ptr::null_mut(),
                ptr::null(),
                local_system.as_ptr(),
                ptr::null(),
                display_name.as_ptr(),
            )
        };
        ensure!(ok != 0, "{}", last_error("无法更新 Windows 服务配置"));
        return Ok(());
    }
    let manager = manager(SC_MANAGER_CREATE_SERVICE)?;
    // SAFETY: all pointers refer to nul-terminated buffers alive during this call.
    let handle = unsafe {
        CreateServiceW(
            manager.0,
            name_wide.as_ptr(),
            display_name.as_ptr(),
            SERVICE_ALL_ACCESS,
            SERVICE_WIN32_OWN_PROCESS,
            SERVICE_AUTO_START,
            SERVICE_ERROR_NORMAL,
            binary_path.as_ptr(),
            ptr::null(),
            ptr::null_mut(),
            ptr::null(),
            local_system.as_ptr(),
            ptr::null(),
        )
    };
    ensure!(!handle.is_null(), "{}", last_error("无法创建 Windows 服务"));
    drop(ServiceHandle(handle));
    Ok(())
}

#[cfg(target_os = "windows")]
pub(super) fn delete(name: &str) -> Result<()> {
    if !exists(name) {
        return Ok(());
    }
    if state(name).is_some_and(|value| value != SERVICE_STOPPED) {
        stop(name)?;
    }
    let service = service(name, DELETE)?;
    // SAFETY: service is a valid handle opened with DELETE access.
    let ok = unsafe { DeleteService(service.0) };
    ensure!(ok != 0, "{}", last_error("无法删除 Windows 服务"));
    Ok(())
}

pub(crate) fn install_or_configure(name: &str, binary_path: &Path) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        create_or_configure(name, binary_path)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (name, binary_path);
        bail!("Windows 服务仅支持 Windows")
    }
}

pub(crate) fn delete_service(name: &str) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        delete(name)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = name;
        bail!("Windows 服务仅支持 Windows")
    }
}
