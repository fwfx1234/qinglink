//! Windows 服务实现。
//!
//! 处理服务生命周期、命名管道 IPC、会话代理监督，
//! 以及安全桌面输入注入。

use std::io::{Read, Write};
use std::os::windows::io::FromRawHandle;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::mpsc;
use std::time::Duration;

mod lifecycle_monitor;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use windows::Win32::Foundation::{CloseHandle, GENERIC_WRITE, HANDLE, HLOCAL, LocalFree};
use windows::Win32::Security::Authorization::{
    ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
};
use windows::Win32::Security::SECURITY_ATTRIBUTES;
use windows::Win32::Storage::FileSystem::{FILE_FLAGS_AND_ATTRIBUTES, FlushFileBuffers};
use windows::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, PIPE_READMODE_BYTE, PIPE_TYPE_BYTE,
    PIPE_WAIT,
};
use windows::core::PCWSTR;
use windows_service::service::ServiceControl;
use windows_service::service::{ServiceState, ServiceStatus, ServiceType};
use windows_service::service_control_handler::{self, ServiceControlHandlerResult};
use windows_service::service_dispatcher;

const SERVICE_NAME: &str = "QingLinkService";
const PIPE_NAME: &str = r"\\.\pipe\QingLinkControlV2";
const AGENT_PIPE_PREFIX: &str = r"\\.\pipe\QingLinkSessionAgent-";
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
static MOUSE_BUTTONS: AtomicU8 = AtomicU8::new(0);
static AGENT_MODE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

struct AgentProcess {
    process_handle: Option<HANDLE>,
}

impl AgentProcess {
    fn exited(&mut self) -> bool {
        #[cfg(target_os = "windows")]
        if let Some(handle) = self.process_handle {
            use windows::Win32::System::Threading::GetExitCodeProcess;
            let mut code = 0u32;
            return unsafe { GetExitCodeProcess(handle, &mut code).is_ok() && code != 259 };
        }
        true
    }

    fn terminate(&mut self) {
        #[cfg(target_os = "windows")]
        if let Some(handle) = self.process_handle.take() {
            let _ = unsafe { windows::Win32::System::Threading::TerminateProcess(handle, 1) };
            let _ = unsafe { CloseHandle(handle) };
        }
    }
}

impl Drop for AgentProcess {
    fn drop(&mut self) {
        self.terminate();
    }
}

pub fn helper_move(dx: i32, dy: i32) {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_MOVE, MOUSEINPUT, SendInput,
    };
    let input = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx,
                dy,
                mouseData: 0,
                dwFlags: MOUSEEVENTF_MOVE,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    unsafe {
        let _ = SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
    }
}

pub fn helper_button(button: &str) {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP,
        MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEINPUT, SendInput,
    };
    let (down, up) = match button {
        "right" => (MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP),
        _ => (MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP),
    };
    let inputs = [
        INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: 0,
                    dy: 0,
                    mouseData: 0,
                    dwFlags: down,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
        INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: 0,
                    dy: 0,
                    mouseData: 0,
                    dwFlags: up,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
    ];
    unsafe {
        let _ = SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }
}

pub fn helper_button_state(button: &str, state: &str) {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP,
        MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEINPUT, SendInput,
    };
    let flag = match (button, state) {
        ("right", "down") => MOUSEEVENTF_RIGHTDOWN,
        ("right", _) => MOUSEEVENTF_RIGHTUP,
        ("left", "down") => MOUSEEVENTF_LEFTDOWN,
        _ => MOUSEEVENTF_LEFTUP,
    };
    let input = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: 0,
                dy: 0,
                mouseData: 0,
                dwFlags: flag,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    unsafe {
        let _ = SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
    }
}

pub fn helper_wheel(delta: i32) {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_WHEEL, MOUSEINPUT, SendInput,
    };
    let input = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: 0,
                dy: 0,
                mouseData: (delta.clamp(-20, 20) * 120) as u32,
                dwFlags: MOUSEEVENTF_WHEEL,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    unsafe {
        let _ = SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
    }
}

pub fn helper_key(key: &str) {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, SendInput, VK_BACK, VK_DELETE,
        VK_DOWN, VK_END, VK_ESCAPE, VK_HOME, VK_LEFT, VK_NEXT, VK_PRIOR, VK_RETURN, VK_RIGHT,
        VK_SPACE, VK_TAB, VK_UP,
    };
    let vk = match key {
        "backspace" => VK_BACK,
        "enter" => VK_RETURN,
        "escape" => VK_ESCAPE,
        "tab" => VK_TAB,
        "delete" => VK_DELETE,
        "home" => VK_HOME,
        "end" => VK_END,
        "pageup" => VK_PRIOR,
        "pagedown" => VK_NEXT,
        "up" => VK_UP,
        "down" => VK_DOWN,
        "left" => VK_LEFT,
        "right" => VK_RIGHT,
        "space" => VK_SPACE,
        _ => return,
    };
    let inputs = [
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    wScan: 0,
                    dwFlags: Default::default(),
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    wScan: 0,
                    dwFlags: KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
    ];
    unsafe {
        let _ = SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }
}

#[derive(Debug, Deserialize)]
struct Request {
    version: u32,
    #[serde(default)]
    request_id: Option<String>,
    command: String,
    pin: Option<String>,
    text: Option<String>,
    dx: Option<i32>,
    dy: Option<i32>,
    button: Option<String>,
    pressed: Option<bool>,
    delta: Option<i32>,
    usage: Option<u8>,
    modifiers: Option<u8>,
    parent_pid: Option<u32>,
    hwnd: Option<usize>,
    #[serde(default)]
    pid: Option<u32>,
    direction: Option<String>,
}

#[derive(Debug, Serialize)]
struct Response<'a> {
    ok: bool,
    code: &'a str,
    message: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    request_id: Option<&'a str>,
}

pub fn run() -> Result<()> {
    service_dispatcher::start(SERVICE_NAME, ffi_service_main)
        .context("failed to dispatch Windows service")?;
    Ok(())
}

extern "system" fn ffi_service_main(_arguments: u32, _argv: *mut *mut u16) {
    if let Err(error) = service_main() {
        eprintln!("{error:#}");
    }
}

fn service_main() -> Result<()> {
    crate::logging::init_service_logging();
    tracing::info!("QingLink service starting");
    let (stop_tx, stop_rx) = mpsc::channel();
    let control_stop_tx = stop_tx.clone();
    let lifecycle_stop = Arc::new(AtomicBool::new(false));
    let lifecycle_monitor = lifecycle_monitor::LifecycleMonitor::new(Arc::clone(&lifecycle_stop));
    lifecycle_monitor.start_monitors();
    let paths = qinglink_storage::AppPaths::resolve_system()?;
    let database = std::sync::Arc::new(qinglink_storage::database::DatabaseService::new(
        paths.clone(),
    ));
    database.register_database(qinglink_storage::database::DatabaseSpec::app(
        "qinglink/data",
        "qinglink.db",
    ))?;
    let network_state = qinglink_server::AppState::new(
        qinglink_server::service::RemoteControlService::new(paths),
        database,
    );
    let status_handle =
        service_control_handler::register(SERVICE_NAME, move |control| match control {
            ServiceControl::Stop | ServiceControl::Shutdown => {
                let _ = control_stop_tx.send(());
                wake_control_pipe();
                ServiceControlHandlerResult::NoError
            }
            _ => ServiceControlHandlerResult::NotImplemented,
        })?;
    status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::StartPending,
        controls_accepted: windows_service::service::ServiceControlAccept::empty(),
        exit_code: windows_service::service::ServiceExitCode::Win32(0),
        checkpoint: 1,
        wait_hint: Duration::from_secs(60),
        process_id: None,
    })?;
    let agent_stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let agent_stop_worker = std::sync::Arc::clone(&agent_stop);
    let agent_worker = std::thread::spawn(move || session_agent_supervisor(agent_stop_worker));
    let network_stop = std::sync::Arc::clone(&agent_stop);
    let (network_ready_tx, network_ready_rx) = mpsc::channel();
    let network_state_worker = network_state.clone();
    let network_worker = std::thread::spawn(move || {
        run_network_server(network_stop, network_state_worker, Some(network_ready_tx))
    });
    if let Err(error) = network_ready_rx.recv_timeout(Duration::from_secs(60)) {
        agent_stop.store(true, Ordering::Release);
        let _ = agent_worker.join();
        let _ = network_worker.join();
        return Err(error).context("network listener did not become ready within 60 seconds");
    }
    status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Running,
        controls_accepted: windows_service::service::ServiceControlAccept::STOP
            | windows_service::service::ServiceControlAccept::SHUTDOWN,
        exit_code: windows_service::service::ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    })?;
    loop {
        if stop_rx.try_recv().is_ok() || lifecycle_stop.load(Ordering::Acquire) {
            if lifecycle_stop.load(Ordering::Acquire) {
                tracing::info!("生命周期监控触发停止");
            }
            break;
        }
        if let Err(error) = serve_one_client(&lifecycle_monitor, &network_state, &stop_tx) {
            tracing::warn!(%error, "客户端请求处理失败，继续运行");
        }
    }
    agent_stop.store(true, Ordering::Release);
    let _ = agent_worker.join();
    let _ = network_worker.join();
    status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Stopped,
        controls_accepted: windows_service::service::ServiceControlAccept::empty(),
        exit_code: windows_service::service::ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    })?;
    Ok(())
}

