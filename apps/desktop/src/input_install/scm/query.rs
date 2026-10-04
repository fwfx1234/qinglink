//! SCM / 服务句柄的打开，以及存在性、当前状态与镜像路径的查询原语。
//!
//! 这里只负责把 Windows 的查询结果翻译成 Rust 值；句柄的生命周期交给
//! [`super::ffi::ServiceHandle`]，平台门面与等待逻辑在 [`super::status`]。

use std::{path::Path, ptr};

use anyhow::{Result, ensure};

use super::ffi::{
    OpenSCManagerW, OpenServiceW, QueryServiceConfigW, QueryServiceStatusEx, SC_MANAGER_CONNECT,
    SC_STATUS_PROCESS_INFO, SERVICE_QUERY_CONFIG, SERVICE_QUERY_STATUS, ServiceConfig,
    ServiceHandle, ServiceStatusProcess, last_error, wide,
};

pub(super) fn manager(access: u32) -> Result<ServiceHandle> {
    // SAFETY: null names select the local SCM and the access mask is constant.
    let handle = unsafe { OpenSCManagerW(ptr::null(), ptr::null(), access) };
    ensure!(
        !handle.is_null(),
        "{}",
        last_error("无法打开 Windows 服务管理器")
    );
    Ok(ServiceHandle(handle))
}

pub(super) fn service(name: &str, access: u32) -> Result<ServiceHandle> {
    let manager = manager(SC_MANAGER_CONNECT)?;
    let name = wide(name);
    // SAFETY: name is a nul-terminated UTF-16 buffer valid for this call.
    let handle = unsafe { OpenServiceW(manager.0, name.as_ptr(), access) };
    ensure!(!handle.is_null(), "{}", last_error("无法打开 Windows 服务"));
    Ok(ServiceHandle(handle))
}

pub(super) fn exists(name: &str) -> bool {
    service(name, SERVICE_QUERY_STATUS).is_ok()
}

pub(super) fn state(name: &str) -> Option<u32> {
    let service = service(name, SERVICE_QUERY_STATUS).ok()?;
    let mut status = ServiceStatusProcess {
        service_type: 0,
        current_state: 0,
        controls_accepted: 0,
        win32_exit_code: 0,
        service_specific_exit_code: 0,
        check_point: 0,
        wait_hint: 0,
        process_id: 0,
        service_flags: 0,
    };
    let mut needed = 0;
    // SAFETY: status points to enough storage for SERVICE_STATUS_PROCESS.
    let ok = unsafe {
        QueryServiceStatusEx(
            service.0,
            SC_STATUS_PROCESS_INFO,
            (&mut status as *mut ServiceStatusProcess).cast(),
            std::mem::size_of::<ServiceStatusProcess>() as u32,
            &mut needed,
        )
    };
    (ok != 0).then_some(status.current_state)
}

pub(super) fn image_path_matches(name: &str, expected: &Path) -> bool {
    let Ok(service) = service(name, SERVICE_QUERY_CONFIG) else {
        return false;
    };
    let mut needed = 0;
    // SAFETY: the first call asks Windows for the required byte count.
    let _ = unsafe { QueryServiceConfigW(service.0, ptr::null_mut(), 0, &mut needed) };
    if needed == 0 {
        return false;
    }
    let words = (needed as usize).div_ceil(std::mem::size_of::<u32>());
    let mut buffer = vec![0u32; words];
    // SAFETY: buffer is writable and at least `needed` bytes long.
    let ok = unsafe {
        QueryServiceConfigW(
            service.0,
            buffer.as_mut_ptr().cast(),
            (buffer.len() * std::mem::size_of::<u32>()) as u32,
            &mut needed,
        )
    };
    if ok == 0 {
        return false;
    }
    // SAFETY: a successful query initialized a SERVICE_CONFIG pointer in buffer.
    let config = unsafe { &*buffer.as_ptr().cast::<ServiceConfig>() };
    let actual = unsafe { read_wide(config.binary_path_name) };
    normalize_path(&actual) == normalize_path(&expected.to_string_lossy())
}

unsafe fn read_wide(pointer: *const u16) -> String {
    if pointer.is_null() {
        return String::new();
    }
    let mut length = 0;
    // SAFETY: caller ensures pointer is valid and null-terminated
    unsafe {
        while *pointer.add(length) != 0 {
            length += 1;
        }
        String::from_utf16_lossy(std::slice::from_raw_parts(pointer, length))
    }
}

fn normalize_path(value: &str) -> String {
    value.trim().trim_matches('"').to_ascii_lowercase()
}
