//! 服务启停：调用 SCM 启动/停止服务，并在启动、停止后等待状态收敛。
//!
//! `start` / `stop` 是 Windows 侧原语，平台门面 [`start_and_wait`] 与
//! [`stop_and_wait`] 负责“已处于目标状态则不重复下发控制码”的语义。

use anyhow::{Result, anyhow, ensure};

#[cfg(target_os = "windows")]
use std::{io, ptr};

#[cfg(target_os = "windows")]
use super::ffi::{
    ControlService, ERROR_SERVICE_ALREADY_RUNNING, ERROR_SERVICE_NOT_ACTIVE, SERVICE_CONTROL_STOP,
    SERVICE_QUERY_STATUS, SERVICE_RUNNING, SERVICE_START, SERVICE_STOP, SERVICE_STOPPED,
    ServiceStatusProcess, StartServiceW,
};
#[cfg(target_os = "windows")]
use super::query::{service, state};
#[cfg(target_os = "windows")]
use super::status::{service_is_running, wait_for_running, wait_for_stopped};

#[cfg(target_os = "windows")]
pub(super) fn start(name: &str) -> Result<()> {
    let service = service(name, SERVICE_START | SERVICE_QUERY_STATUS)?;
    // SAFETY: no arguments are passed and the service handle is valid.
    let ok = unsafe { StartServiceW(service.0, 0, ptr::null()) };
    if ok != 0 || state(name) == Some(SERVICE_RUNNING) {
        return Ok(());
    }
    let error = io::Error::last_os_error();
    if error.raw_os_error() == Some(ERROR_SERVICE_ALREADY_RUNNING) {
        Ok(())
    } else {
        Err(anyhow!("启动 Windows 服务失败: {error}"))
    }
}

#[cfg(target_os = "windows")]
pub(super) fn stop(name: &str) -> Result<()> {
    let service = service(name, SERVICE_STOP | SERVICE_QUERY_STATUS)?;
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
    // SAFETY: status points to writable storage and the service handle is valid.
    let ok = unsafe { ControlService(service.0, SERVICE_CONTROL_STOP, &mut status) };
    if ok != 0 || state(name) == Some(SERVICE_STOPPED) {
        return Ok(());
    }
    let error = io::Error::last_os_error();
    if error.raw_os_error() == Some(ERROR_SERVICE_NOT_ACTIVE) {
        Ok(())
    } else {
        Err(anyhow!("停止 Windows 服务失败: {error}"))
    }
}

pub(crate) fn start_and_wait(name: &str) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        if !service_is_running(name) {
            start(name)?;
        }
        wait_for_running(name);
        ensure!(
            service_is_running(name),
            "服务未能启动，请检查 Windows 事件日志"
        );
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = name;
        bail!("Windows 服务仅支持 Windows")
    }
}

pub(crate) fn stop_and_wait(name: &str) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        if service_is_running(name) {
            stop(name)?;
            wait_for_stopped(name);
            ensure!(
                !service_is_running(name),
                "Windows 服务未能停止，请检查服务状态"
            );
        }
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = name;
        Ok(())
    }
}