pub fn run_session_agent_entry(args: &[String]) -> Result<()> {
    run_session_agent(args)
}

fn wake_control_pipe() {
    // ConnectNamedPipe blocks while idle. A local SYSTEM client wakes the
    // listener so SCM Stop/Shutdown is observed without waiting for a
    // desktop IPC request.
    if let Ok(mut pipe) = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(PIPE_NAME)
    {
        let _ = pipe.write_all(&0u32.to_le_bytes());
    }
}

fn run_network_server(
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    state: qinglink_server::AppState,
    mut ready: Option<mpsc::Sender<()>>,
) {
    let result = (|| -> Result<()> {
        let runtime = tokio::runtime::Runtime::new()?;
        runtime.block_on(async move {
                // A Windows service can be reported as RUNNING before the TCP
                // port is available (for example while another process is
                // releasing the port). Keep the network supervisor alive and
                // retry with bounded exponential backoff instead of leaving a
                // permanently healthy-looking service with no listener.
                let mut retry_delay = Duration::from_secs(1);
                loop {
                    if stop.load(Ordering::Acquire) {
                        break;
                    }

                    let (network_shutdown_tx, network_shutdown_rx) = tokio::sync::oneshot::channel();
                    let stop_for_shutdown = std::sync::Arc::clone(&stop);
                    let shutdown = async move {
                        tokio::select! {
                            _ = wait_for_stop(stop_for_shutdown) => {}
                            _ = network_shutdown_rx => {}
                        }
                    };
                    match qinglink_server::RemoteServer::run_with_shutdown(
                        state.clone(),
                        3721,
                        shutdown,
                    )
                    .await
                    {
                        Ok((address, server)) => {
                            retry_delay = Duration::from_secs(1);
                            let discovery = match qinglink_discovery::UdpDiscoveryResponder::start(
                                state.service.pc_id(),
                                state.service.pc_name(),
                                address.port(),
                                None,
                            ) {
                                Ok(value) => value,
                                Err(error) => {
                                    tracing::error!(%error, "QingLink UDP discovery unavailable");
                                    state.service.set_server_running(false, address.port());
                                    let _ = network_shutdown_tx.send(());
                                    let _ = server.await;
                                    tokio::time::sleep(Duration::from_secs(1)).await;
                                    continue;
                                }
                            };
                            tracing::info!(%address, "QingLink system service network server listening");
                            if let Some(ready) = ready.take() {
                                let _ = ready.send(());
                            }
                            let _ = server.await;
                            drop(discovery);
                            if !stop.load(Ordering::Acquire) {
                                tracing::warn!("QingLink network server stopped unexpectedly; restarting");
                            }
                        }
                        Err(error) => {
                            tracing::error!(%error, ?retry_delay, "QingLink network bind failed; retrying");
                            tokio::select! {
                                _ = tokio::time::sleep(retry_delay) => {}
                                _ = wait_for_stop(std::sync::Arc::clone(&stop)) => break,
                            }
                            retry_delay = (retry_delay * 2).min(Duration::from_secs(15));
                        }
                    }
                }
                Ok::<(), anyhow::Error>(())
            })
    })();
    if let Err(error) = result {
        tracing::error!(%error, "QingLink network server stopped");
    }
}

fn serve_one_client(
    monitor: &lifecycle_monitor::LifecycleMonitor,
    state: &qinglink_server::AppState,
    stop_tx: &mpsc::Sender<()>,
) -> Result<()> {
    let name: Vec<u16> = PIPE_NAME.encode_utf16().chain([0]).collect();
    // The desktop client runs as an interactive standard user, while the
    // service itself is LocalSystem.  Do not grant Everyone full access:
    // that would let an unrelated local process inject input or submit a
    // lock-screen PIN without going through the authenticated network
    // session.  IU covers the currently interactive user session.
    let sddl: Vec<u16> = "D:(A;;GA;;;SY)(A;;GA;;;BA)(A;;GRGW;;;IU)"
        .encode_utf16()
        .chain([0])
        .collect();
    let mut descriptor = std::ptr::null_mut();
    let mut descriptor_len = 0u32;
    unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            windows::core::PCWSTR(sddl.as_ptr()),
            SDDL_REVISION_1,
            &mut descriptor as *mut _ as *mut windows::Win32::Security::PSECURITY_DESCRIPTOR,
            Some(&mut descriptor_len),
        )?;
    }
    let security = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: descriptor,
        bInheritHandle: false.into(),
    };
    let pipe = unsafe {
        CreateNamedPipeW(
            PCWSTR(name.as_ptr()),
            FILE_FLAGS_AND_ATTRIBUTES(3),
            PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT,
            255,
            64 * 1024,
            64 * 1024,
            0,
            Some(&security),
        )
    };
    unsafe {
        LocalFree(Some(HLOCAL(descriptor)));
    }
    unsafe { ConnectNamedPipe(pipe, None) }.ok();
    let result = handle_client(pipe, monitor, Some(state), Some(stop_tx));
    unsafe {
        FlushFileBuffers(pipe).ok();
        DisconnectNamedPipe(pipe).ok();
        CloseHandle(pipe).ok();
    }
    result
}

fn validate_agent_pipe_client(pipe: HANDLE) -> Result<()> {
    use windows::Win32::System::Pipes::{GetNamedPipeClientProcessId, GetNamedPipeClientSessionId};
    use windows::Win32::System::RemoteDesktop::ProcessIdToSessionId;
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
    let mut pid = 0u32;
    let mut pipe_session = 0u32;
    unsafe {
        GetNamedPipeClientProcessId(pipe, &mut pid)?;
        GetNamedPipeClientSessionId(pipe, &mut pipe_session)?;
    }
    let mut process_session = 0u32;
    unsafe {
        ProcessIdToSessionId(pid, &mut process_session)?;
    }
    anyhow::ensure!(
        pipe_session == 0 && process_session == 0,
        "agent IPC client is not the LocalSystem service"
    );
    let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid)? };
    let same_binary = process_image_path(process).ok()
        == std::env::current_exe()
            .ok()
            .and_then(|path| path.canonicalize().ok());
    unsafe {
        CloseHandle(process).ok();
    }
    anyhow::ensure!(same_binary, "agent IPC client binary is not trusted");
    Ok(())
}

