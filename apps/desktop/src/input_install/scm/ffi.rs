//! advapi32 / kernel32 原始绑定、服务访问位常量与 Windows 服务数据结构。
//!
//! 这里只放 FFI 声明、访问位常量、`#[repr(C)]` 结构与句柄 RAII；句柄打开、
//! 状态查询与配置修改的语义化封装在 [`super::query`]、[`super::control`]、
//! [`super::manage`] 与 [`super::permissions`] 中，SCM 句柄统一由
//! [`ServiceHandle`] 的 `Drop` 关闭。

use std::{ffi::c_void, io};

use anyhow::anyhow;

pub(super) type RawServiceHandle = *mut c_void;

/// 服务已停止（`SERVICE_STOPPED`）。
pub(super) const SERVICE_STOPPED: u32 = 1;
/// 服务正在运行（`SERVICE_RUNNING`）。
pub(super) const SERVICE_RUNNING: u32 = 4;

pub(super) const SC_MANAGER_CONNECT: u32 = 0x0001;
pub(super) const SC_MANAGER_CREATE_SERVICE: u32 = 0x0002;
pub(super) const SERVICE_QUERY_CONFIG: u32 = 0x0001;
pub(super) const SERVICE_CHANGE_CONFIG: u32 = 0x0002;
pub(super) const SERVICE_QUERY_STATUS: u32 = 0x0004;
pub(super) const SERVICE_START: u32 = 0x0010;
pub(super) const SERVICE_STOP: u32 = 0x0020;
pub(super) const DELETE: u32 = 0x0001_0000;
pub(super) const WRITE_DAC: u32 = 0x0004_0000;
pub(super) const SERVICE_ALL_ACCESS: u32 = 0x000F_01FF;
pub(super) const SERVICE_WIN32_OWN_PROCESS: u32 = 0x0000_0010;
pub(super) const SERVICE_AUTO_START: u32 = 0x0000_0002;
pub(super) const SERVICE_ERROR_NORMAL: u32 = 0x0000_0001;
pub(super) const SERVICE_CONTROL_STOP: u32 = 0x0000_0001;
pub(super) const SC_STATUS_PROCESS_INFO: u32 = 0;
pub(super) const DACL_SECURITY_INFORMATION: u32 = 0x0000_0004;
pub(super) const ERROR_SERVICE_ALREADY_RUNNING: i32 = 1056;
pub(super) const ERROR_SERVICE_NOT_ACTIVE: i32 = 1062;

#[repr(C)]
pub(super) struct ServiceStatusProcess {
    pub(super) service_type: u32,
    pub(super) current_state: u32,
    pub(super) controls_accepted: u32,
    pub(super) win32_exit_code: u32,
    pub(super) service_specific_exit_code: u32,
    pub(super) check_point: u32,
    pub(super) wait_hint: u32,
    pub(super) process_id: u32,
    pub(super) service_flags: u32,
}

#[repr(C)]
pub(super) struct ServiceConfig {
    pub(super) service_type: u32,
    pub(super) start_type: u32,
    pub(super) error_control: u32,
    pub(super) binary_path_name: *mut u16,
    pub(super) load_order_group: *mut u16,
    pub(super) tag_id: u32,
    pub(super) dependencies: *mut u16,
    pub(super) start_name: *mut u16,
    pub(super) display_name: *mut u16,
}

pub(super) struct ServiceHandle(pub(super) RawServiceHandle);

impl Drop for ServiceHandle {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: the handle was returned by an SCM API and is closed once here.
            unsafe { CloseServiceHandle(self.0) };
        }
    }
}

#[link(name = "advapi32")]
unsafe extern "system" {
    pub(super) fn OpenSCManagerW(
        machine_name: *const u16,
        database_name: *const u16,
        desired_access: u32,
    ) -> RawServiceHandle;
    pub(super) fn OpenServiceW(
        manager: RawServiceHandle,
        service_name: *const u16,
        desired_access: u32,
    ) -> RawServiceHandle;
    pub(super) fn CloseServiceHandle(handle: RawServiceHandle) -> i32;
    pub(super) fn QueryServiceStatusEx(
        service: RawServiceHandle,
        info_level: u32,
        buffer: *mut u8,
        buffer_size: u32,
        bytes_needed: *mut u32,
    ) -> i32;
    pub(super) fn QueryServiceConfigW(
        service: RawServiceHandle,
        config: *mut ServiceConfig,
        buffer_size: u32,
        bytes_needed: *mut u32,
    ) -> i32;
    pub(super) fn StartServiceW(
        service: RawServiceHandle,
        argument_count: u32,
        arguments: *const *const u16,
    ) -> i32;
    pub(super) fn ControlService(
        service: RawServiceHandle,
        control: u32,
        status: *mut ServiceStatusProcess,
    ) -> i32;
    pub(super) fn CreateServiceW(
        manager: RawServiceHandle,
        service_name: *const u16,
        display_name: *const u16,
        desired_access: u32,
        service_type: u32,
        start_type: u32,
        error_control: u32,
        binary_path_name: *const u16,
        load_order_group: *const u16,
        tag_id: *mut u32,
        dependencies: *const u16,
        service_start_name: *const u16,
        password: *const u16,
    ) -> RawServiceHandle;
    pub(super) fn ChangeServiceConfigW(
        service: RawServiceHandle,
        service_type: u32,
        start_type: u32,
        error_control: u32,
        binary_path_name: *const u16,
        load_order_group: *const u16,
        tag_id: *mut u32,
        dependencies: *const u16,
        service_start_name: *const u16,
        password: *const u16,
        display_name: *const u16,
    ) -> i32;
    pub(super) fn DeleteService(service: RawServiceHandle) -> i32;
    pub(super) fn SetServiceObjectSecurity(
        service: RawServiceHandle,
        security_information: u32,
        security_descriptor: *const c_void,
    ) -> i32;
    pub(super) fn ConvertStringSecurityDescriptorToSecurityDescriptorW(
        string_security_descriptor: *const u16,
        string_sd_revision: u32,
        security_descriptor: *mut *mut c_void,
        security_descriptor_size: *mut u32,
    ) -> i32;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    pub(super) fn LocalFree(memory: *mut c_void) -> *mut c_void;
}

pub(super) fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain([0]).collect()
}

pub(super) fn last_error(context: &str) -> anyhow::Error {
    anyhow!("{context}: {}", io::Error::last_os_error())
}
