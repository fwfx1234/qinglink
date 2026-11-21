//! 会话代理进程的监管与 LocalSystem 令牌移交。

use anyhow::{Context, Result};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;
use windows::Win32::Foundation::{CloseHandle, HANDLE};

pub(super) struct AgentProcess {
    process_handle: Option<HANDLE>,
}
impl AgentProcess {
    pub(super) fn exited(&mut self) -> bool {
        if let Some(handle) = self.process_handle {
            use windows::Win32::System::Threading::GetExitCodeProcess;
            let mut code = 0u32;
            // SAFETY: process_handle is owned and valid until this method returns.
            return unsafe { GetExitCodeProcess(handle, &mut code).is_ok() && code != 259 };
        }
        true
    }
    pub(super) fn terminate(&mut self) {
        if let Some(handle) = self.process_handle.take() {
            // SAFETY: this owner is the only code closing/terminating the process.
            let _ = unsafe { windows::Win32::System::Threading::TerminateProcess(handle, 1) };
            unsafe {
                let _ = CloseHandle(handle);
            }
        }
    }
}
impl Drop for AgentProcess {
    fn drop(&mut self) {
        self.terminate();
    }
}

pub(super) fn run(stop: &Arc<AtomicBool>) {
    let mut active = u32::MAX;
    let mut child = None;
    tracing::info!("session agent supervisor started");
    while !stop.load(Ordering::Acquire) {
        let session = super::desktop::active_interactive_session().unwrap_or_else(|| unsafe {
            windows::Win32::System::RemoteDesktop::WTSGetActiveConsoleSessionId()
        });
        let exited = child.as_mut().is_some_and(AgentProcess::exited);
        if session != active || exited {
            if let Some(mut process) = child.take() {
                process.terminate();
            }
            active = session;
            if session != u32::MAX {
                match launch(session) {
                    Ok(agent) => {
                        tracing::info!(session_id = session, "Session agent launched");
                        child = Some(agent);
                    }
                    Err(error) => {
                        tracing::error!(session_id = session, %error, "Failed to launch session agent")
                    }
                }
            }
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    if let Some(mut process) = child {
        process.terminate();
    }
    tracing::info!("session agent supervisor stopping");
}

fn launch(session_id: u32) -> Result<AgentProcess> {
    let executable = std::env::current_exe().context("cannot locate service executable")?;
    use windows::Win32::System::Threading::{
        CreateProcessAsUserW, PROCESS_CREATION_FLAGS, PROCESS_INFORMATION, STARTUPINFOW,
    };
    let token = find_winlogon_token(session_id).unwrap_or_default();
    if token.is_invalid() {
        anyhow::bail!("winlogon token not found");
    }
    if let Ok(level) = token_integrity_level(token) {
        tracing::info!(session_id, level, "Session agent token acquired");
    }
    let mut command = format!(
        "\"{}\" --session-agent --session {}",
        executable.display(),
        session_id
    )
    .encode_utf16()
    .chain([0])
    .collect::<Vec<u16>>();
    let mut startup = STARTUPINFOW {
        cb: std::mem::size_of::<STARTUPINFOW>() as u32,
        ..Default::default()
    };
    let desktop_name: Vec<u16> = "WinSta0\\Default".encode_utf16().chain([0]).collect();
    startup.lpDesktop = windows::core::PWSTR(desktop_name.as_ptr() as *mut u16);
    let mut info = PROCESS_INFORMATION::default();
    // SAFETY: all pointers refer to live, terminated UTF-16 buffers for this call.
    let result = unsafe {
        CreateProcessAsUserW(
            Some(token),
            windows::core::PCWSTR::null(),
            Some(windows::core::PWSTR(command.as_mut_ptr())),
            None,
            None,
            false,
            PROCESS_CREATION_FLAGS(super::CREATE_NO_WINDOW),
            None,
            windows::core::PCWSTR::null(),
            &startup,
            &mut info,
        )
    };
    unsafe {
        CloseHandle(token).ok();
    }
    result?;
    unsafe {
        CloseHandle(info.hThread).ok();
    }
    Ok(AgentProcess {
        process_handle: Some(info.hProcess),
    })
}

fn token_integrity_level(token: HANDLE) -> Result<u32> {
    use windows::Win32::Security::{
        GetSidSubAuthority, GetSidSubAuthorityCount, GetTokenInformation, TOKEN_MANDATORY_LABEL,
        TokenIntegrityLevel,
    };
    let mut size = 0u32;
    unsafe {
        let _ = GetTokenInformation(token, TokenIntegrityLevel, None, 0, &mut size);
    }
    anyhow::ensure!(size != 0, "GetTokenInformation returned size 0");
    let mut buffer = vec![0u8; size as usize];
    unsafe {
        GetTokenInformation(
            token,
            TokenIntegrityLevel,
            Some(buffer.as_mut_ptr().cast()),
            size,
            &mut size,
        )?;
    }
    // SAFETY: buffer was sized by GetTokenInformation for TOKEN_MANDATORY_LABEL.
    let sid = unsafe {
        (*(buffer.as_ptr().cast::<TOKEN_MANDATORY_LABEL>()))
            .Label
            .Sid
    };
    unsafe {
        let count_ptr = GetSidSubAuthorityCount(sid);
        anyhow::ensure!(
            !count_ptr.is_null(),
            "GetSidSubAuthorityCount returned null"
        );
        let count = *count_ptr;
        anyhow::ensure!(count > 0, "SID has no sub-authorities");
        Ok(*GetSidSubAuthority(sid, (count - 1) as u32))
    }
}

fn find_winlogon_token(session_id: u32) -> Result<HANDLE> {
    use windows::Win32::Security::{
        GetTokenInformation, TOKEN_ASSIGN_PRIMARY, TOKEN_DUPLICATE, TOKEN_QUERY, TokenSessionId,
    };
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
        TH32CS_SNAPPROCESS,
    };
    use windows::Win32::System::Threading::{
        OpenProcess, OpenProcessToken, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)? };
    let mut entry = PROCESSENTRY32W {
        dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
        ..Default::default()
    };
    let mut found = None;
    let mut next = unsafe { Process32FirstW(snapshot, &mut entry) }.is_ok();
    while next {
        let end = entry
            .szExeFile
            .iter()
            .position(|unit| *unit == 0)
            .unwrap_or(entry.szExeFile.len());
        let name = String::from_utf16_lossy(&entry.szExeFile[..end]);
        if name.eq_ignore_ascii_case("winlogon.exe")
            && let Ok(process) = unsafe {
                OpenProcess(
                    PROCESS_QUERY_LIMITED_INFORMATION,
                    false,
                    entry.th32ProcessID,
                )
            }
        {
            let mut token = HANDLE::default();
            let opened = unsafe {
                OpenProcessToken(
                    process,
                    TOKEN_QUERY | TOKEN_DUPLICATE | TOKEN_ASSIGN_PRIMARY,
                    &mut token,
                )
            }
            .is_ok();
            let matches = if opened {
                let mut token_session = 0u32;
                let mut size = 0u32;
                let token_query_ok = unsafe {
                    GetTokenInformation(
                        token,
                        TokenSessionId,
                        Some((&mut token_session as *mut u32).cast()),
                        4,
                        &mut size,
                    )
                    .is_ok()
                };
                token_query_ok && token_session == session_id
            } else {
                false
            };
            if matches {
                found = Some(token);
            } else if opened {
                unsafe {
                    CloseHandle(token).ok();
                }
            }
            unsafe {
                CloseHandle(process).ok();
            }
        }
        if found.is_some() {
            break;
        }
        next = unsafe { Process32NextW(snapshot, &mut entry) }.is_ok();
    }
    unsafe {
        CloseHandle(snapshot).ok();
    }
    found.context("winlogon token not found")
}

