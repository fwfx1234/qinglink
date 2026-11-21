//! Windows 服务实现。
//!
//! 根模块刻意只保留兼容入口与共享的信任常量。业务代码都在各司其职的子模块里。

use anyhow::Result;
use std::sync::atomic::AtomicBool;
use windows::Win32::Foundation::{CloseHandle, HANDLE};

mod authorization;
mod desktop;
mod dispatch;
mod dispatch_input;
mod dispatch_session;
mod focus_cache;
mod input;
mod lifecycle_monitor;
mod management;
mod network;
mod pipe;
mod scm;
mod session_agent;

pub fn run() -> Result<()> {
    scm::run()
}

pub fn run_session_agent_entry(args: &[String]) -> Result<()> {
    session_agent::run_entry(args)
}

const SERVICE_NAME: &str = "QingLinkService";
const PIPE_NAME: &str = r"\\.\pipe\QingLinkControlV2";
const AGENT_PIPE_PREFIX: &str = r"\\.\pipe\QingLinkSessionAgent-";
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
static AGENT_MODE: AtomicBool = AtomicBool::new(false);

pub(super) fn process_image_path(process: HANDLE) -> Result<std::path::PathBuf> {
    use windows::Win32::System::Threading::QueryFullProcessImageNameW;
    let mut buffer = vec![0u16; 32_768];
    let mut length = buffer.len() as u32;
    // SAFETY: process is a live query handle and buffer is writable.
    unsafe {
        QueryFullProcessImageNameW(
            process,
            Default::default(),
            windows::core::PWSTR(buffer.as_mut_ptr()),
            &mut length,
        )?;
    }
    Ok(
        std::path::PathBuf::from(String::from_utf16_lossy(&buffer[..length as usize]))
            .canonicalize()?,
    )
}

pub(super) fn validate_agent_pipe_client(pipe: HANDLE) -> Result<()> {
    use windows::Win32::System::Pipes::{GetNamedPipeClientProcessId, GetNamedPipeClientSessionId};
    use windows::Win32::System::RemoteDesktop::ProcessIdToSessionId;
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
    let mut pid = 0u32;
    let mut pipe_session = 0u32;
    // SAFETY: pipe is borrowed for the duration of these identity queries.
    unsafe {
        GetNamedPipeClientProcessId(pipe, &mut pid)?;
        GetNamedPipeClientSessionId(pipe, &mut pipe_session)?;
    }
    let mut process_session = 0u32;
    // SAFETY: pid came from the kernel pipe endpoint.
    unsafe {
        ProcessIdToSessionId(pid, &mut process_session)?;
    }
    anyhow::ensure!(
        pipe_session == 0 && process_session == 0,
        "agent IPC client is not the LocalSystem service"
    );
    let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid)? };
    let system_identity = authorization::system_process(process);
    let same_binary = process_image_path(process).ok()
        == std::env::current_exe()
            .ok()
            .and_then(|path| path.canonicalize().ok());
    unsafe {
        CloseHandle(process).ok();
    }
    system_identity?;
    anyhow::ensure!(same_binary, "agent IPC client binary is not trusted");
    Ok(())
}