fn process_image_path(process: HANDLE) -> Result<std::path::PathBuf> {
    use windows::Win32::System::Threading::QueryFullProcessImageNameW;
    let mut buffer = vec![0u16; 32_768];
    let mut length = buffer.len() as u32;
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

/// 保持一个签名的输入代理附加到活动的控制台会话。
/// 子进程在会话转换或崩溃后重启。
fn session_agent_supervisor(stop: std::sync::Arc<std::sync::atomic::AtomicBool>) {
    let mut active = u32::MAX;
    let mut child: Option<AgentProcess> = None;
    tracing::info!("session agent supervisor started");
    while !stop.load(Ordering::Acquire) {
        let session = active_interactive_session().unwrap_or_else(|| unsafe {
            windows::Win32::System::RemoteDesktop::WTSGetActiveConsoleSessionId()
        });
        let exited = child.as_mut().is_some_and(AgentProcess::exited);
        if session != active || exited {
            if let Some(mut process) = child.take() {
                tracing::info!(
                    old_session = active,
                    new_session = session,
                    "session changed, terminating agent"
                );
                process.terminate();
            }
            active = session;
            if session != u32::MAX {
                match launch_session_agent(session) {
                    Ok(agent) => {
                        tracing::info!(session_id = session, "Session agent launched");
                        child = Some(agent);
                    }
                    Err(error) => {
                        tracing::error!(session_id = session, %error, "Failed to launch session agent");
                    }
                }
            }
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    tracing::info!("session agent supervisor stopping");
    if let Some(mut process) = child {
        process.terminate();
    }
}

fn launch_session_agent(session_id: u32) -> Result<AgentProcess> {
    let executable = std::env::current_exe().context("cannot locate service executable")?;
    tracing::info!(session_id, exe = %executable.display(), "launching session agent");
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::System::Threading::{
            CreateProcessAsUserW, PROCESS_CREATION_FLAGS, PROCESS_INFORMATION, STARTUPINFOW,
        };
        let token = find_winlogon_token(session_id).unwrap_or_default();
        let used_winlogon_token = !token.is_invalid();
        if token.is_invalid() {
            tracing::warn!(
                session_id,
                "Winlogon token unavailable; secure session agent will not start"
            );
        }
        if !token.is_invalid() {
            if let Ok(level) = token_integrity_level(token) {
                tracing::info!(
                    session_id,
                    level,
                    used_winlogon_token,
                    "Session agent token acquired"
                );
                if level < 0x3000 {
                    tracing::warn!(
                        session_id,
                        level,
                        "Token integrity level below High (0x3000). UAC consent desktop input may be blocked"
                    );
                }
            } else {
                tracing::warn!(session_id, "Could not determine token integrity level");
            }
            let mut command = format!(
                "\"{}\" --session-agent --session {}",
                executable.display(),
                session_id
            )
            .encode_utf16()
            .chain([0])
            .collect::<Vec<u16>>();
            let mut startup = STARTUPINFOW::default();
            startup.cb = std::mem::size_of::<STARTUPINFOW>() as u32;
            let desktop_name: Vec<u16> = "WinSta0\\Default".encode_utf16().chain([0]).collect();
            startup.lpDesktop = windows::core::PWSTR(desktop_name.as_ptr() as *mut u16);
            let mut info = PROCESS_INFORMATION::default();
            let result = unsafe {
                CreateProcessAsUserW(
                    Some(token),
                    windows::core::PCWSTR::null(),
                    Some(windows::core::PWSTR(command.as_mut_ptr())),
                    None,
                    None,
                    false,
                    PROCESS_CREATION_FLAGS(CREATE_NO_WINDOW),
                    None,
                    windows::core::PCWSTR::null(),
                    &startup,
                    &mut info,
                )
            };
            unsafe {
                CloseHandle(token).ok();
            }
            if result.is_ok() {
                unsafe {
                    CloseHandle(info.hThread).ok();
                }
                return Ok(AgentProcess {
                    process_handle: Some(info.hProcess),
                });
            }
        }
    }
    anyhow::bail!("cannot launch secure session agent: Winlogon token unavailable")
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
    if size == 0 {
        anyhow::bail!("GetTokenInformation returned size 0");
    }
    let mut buffer = vec![0u8; size as usize];
    let result = unsafe {
        GetTokenInformation(
            token,
            TokenIntegrityLevel,
            Some(buffer.as_mut_ptr().cast()),
            size,
            &mut size,
        )
    };
    if result.is_err() {
        anyhow::bail!("GetTokenInformation(TokenIntegrityLevel) failed");
    }
    let label = unsafe { &*(buffer.as_ptr().cast::<TOKEN_MANDATORY_LABEL>()) };
    let sid = label.Label.Sid;
    unsafe {
        let count_ptr = GetSidSubAuthorityCount(sid);
        if count_ptr.is_null() {
            anyhow::bail!("GetSidSubAuthorityCount returned null");
        }
        let count = *count_ptr;
        if count < 1 {
            anyhow::bail!("SID has no sub-authorities");
        }
        let authority = *(GetSidSubAuthority(sid, (count - 1) as u32) as *const u32);
        Ok(authority as u32)
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
    let mut entry = PROCESSENTRY32W::default();
    entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
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
            if unsafe {
                OpenProcessToken(
                    process,
                    TOKEN_QUERY | TOKEN_DUPLICATE | TOKEN_ASSIGN_PRIMARY,
                    &mut token,
                )
            }
            .is_ok()
            {
                let mut token_session = 0u32;
                let mut size = 0u32;
                let matches = unsafe {
                    GetTokenInformation(
                        token,
                        TokenSessionId,
                        Some((&mut token_session as *mut u32).cast()),
                        4,
                        &mut size,
                    )
                }
                .is_ok()
                    && token_session == session_id;
                if matches {
                    found = Some(token);
                } else {
                    unsafe {
                        CloseHandle(token).ok();
                    }
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

fn run_session_agent(args: &[String]) -> Result<()> {
    crate::logging::init_service_logging();
    AGENT_MODE.store(true, Ordering::Release);
    unsafe {
        std::env::set_var("QINGLINK_SESSION_AGENT", "1");
    }
    let session = args
        .windows(2)
        .find_map(|pair| (pair[0] == "--session").then(|| pair[1].parse::<u32>().ok()))
        .flatten()
        .context("session agent requires --session")?;
    tracing::info!(
        session_id = session,
        "Session agent starting (secure desktop input mode)"
    );
    let pipe_name = format!("{AGENT_PIPE_PREFIX}{session}");
    loop {
        if let Err(error) = serve_agent_pipe(&pipe_name) {
            tracing::error!(%error, "Session agent pipe reset");
            std::thread::sleep(Duration::from_millis(100));
        }
    }
}

fn serve_agent_pipe(pipe_name: &str) -> Result<()> {
    let name: Vec<u16> = pipe_name.encode_utf16().chain([0]).collect();
    let sddl: Vec<u16> = "D:(A;;GA;;;WD)".encode_utf16().chain([0]).collect();
    let mut descriptor = std::ptr::null_mut();
    let mut descriptor_len = 0u32;
    unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            PCWSTR(sddl.as_ptr()),
            SDDL_REVISION_1,
            &mut descriptor as *mut _ as *mut windows::Win32::Security::PSECURITY_DESCRIPTOR,
            Some(&mut descriptor_len),
        )?;
    }
    let security = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: descriptor,
        bInheritHandle: false.into(),
    };
    let pipe = unsafe {
        CreateNamedPipeW(
            PCWSTR(name.as_ptr()),
            FILE_FLAGS_AND_ATTRIBUTES(3),
            PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT,
            1,
            64 * 1024,
            64 * 1024,
            0,
            Some(&security),
        )
    };
    unsafe {
        LocalFree(Some(HLOCAL(descriptor)));
    }
    unsafe { ConnectNamedPipe(pipe, None) }.ok();
    if let Err(error) = validate_agent_pipe_client(pipe) {
        unsafe {
            DisconnectNamedPipe(pipe).ok();
            CloseHandle(pipe).ok();
        }
        return Err(error);
    }
    let dummy_monitor = lifecycle_monitor::LifecycleMonitor::new(Arc::new(AtomicBool::new(false)));
    let result = handle_client(pipe, &dummy_monitor, None, None);
    unsafe {
        FlushFileBuffers(pipe).ok();
        DisconnectNamedPipe(pipe).ok();
        CloseHandle(pipe).ok();
    }
    result
}

fn handle_client(
    pipe: HANDLE,
    monitor: &lifecycle_monitor::LifecycleMonitor,
    state: Option<&qinglink_server::AppState>,
    stop_tx: Option<&mpsc::Sender<()>>,
) -> Result<()> {
    let client_pid = named_pipe_client_pid(pipe)?;
    let mut stream = unsafe { std::fs::File::from_raw_handle(pipe.0 as _) };
    let mut length = [0u8; 4];
    stream.read_exact(&mut length)?;
    let length = u32::from_le_bytes(length) as usize;
    if length == 0 || length > 64 * 1024 {
        let result = write_response(&mut stream, false, "INVALID_REQUEST", "请求大小无效");
        std::mem::forget(stream);
        return result;
    }
    let mut payload = vec![0u8; length];
    stream.read_exact(&mut payload)?;
    let request: Request = serde_json::from_slice(&payload)?;
    if AGENT_MODE.load(Ordering::Acquire) {
        if let Err(error) = attach_agent_desktop() {
            tracing::error!(%error, "Failed to attach agent desktop");
        }
    }
    if request.version != 1 && request.version != 2 {
        tracing::warn!(version = request.version, "unsupported protocol version");
        let result = write_response(&mut stream, false, "UNSUPPORTED_VERSION", "协议版本不支持");
        std::mem::forget(stream);
        return result;
    }
    tracing::debug!(command = %request.command, "processing client command");
    match request.command.as_str() {
        "get_status" => {
            let available = state.is_some_and(|state| state.service.is_server_running());
            let result = write_response_with_id(
                &mut stream,
                available,
                if available { "READY" } else { "DEGRADED" },
                if available {
                    "服务已就绪"
                } else {
                    "网络服务未就绪"
                },
                request.request_id.as_deref(),
            );
            std::mem::forget(stream);
            result
        }
        "register_parent" => {
            if let Some(pid) = request.parent_pid {
                let result = if pid == client_pid {
                    monitor.register_parent(pid);
                    write_response(&mut stream, true, "REGISTERED", "父进程已注册")
                } else {
                    tracing::warn!(
                        claimed_pid = pid,
                        client_pid,
                        "拒绝 PID 不匹配的桌面端注册请求"
                    );
                    write_response(
                        &mut stream,
                        false,
                        "PID_MISMATCH",
                        "父进程 PID 与管道客户端不匹配",
                    )
                };
                std::mem::forget(stream);
                result
            } else {
                let result = write_response(&mut stream, false, "MISSING_PID", "缺少 parent_pid");
                std::mem::forget(stream);
                result
            }
        }
        "heartbeat" => {
            let result = if monitor.accepts_client_pid(client_pid) {
                write_response(&mut stream, true, "HEARTBEAT_OK", "心跳已接收")
            } else {
                tracing::warn!(client_pid, "拒绝未登记桌面端的心跳请求");
                write_response(&mut stream, false, "UNREGISTERED_CLIENT", "桌面端尚未登记")
            };
            std::mem::forget(stream);
            result
        }
        "shutdown" => {
            let authorized = monitor.accepts_client_pid(client_pid);
            let result = if authorized {
                tracing::info!(client_pid, "收到已登记桌面端的 Shutdown 命令，服务即将停止");
                let result = write_response(&mut stream, true, "SHUTTING_DOWN", "服务正在关闭");
                if let Some(stop_tx) = stop_tx {
                    let _ = stop_tx.send(());
                }
                result
            } else {
                tracing::warn!(client_pid, "拒绝未登记桌面端的 Shutdown 命令");
                write_response(&mut stream, false, "UNREGISTERED_CLIENT", "桌面端尚未登记")
            };
            std::mem::forget(stream);
            result
        }
        "lock" => {
            let result = match qinglink_windows::power::lock() {
                Ok(()) => write_response(&mut stream, true, "SERVICE_OK", "锁屏已请求"),
                Err(error) => {
                    tracing::error!(%error, "lock request failed");
                    write_response(&mut stream, false, "LOCK_FAILED", &error.to_string())
                }
            };
            std::mem::forget(stream);
            result
        }
        "secure_ascii" => {
            let text = request.text.as_deref().unwrap_or("");
            tracing::info!(length = text.len(), "secure ASCII input requested");
            let result = qinglink_windows::secure_desktop::send_secure_ascii(text);
            match result {
                Ok(()) => {
                    tracing::info!(length = text.len(), "secure ASCII input delivered");
                    write_response(&mut stream, true, "SERVICE_OK", "安全文本已发送")
                }
                Err(error) => {
                    tracing::error!(%error, "secure ASCII input failed");
                    write_response(&mut stream, false, "INPUT_FAILED", &error.to_string())
                }
            }
            .inspect(|_| {
                std::mem::forget(stream);
            })
        }
        "send_text" => {
            let text = request.text.as_deref().unwrap_or("");
            tracing::info!(length = text.len(), "Unicode text input requested");
            let result = qinglink_windows::secure_desktop::send_text(text);
            match result {
                Ok(()) => write_response(&mut stream, true, "SERVICE_OK", "文字已发送"),
                Err(error) => {
                    write_response(&mut stream, false, "INPUT_FAILED", &error.to_string())
                }
            }
            .inspect(|_| {
                std::mem::forget(stream);
            })
        }
        "list_windows" => {
            let result =
                serde_json::to_string(&qinglink_windows::platform_windows::enum_windows_light());
            match result {
                Ok(payload) => write_response(&mut stream, true, "WINDOW_LIST", &payload),
                Err(error) => {
                    write_response(&mut stream, false, "WINDOW_LIST_FAILED", &error.to_string())
                }
            }
            .inspect(|_| {
                std::mem::forget(stream);
            })
        }
        "focus_window" => {
            let result = request
                .hwnd
                .ok_or_else(|| anyhow::anyhow!("缺少窗口句柄"))
                .and_then(qinglink_windows::platform_windows::focus_window);
            let result = match result {
                Ok(()) => write_response(&mut stream, true, "WINDOW_OK", "窗口已激活"),
                Err(error) => {
                    write_response(&mut stream, false, "FOCUS_FAILED", &error.to_string())
                }
            };
            std::mem::forget(stream);
            result
        }
        "focus_process" => {
            let result = request
                .pid
                .ok_or_else(|| anyhow::anyhow!("缺少进程 PID"))
                .and_then(qinglink_windows::platform_windows::focus_process);
            let result = match result {
                Ok(()) => write_response(&mut stream, true, "WINDOW_OK", "窗口已激活"),
                Err(error) => {
                    write_response(&mut stream, false, "FOCUS_FAILED", &error.to_string())
                }
            };
            std::mem::forget(stream);
            result
        }
        "minimize_window" | "maximize_window" | "restore_window" | "close_window" => {
            let result = request
                .hwnd
                .ok_or_else(|| anyhow::anyhow!("缺少窗口句柄"))
                .and_then(|hwnd| match request.command.as_str() {
                    "minimize_window" => qinglink_windows::platform_windows::minimize_window(hwnd),
                    "maximize_window" => qinglink_windows::platform_windows::maximize_window(hwnd),
                    "restore_window" => qinglink_windows::platform_windows::restore_window(hwnd),
                    "close_window" => qinglink_windows::platform_windows::close_window(hwnd),
                    _ => unreachable!(),
                });
            let result = match result {
                Ok(()) => write_response(&mut stream, true, "WINDOW_OK", "窗口操作已完成"),
                Err(error) => write_response(
                    &mut stream,
                    false,
                    "WINDOW_ACTION_FAILED",
                    &error.to_string(),
                ),
            };
            std::mem::forget(stream);
            result
        }
        "switch_window" => {
            let direction = match request.direction.as_deref() {
                Some("next") => qinglink_protocol::WindowSwitchDirection::Next,
                Some("previous") => qinglink_protocol::WindowSwitchDirection::Previous,
                _ => {
                    let result =
                        write_response(&mut stream, false, "INVALID_DIRECTION", "窗口切换方向无效");
                    std::mem::forget(stream);
                    return result;
                }
            };
            let result = qinglink_windows::platform_windows::switch_window(direction);
            let result = match result {
                Ok(()) => write_response(&mut stream, true, "WINDOW_OK", "窗口已切换"),
                Err(error) => write_response(
                    &mut stream,
                    false,
                    "SWITCH_WINDOW_FAILED",
                    &error.to_string(),
                ),
            };
            std::mem::forget(stream);
            result
        }
        "status" => {
            // The service must expose the target session-agent state too;
            // reporting the service process as "ready" would make a
            // missing Winlogon agent look like an available input path.
            let result = match qinglink_windows::secure_desktop::status() {
                Ok(status) => write_agent_status_response(&mut stream, &status),
                Err(error) => {
                    write_response(&mut stream, false, "STATUS_FAILED", &error.to_string())
                }
            };
            std::mem::forget(stream);
            result
        }
        "mouse_move" => {
            let dx = request.dx.unwrap_or(0);
            let dy = request.dy.unwrap_or(0);
            tracing::debug!(dx, dy, "mouse move requested");
            let result = inject_mouse_move(dx, dy);
            match result {
                Ok(()) => write_response(&mut stream, true, "SERVICE_OK", "鼠标移动已发送"),
                Err(error) => {
                    tracing::warn!(dx, dy, %error, "mouse move failed");
                    write_response(&mut stream, false, "INPUT_FAILED", &error.to_string())
                }
            }
            .inspect(|_| {
                std::mem::forget(stream);
            })
        }
        "mouse_move_absolute" => {
            let x = request.dx.unwrap_or(0);
            let y = request.dy.unwrap_or(0);
            tracing::info!(x, y, "absolute mouse move requested");
            let result = inject_mouse_move_absolute(x, y);
            match result {
                Ok(()) => write_response(&mut stream, true, "SERVICE_OK", "绝对定位鼠标移动已发送"),
                Err(error) => {
                    tracing::warn!(x, y, %error, "absolute mouse move failed");
                    write_response(&mut stream, false, "INPUT_FAILED", &error.to_string())
                }
            }
            .inspect(|_| {
                std::mem::forget(stream);
            })
        }
        "mouse_button" => {
            let button = request.button.as_deref().unwrap_or("");
            tracing::info!(button, "mouse button click requested");
            let result = inject_mouse_button(button);
            match result {
                Ok(()) => {
                    tracing::info!(button, "mouse button click delivered");
                    write_response(&mut stream, true, "SERVICE_OK", "鼠标按键已发送")
                }
                Err(error) => {
                    tracing::warn!(button, %error, "mouse button failed");
                    write_response(&mut stream, false, "INPUT_FAILED", &error.to_string())
                }
            }
            .inspect(|_| {
                std::mem::forget(stream);
            })
        }
        "mouse_button_state" => {
            let button = request.button.as_deref().unwrap_or("");
            let pressed = request.pressed.unwrap_or(false);
            tracing::info!(button, pressed, "mouse button state requested");
            let result = inject_mouse_button_state(button, pressed);
            match result {
                Ok(()) => {
                    tracing::info!(button, pressed, "mouse button state delivered");
                    write_response(&mut stream, true, "SERVICE_OK", "鼠标按键状态已发送")
                }
                Err(error) => {
                    tracing::warn!(button, pressed, %error, "mouse button state failed");
                    write_response(&mut stream, false, "INPUT_FAILED", &error.to_string())
                }
            }
            .inspect(|_| {
                std::mem::forget(stream);
            })
        }
        "mouse_wheel" => {
            let delta = request.delta.unwrap_or(0);
            tracing::debug!(delta, "mouse wheel requested");
            let result = inject_mouse_wheel(delta);
            match result {
                Ok(()) => write_response(&mut stream, true, "SERVICE_OK", "鼠标滚轮已发送"),
                Err(error) => {
                    tracing::warn!(delta, %error, "mouse wheel failed");
                    write_response(&mut stream, false, "INPUT_FAILED", &error.to_string())
                }
            }
            .inspect(|_| {
                std::mem::forget(stream);
            })
        }
        "key_press" => {
            let key = request.button.as_deref().unwrap_or("");
            tracing::debug!(key, "key press requested");
            if let Some(usage) = key_to_usage(key) {
                let result = inject_key_event(usage, 0, true)
                    .and_then(|_| inject_key_event(usage, 0, false));
                match result {
                    Ok(()) => {
                        tracing::debug!(key, usage, "key press delivered");
                        write_response(&mut stream, true, "SERVICE_OK", "按键已发送")
                    }
                    Err(error) => {
                        tracing::warn!(key, usage, %error, "key press failed");
                        write_response(&mut stream, false, "INPUT_FAILED", &error.to_string())
                    }
                }
                .inspect(|_| {
                    std::mem::forget(stream);
                })
            } else {
                tracing::warn!(key, "unsupported key requested");
                let result = write_response(&mut stream, false, "INVALID_KEY", "不支持的按键");
                std::mem::forget(stream);
                result
            }
        }
        "key_event" => {
            let usage = request.usage.unwrap_or(0);
            let modifiers = request.modifiers.unwrap_or(0);
            let pressed = request.pressed.unwrap_or(false);
            tracing::debug!(usage, modifiers, pressed, "HID key event requested");
            let result = inject_key_event(usage, modifiers, pressed);
            match result {
                Ok(()) => {
                    tracing::debug!(usage, modifiers, pressed, "HID key event delivered");
                    write_response(&mut stream, true, "SERVICE_OK", "HID 按键已发送")
                }
                Err(error) => {
                    tracing::warn!(usage, modifiers, pressed, %error, "HID key event failed");
                    write_response(&mut stream, false, "INPUT_FAILED", &error.to_string())
                }
            }
            .inspect(|_| {
                std::mem::forget(stream);
            })
        }
        "submit_pin" => {
            let Some(pin) = request.pin else {
                tracing::warn!("submit_pin: empty PIN received");
                let result =
                    write_response(&mut stream, false, "INVALID_PIN_FORMAT", "PIN 不能为空");
                std::mem::forget(stream);
                return result;
            };
            if !(4..=32).contains(&pin.len()) || !pin.bytes().all(|byte| byte.is_ascii_digit()) {
                tracing::warn!(length = pin.len(), "submit_pin: invalid PIN format");
                let result = write_response(
                    &mut stream,
                    false,
                    "INVALID_PIN_FORMAT",
                    "PIN 必须是 4-32 位数字",
                );
                std::mem::forget(stream);
                return result;
            }
            tracing::info!(
                length = pin.len(),
                "submit_pin: submitting PIN for Windows login"
            );
            let result = pin
                .chars()
                .try_for_each(|digit| {
                    let usage = if digit == '0' {
                        0x27
                    } else {
                        0x1d + digit.to_digit(10).unwrap_or(0) as u8
                    };
                    inject_key_event(usage, 0, true)?;
                    inject_key_event(usage, 0, false)
                })
                .and_then(|()| {
                    inject_key_event(0x28, 0, true)?;
                    inject_key_event(0x28, 0, false)
                });
            let result = match result {
                Ok(()) => write_response(&mut stream, true, "SERVICE_OK", "PIN 已通过 HID 发送"),
                Err(error) => {
                    write_response(&mut stream, false, "INPUT_FAILED", &error.to_string())
                }
            };
            std::mem::forget(stream);
            result
        }
        _ => {
            let result = write_response(&mut stream, false, "UNKNOWN_COMMAND", "未知输入命令");
            std::mem::forget(stream);
            result
        }
    }
}

fn attach_agent_desktop() -> Result<()> {
    use windows::Win32::System::StationsAndDesktops::{
        CloseDesktop, DESKTOP_ACCESS_FLAGS, DESKTOP_CREATEMENU, DESKTOP_CREATEWINDOW,
        DESKTOP_ENUMERATE, DESKTOP_HOOKCONTROL, DESKTOP_READOBJECTS, DESKTOP_SWITCHDESKTOP,
        DESKTOP_WRITEOBJECTS, OpenInputDesktop, SetThreadDesktop,
    };
    let access = DESKTOP_ACCESS_FLAGS(
        DESKTOP_CREATEMENU.0
            | DESKTOP_CREATEWINDOW.0
            | DESKTOP_ENUMERATE.0
            | DESKTOP_HOOKCONTROL.0
            | DESKTOP_READOBJECTS.0
            | DESKTOP_SWITCHDESKTOP.0
            | DESKTOP_WRITEOBJECTS.0
            | GENERIC_WRITE.0,
    );
    let current_name = get_thread_desktop_name();
    let input_name = get_input_desktop_name();
    if let (Some(current), Some(input)) = (&current_name, &input_name) {
        if current.eq_ignore_ascii_case(input) {
            tracing::debug!(desktop = %current, "attach_agent_desktop: already attached to input desktop");
            return Ok(());
        }
        tracing::info!(from = %current, to = %input, "attach_agent_desktop: switching thread desktop for agent");
    }
    let desktop = unsafe { OpenInputDesktop(Default::default(), false, access)? };
    // Keep the handle alive while the thread is attached. Closing the
    // selected desktop makes later Winlogon/UAC transitions unreliable.
    if let Err(error) = unsafe { SetThreadDesktop(desktop) } {
        unsafe {
            CloseDesktop(desktop).ok();
        }
        return Err(error.into());
    }
    let new_desktop = get_thread_desktop_name();
    tracing::info!(desktop = ?new_desktop, "attach_agent_desktop: thread desktop switched successfully");
    Ok(())
}

fn inject_mouse_move(dx: i32, dy: i32) -> Result<()> {
    anyhow::ensure!(dx.abs() <= 500 && dy.abs() <= 500, "鼠标位移过大");
    tracing::debug!(dx, dy, "inject_mouse_move: injecting relative mouse move");
    send_mouse_report(dx, dy, 0)
}

fn inject_mouse_move_absolute(x: i32, y: i32) -> Result<()> {
    tracing::debug!(
        x,
        y,
        "inject_mouse_move_absolute: injecting absolute mouse move"
    );
    send_mouse_report_absolute(x, y)
}

fn key_to_usage(key: &str) -> Option<u8> {
    let key = key.trim().to_ascii_lowercase();
    if key.len() == 1 {
        let byte = key.as_bytes()[0];
        if byte.is_ascii_lowercase() {
            return Some(byte - b'a' + 0x04);
        }
        if byte.is_ascii_digit() {
            return Some(if byte == b'0' {
                0x27
            } else {
                byte - b'1' + 0x1e
            });
        }
    }
    Some(match key.as_str() {
        "backspace" => 0x2a,
        "enter" => 0x28,
        "escape" => 0x29,
        "tab" => 0x2b,
        "space" => 0x2c,
        "delete" => 0x4c,
        "home" => 0x4a,
        "end" => 0x4d,
        "pageup" => 0x4b,
        "pagedown" => 0x4e,
        "up" => 0x52,
        "down" => 0x51,
        "left" => 0x50,
        "right" => 0x4f,
        "f1" => 0x3a,
        "f2" => 0x3b,
        "f3" => 0x3c,
        "f4" => 0x3d,
        "f5" => 0x3e,
        "f6" => 0x3f,
        "f7" => 0x40,
        "f8" => 0x41,
        "f9" => 0x42,
        "f10" => 0x43,
        "f11" => 0x44,
        "f12" => 0x45,
        _ => return None,
    })
}

async fn wait_for_stop(stop: std::sync::Arc<std::sync::atomic::AtomicBool>) {
    while !stop.load(Ordering::Acquire) {
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

fn named_pipe_client_pid(pipe: HANDLE) -> Result<u32> {
    use windows::Win32::System::Pipes::GetNamedPipeClientProcessId;

    let mut pid = 0u32;
    unsafe {
        GetNamedPipeClientProcessId(pipe, &mut pid)?;
    }
    anyhow::ensure!(pid != 0, "named pipe client process ID is empty");
    Ok(pid)
}

/// Select the active interactive session, including RDP sessions. The
/// console-session API alone returns no useful target when the user is
/// connected through RDP.
fn active_interactive_session() -> Option<u32> {
    #[repr(C)]
    struct SessionInfo {
        session_id: u32,
        station_name: *mut u16,
        state: i32,
    }
    #[link(name = "wtsapi32")]
    unsafe extern "system" {
        fn WTSEnumerateSessionsW(
            server: *mut std::ffi::c_void,
            reserved: u32,
            version: u32,
            sessions: *mut *mut SessionInfo,
            count: *mut u32,
        ) -> i32;
        fn WTSFreeMemory(memory: *mut std::ffi::c_void);
    }
    const WTS_ACTIVE: i32 = 0;
    let mut sessions = std::ptr::null_mut();
    let mut count = 0u32;
    let ok =
        unsafe { WTSEnumerateSessionsW(std::ptr::null_mut(), 0, 1, &mut sessions, &mut count) }
            != 0;
    if !ok || sessions.is_null() {
        return None;
    }
    let entries = unsafe { std::slice::from_raw_parts(sessions, count as usize) };
    let mut console = None;
    let mut rdp = None;
    for entry in entries {
        if entry.state != WTS_ACTIVE {
            continue;
        }
        let name = if entry.station_name.is_null() {
            String::new()
        } else {
            let mut length = 0usize;
            unsafe {
                while *entry.station_name.add(length) != 0 && length < 256 {
                    length += 1;
                }
                String::from_utf16_lossy(std::slice::from_raw_parts(entry.station_name, length))
            }
        };
        if name.eq_ignore_ascii_case("Console") {
            console = Some(entry.session_id);
        } else if name.to_ascii_lowercase().starts_with("rdp-") {
            rdp = Some(entry.session_id);
        }
    }
    unsafe { WTSFreeMemory(sessions.cast()) };
    console.or(rdp)
}

fn inject_key_event(usage: u8, modifiers: u8, pressed: bool) -> Result<()> {
    anyhow::ensure!((0x04..=0x73).contains(&usage), "不支持的 HID usage");
    // All eight USB HID modifier bits are supported (left/right
    // Ctrl/Shift/Alt/GUI).
    use windows::Win32::UI::Input::KeyboardAndMouse::{INPUT, SendInput};
    if let Err(error) = attach_input_desktop() {
        tracing::error!(%error, usage, pressed, "inject_key_event: attach_input_desktop failed");
        return Err(error);
    }
    let desktop = get_thread_desktop_name().unwrap_or_else(|| "unknown".to_string());
    let (scan, extended) =
        usage_to_scancode(usage).ok_or_else(|| anyhow::anyhow!("不支持的 HID usage"))?;
    // Emit modifiers together with the key so a single request is
    // sufficient for shortcuts such as Ctrl+Alt+Del or Win+L.
    let modifier_scans = [
        (0x01, 0x1d, false),
        (0x02, 0x2a, false),
        (0x04, 0x38, false),
        (0x08, 0x5b, true),
        (0x10, 0x1d, true),
        (0x20, 0x36, false),
        (0x40, 0x38, true),
        (0x80, 0x5c, true),
    ];
    let mut inputs = Vec::with_capacity(9);
    if pressed {
        for (bit, modifier_scan, modifier_extended) in modifier_scans {
            if modifiers & bit != 0 {
                inputs.push(scan_input(modifier_scan, true, modifier_extended));
            }
        }
        inputs.push(scan_input(scan, true, extended));
    } else {
        inputs.push(scan_input(scan, false, extended));
        for (bit, modifier_scan, modifier_extended) in modifier_scans.into_iter().rev() {
            if modifiers & bit != 0 {
                inputs.push(scan_input(modifier_scan, false, modifier_extended));
            }
        }
    }
    tracing::debug!(%desktop, usage, scan, extended, modifiers, pressed, "inject_key_event: injecting key event");
    let inserted = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
    anyhow::ensure!(inserted == inputs.len() as u32, "SendInput 键盘事件失败");
    Ok(())
}

#[cfg(target_os = "windows")]
fn scan_input(
    scan: u16,
    pressed: bool,
    extended: bool,
) -> windows::Win32::UI::Input::KeyboardAndMouse::INPUT {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP,
        KEYEVENTF_SCANCODE, VIRTUAL_KEY,
    };
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(0),
                wScan: scan,
                dwFlags: KEYEVENTF_SCANCODE
                    | if extended {
                        KEYEVENTF_EXTENDEDKEY
                    } else {
                        Default::default()
                    }
                    | if pressed {
                        Default::default()
                    } else {
                        KEYEVENTF_KEYUP
                    },
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn inject_mouse_button(button: &str) -> Result<()> {
    anyhow::ensure!(matches!(button, "left" | "right"), "不支持的鼠标按键");
    tracing::debug!(button, "inject_mouse_button: injecting mouse click");
    inject_mouse_button_state(button, true)?;
    inject_mouse_button_state(button, false)
}

fn inject_mouse_button_state(button: &str, pressed: bool) -> Result<()> {
    anyhow::ensure!(matches!(button, "left" | "right"), "不支持的鼠标按键");

    // A button transition is a distinct SendInput event.  The old
    // implementation only changed MOUSE_BUTTONS and emitted a MOVE
    // report, which meant the release path never generated LEFTUP/
    // RIGHTUP and left the physical button logically held down.
    send_mouse_button_event(button, pressed)?;

    let mask = if button == "left" { 1 } else { 2 };
    if pressed {
        MOUSE_BUTTONS.fetch_or(mask, Ordering::SeqCst);
    } else {
        MOUSE_BUTTONS.fetch_and(!mask, Ordering::SeqCst);
    }
    let buttons = MOUSE_BUTTONS.load(Ordering::SeqCst);
    tracing::info!(
        button,
        pressed,
        buttons_state = buttons,
        "inject_mouse_button_state: button transition delivered"
    );
    Ok(())
}

fn send_mouse_button_event(button: &str, pressed: bool) -> Result<()> {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP,
        MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEINPUT, SendInput,
    };

    attach_input_desktop()?;
    let flags = match (button, pressed) {
        ("left", true) => MOUSEEVENTF_LEFTDOWN,
        ("left", false) => MOUSEEVENTF_LEFTUP,
        ("right", true) => MOUSEEVENTF_RIGHTDOWN,
        ("right", false) => MOUSEEVENTF_RIGHTUP,
        _ => anyhow::bail!("不支持的鼠标按键状态"),
    };
    let input = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: 0,
                dy: 0,
                mouseData: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    let inserted = unsafe { SendInput(&[input], std::mem::size_of::<INPUT>() as i32) };
    if inserted != 1 {
        tracing::error!(
            button,
            pressed,
            inserted,
            "SendInput mouse button transition failed"
        );
    }
    anyhow::ensure!(inserted == 1, "SendInput 鼠标按键状态失败");
    Ok(())
}

fn inject_mouse_wheel(delta: i32) -> Result<()> {
    let clamped = delta.clamp(-20, 20);
    tracing::debug!(delta, clamped, "inject_mouse_wheel: injecting wheel event");
    send_mouse_report(0, 0, clamped as i8)
}

fn get_thread_desktop_name() -> Option<String> {
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::StationsAndDesktops::{
        GetThreadDesktop, GetUserObjectInformationW, UOI_NAME,
    };
    use windows::Win32::System::Threading::GetCurrentThreadId;
    unsafe {
        let desktop = GetThreadDesktop(GetCurrentThreadId()).ok()?;
        let mut buffer = [0u16; 128];
        let mut needed = 0u32;
        let result = GetUserObjectInformationW(
            HANDLE(desktop.0),
            UOI_NAME,
            Some(buffer.as_mut_ptr().cast()),
            (buffer.len() * 2) as u32,
            Some(&mut needed),
        )
        .ok();
        result?;
        let end = buffer
            .iter()
            .position(|unit| *unit == 0)
            .unwrap_or(buffer.len());
        Some(String::from_utf16_lossy(&buffer[..end]))
    }
}

fn get_input_desktop_name() -> Option<String> {
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::StationsAndDesktops::{DESKTOP_ACCESS_FLAGS, DESKTOP_READOBJECTS};
    use windows::Win32::System::StationsAndDesktops::{
        GetUserObjectInformationW, OpenInputDesktop, UOI_NAME,
    };
    let access = DESKTOP_ACCESS_FLAGS(DESKTOP_READOBJECTS.0);
    unsafe {
        let desktop = OpenInputDesktop(Default::default(), false, access).ok()?;
        let mut buffer = [0u16; 128];
        let mut needed = 0u32;
        let result = GetUserObjectInformationW(
            HANDLE(desktop.0),
            UOI_NAME,
            Some(buffer.as_mut_ptr().cast()),
            (buffer.len() * 2) as u32,
            Some(&mut needed),
        )
        .ok();
        windows::Win32::System::StationsAndDesktops::CloseDesktop(desktop).ok();
        result?;
        let end = buffer
            .iter()
            .position(|unit| *unit == 0)
            .unwrap_or(buffer.len());
        Some(String::from_utf16_lossy(&buffer[..end]))
    }
}

fn attach_input_desktop() -> Result<()> {
    use windows::Win32::System::StationsAndDesktops::{
        CloseDesktop, DESKTOP_ACCESS_FLAGS, DESKTOP_CREATEMENU, DESKTOP_CREATEWINDOW,
        DESKTOP_ENUMERATE, DESKTOP_HOOKCONTROL, DESKTOP_READOBJECTS, DESKTOP_SWITCHDESKTOP,
        DESKTOP_WRITEOBJECTS, OpenInputDesktop, SetThreadDesktop,
    };
    let access = DESKTOP_ACCESS_FLAGS(
        DESKTOP_CREATEMENU.0
            | DESKTOP_CREATEWINDOW.0
            | DESKTOP_ENUMERATE.0
            | DESKTOP_HOOKCONTROL.0
            | DESKTOP_READOBJECTS.0
            | DESKTOP_WRITEOBJECTS.0
            | DESKTOP_SWITCHDESKTOP.0
            | GENERIC_WRITE.0,
    );
    let current_name = get_thread_desktop_name();
    let input_name = get_input_desktop_name();
    if let (Some(current), Some(input)) = (&current_name, &input_name) {
        if current.eq_ignore_ascii_case(input) {
            tracing::debug!(desktop = %current, "attach_input_desktop: already attached to input desktop");
            return Ok(());
        }
        tracing::info!(from = %current, to = %input, "attach_input_desktop: switching thread desktop");
    }
    let desktop = unsafe { OpenInputDesktop(Default::default(), false, access)? };
    // See attach_agent_desktop: this thread remains associated with the
    // selected desktop, so its handle must remain open.
    if let Err(error) = unsafe { SetThreadDesktop(desktop) } {
        unsafe {
            CloseDesktop(desktop).ok();
        }
        return Err(error.into());
    }
    Ok(())
}

fn send_mouse_report(dx: i32, dy: i32, wheel: i8) -> Result<()> {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_MOVE, MOUSEEVENTF_WHEEL, MOUSEINPUT, SendInput,
    };
    if let Err(error) = attach_input_desktop() {
        tracing::error!(%error, "send_mouse_report: attach_input_desktop failed");
        return Err(error);
    }
    let desktop = get_thread_desktop_name().unwrap_or_else(|| "unknown".to_string());
    tracing::debug!(%desktop, dx, dy, wheel, "send_mouse_report: injecting mouse input");
    let mut inputs = vec![INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: dx.clamp(-500, 500),
                dy: dy.clamp(-500, 500),
                mouseData: 0,
                dwFlags: MOUSEEVENTF_MOVE,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }];
    if wheel != 0 {
        inputs[0].Anonymous.mi.mouseData = (wheel as i32 * 120) as u32;
        inputs[0].Anonymous.mi.dwFlags = MOUSEEVENTF_WHEEL;
    }
    let inserted = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
    if inserted != inputs.len() as u32 {
        tracing::error!(
            dx,
            dy,
            wheel,
            inserted,
            expected = inputs.len(),
            "SendInput mouse report failed"
        );
    }
    anyhow::ensure!(inserted == inputs.len() as u32, "SendInput 鼠标事件失败");
    Ok(())
}

fn send_mouse_report_absolute(x: i32, y: i32) -> Result<()> {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_MOVE, MOUSEINPUT, SendInput,
    };
    if let Err(error) = attach_input_desktop() {
        tracing::error!(%error, "send_mouse_report_absolute: attach_input_desktop failed");
        return Err(error);
    }
    let desktop = get_thread_desktop_name().unwrap_or_else(|| "unknown".to_string());
    tracing::debug!(%desktop, x, y, "send_mouse_report_absolute: injecting absolute mouse input");
    let input = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: x,
                dy: y,
                mouseData: 0,
                dwFlags: MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_MOVE,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    let inserted = unsafe { SendInput(&[input], std::mem::size_of::<INPUT>() as i32) };
    if inserted != 1 {
        tracing::error!(x, y, inserted, "SendInput absolute mouse report failed");
    }
    anyhow::ensure!(inserted == 1, "SendInput 绝对定位鼠标事件失败");
    Ok(())
}

fn usage_to_scancode(usage: u8) -> Option<(u16, bool)> {
    // USB HID keyboard page 0x07 -> Set 1 scan codes.  Keep this table
    // explicit: it prevents malformed remote input from becoming an
    // arbitrary virtual-key injection while covering the normal PC
    // keyboard, function keys, navigation cluster and numpad.
    const TABLE: &[(u8, u16, bool)] = &[
        (0x04, 0x1e, false),
        (0x05, 0x30, false),
        (0x06, 0x2e, false),
        (0x07, 0x20, false),
        (0x08, 0x12, false),
        (0x09, 0x21, false),
        (0x0a, 0x22, false),
        (0x0b, 0x23, false),
        (0x0c, 0x17, false),
        (0x0d, 0x24, false),
        (0x0e, 0x25, false),
        (0x0f, 0x26, false),
        (0x10, 0x32, false),
        (0x11, 0x31, false),
        (0x12, 0x18, false),
        (0x13, 0x19, false),
        (0x14, 0x10, false),
        (0x15, 0x13, false),
        (0x16, 0x1f, false),
        (0x17, 0x14, false),
        (0x18, 0x16, false),
        (0x19, 0x2f, false),
        (0x1a, 0x11, false),
        (0x1b, 0x2d, false),
        (0x1c, 0x15, false),
        (0x1d, 0x2c, false),
        (0x1e, 0x02, false),
        (0x1f, 0x03, false),
        (0x20, 0x04, false),
        (0x21, 0x05, false),
        (0x22, 0x06, false),
        (0x23, 0x07, false),
        (0x24, 0x08, false),
        (0x25, 0x09, false),
        (0x26, 0x0a, false),
        (0x27, 0x0b, false),
        (0x28, 0x1c, false),
        (0x29, 0x01, false),
        (0x2a, 0x0e, false),
        (0x2b, 0x0f, false),
        (0x2c, 0x39, false),
        (0x2d, 0x0c, false),
        (0x2e, 0x0d, false),
        (0x2f, 0x1a, false),
        (0x30, 0x1b, false),
        (0x31, 0x2b, false),
        (0x33, 0x27, false),
        (0x34, 0x28, false),
        (0x35, 0x29, false),
        (0x36, 0x33, false),
        (0x37, 0x34, false),
        (0x38, 0x35, false),
        (0x39, 0x3a, false),
        (0x3a, 0x3b, false),
        (0x3b, 0x3c, false),
        (0x3c, 0x3d, false),
        (0x3d, 0x3e, false),
        (0x3e, 0x3f, false),
        (0x3f, 0x40, false),
        (0x40, 0x41, false),
        (0x41, 0x42, false),
        (0x42, 0x43, false),
        (0x43, 0x44, false),
        (0x44, 0x57, false),
        (0x45, 0x58, false),
        (0x4a, 0x47, true),
        (0x4b, 0x49, true),
        (0x4c, 0x53, true),
        (0x4d, 0x4f, true),
        (0x4e, 0x51, true),
        (0x4f, 0x4d, true),
        (0x50, 0x4b, true),
        (0x51, 0x50, true),
        (0x52, 0x48, true),
        (0x53, 0x45, false),
        (0x54, 0x35, true),
        (0x55, 0x37, false),
        (0x56, 0x4a, false),
        (0x57, 0x4e, false),
        (0x58, 0x4f, false),
        (0x59, 0x47, false),
        (0x5a, 0x48, false),
        (0x5b, 0x49, false),
        (0x5c, 0x4b, false),
        (0x5d, 0x4c, false),
        (0x5e, 0x4d, false),
        (0x5f, 0x4f, false),
        (0x60, 0x50, false),
        (0x61, 0x51, false),
        (0x62, 0x52, false),
        (0x63, 0x53, false),
    ];
    TABLE
        .iter()
        .find_map(|(candidate, scan, extended)| (*candidate == usage).then_some((*scan, *extended)))
}

fn write_response(stream: &mut std::fs::File, ok: bool, code: &str, message: &str) -> Result<()> {
    let payload = serde_json::to_vec(&Response {
        ok,
        code,
        message,
        request_id: None,
    })?;
    stream.write_all(&(payload.len() as u32).to_le_bytes())?;
    stream.write_all(&payload)?;
    stream.flush()?;
    Ok(())
}

fn write_agent_status_response(
    stream: &mut std::fs::File,
    status: &qinglink_windows::secure_desktop::SecureDesktopStatus,
) -> Result<()> {
    #[derive(Serialize)]
    struct AgentStatus<'a> {
        ok: bool,
        code: &'a str,
        message: &'a str,
        desktop: &'a str,
        locked: bool,
        uac_detected: bool,
        input_ready: bool,
        session_id: u32,
        token_integrity_level: u32,
        agent_state: &'a str,
        last_error: &'a str,
    }
    let message = "QingLink 会话代理已就绪";
    let payload = serde_json::to_vec(&AgentStatus {
        ok: status.input_ready,
        code: if status.input_ready {
            "AGENT_READY"
        } else {
            "AGENT_NOT_READY"
        },
        message,
        desktop: &status.desktop,
        locked: status.locked,
        uac_detected: status.uac_detected,
        input_ready: status.input_ready,
        session_id: status.session_id,
        token_integrity_level: status.token_integrity_level,
        agent_state: &status.agent_state,
        last_error: &status.last_error,
    })?;
    stream.write_all(&(payload.len() as u32).to_le_bytes())?;
    stream.write_all(&payload)?;
    stream.flush()?;
    Ok(())
}

fn write_response_with_id(
    stream: &mut std::fs::File,
    ok: bool,
    code: &str,
    message: &str,
    request_id: Option<&str>,
) -> Result<()> {
    let payload = serde_json::to_vec(&Response {
        ok,
        code,
        message,
        request_id,
    })?;
    stream.write_all(&(payload.len() as u32).to_le_bytes())?;
    stream.write_all(&payload)?;
    stream.flush()?;
    Ok(())
}