pub(super) fn run_entry(args: &[String]) -> Result<()> {
    crate::logging::init_service_logging();
    super::AGENT_MODE.store(true, Ordering::Release);
    unsafe {
        std::env::set_var("QINGLINK_SESSION_AGENT", "1");
    }
    // 焦点读取的 UIA 调用放到独立线程，命令路径只读缓存——否则应用卡顿时会把
    // 串行处理的鼠标/键盘注入堵在后面。
    super::focus_cache::spawn();
    let session = args
        .windows(2)
        .find_map(|pair| (pair[0] == "--session").then(|| pair[1].parse::<u32>().ok()))
        .flatten()
        .context("session agent requires --session")?;
    let pipe_name = format!("{}{}", super::AGENT_PIPE_PREFIX, session);
    loop {
        if let Err(error) = serve_pipe(&pipe_name) {
            tracing::error!(%error, "Session agent pipe reset");
            std::thread::sleep(Duration::from_millis(100));
        }
    }
}

fn serve_pipe(pipe_name: &str) -> Result<()> {
    let cancelled = Arc::new(AtomicBool::new(false));
    let stream = super::pipe::accept(pipe_name, "D:(A;;GA;;;SY)", Arc::clone(&cancelled))?;
    super::validate_agent_pipe_client(stream.handle())?;
    let monitor = super::lifecycle_monitor::LifecycleMonitor::new(cancelled);
    super::dispatch::handle_client(stream, &monitor, None, None)
}
