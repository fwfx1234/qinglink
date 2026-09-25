#![cfg(target_os = "windows")]

use std::io::{Read, Write};
use std::os::windows::io::FromRawHandle;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use anyhow::Result;
use serde::{Deserialize, Serialize};

const PIPE_NAME: &str = r"\\.\pipe\QingLinkControlV2";
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(2);

#[derive(Debug, Serialize)]
struct PipeRequest {
    version: u32,
    request_id: String,
    command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    parent_pid: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct PipeResponse {
    ok: bool,
    code: String,
    message: String,
    #[serde(default)]
    request_id: Option<String>,
}

pub struct ServiceLifecycleManager {
    stop_signal: Arc<AtomicBool>,
    heartbeat_handle: Option<std::thread::JoinHandle<()>>,
    registered: bool,
}

impl ServiceLifecycleManager {
    pub fn new() -> Self {
        Self {
            stop_signal: Arc::new(AtomicBool::new(false)),
            heartbeat_handle: None,
            registered: false,
        }
    }

    pub fn start(&mut self) -> Result<()> {
        self.start_internal(false)
    }

    pub fn try_start(&mut self) -> Result<()> {
        self.start_internal(true)
    }

    fn start_internal(&mut self, silent: bool) -> Result<()> {
        let state = super::input_install::inspect();
        tracing::info!(
            service_installed = state.service_installed,
            service_running = state.service_running,
            service_binary_matches = state.service_binary_matches,
            service_available = state.available,
            detail = %state.detail,
            silent,
            "检查 Windows 服务状态"
        );

        if !state.service_installed {
            tracing::info!("Windows 服务未安装，跳过服务生命周期管理");
            return Ok(());
        }

        if !state.service_binary_matches {
            tracing::warn!(
                detail = %state.detail,
                "Windows 服务版本过旧或无法校验，跳过旧服务注册；请执行安全输入组件修复"
            );
            return Ok(());
        }

        if !state.service_running {
            if silent {
                tracing::info!("Windows 服务未运行（静默模式），尝试使用服务 ACL 静默启动");
                if let Err(error) = super::input_install::start_service_direct() {
                    tracing::warn!(%error, "静默启动 Windows 服务失败");
                    return Ok(());
                }
            } else {
                tracing::info!("Windows 服务未运行，尝试启动...");
                match super::input_install::start_service_elevated() {
                    Ok(msg) => tracing::info!(msg, "服务启动命令已执行"),
                    Err(error) => {
                        tracing::warn!(%error, "启动 Windows 服务失败，安全桌面输入功能将不可用");
                        return Ok(());
                    }
                }
            }

            if !Self::wait_for_network_ready(30) {
                tracing::warn!("Windows 服务进程已运行，但 TCP 3721 未就绪");
                if !silent {
                    tracing::info!("尝试重启未就绪的 Windows 服务");
                    let _ = super::input_install::restart_service_elevated();
                    if !Self::wait_for_network_ready(30) {
                        anyhow::bail!("Windows 服务启动后健康检查失败（TCP 3721 未监听）");
                    }
                } else {
                    return Ok(());
                }
            }
        } else if !Self::wait_for_network_ready(30) {
            anyhow::bail!("Windows 服务处于 RUNNING 但 TCP 3721 未就绪");
        }

        if let Err(error) = self.register_parent() {
            tracing::warn!(%error, "注册桌面端租约失败，服务不会自动跟随桌面端退出");
        } else {
            self.registered = true;
            self.start_heartbeat_thread();
        }

        tracing::info!("服务生命周期管理器已启动");
        Ok(())
    }

    fn wait_for_network_ready(max_attempts: u32) -> bool {
        for attempt in 0..max_attempts {
            if super::input_install::service_network_ready() {
                tracing::info!(attempt = attempt + 1, "Windows 服务健康检查通过");
                return true;
            }
            let delay = Duration::from_millis((250 * (1u64 << attempt.min(3))).min(2000));
            std::thread::sleep(delay);
        }
        false
    }

    fn connect_pipe() -> Result<std::fs::File> {
        let pipe_path = PIPE_NAME;
        let wide_path: Vec<u16> = pipe_path.encode_utf16().chain([0]).collect();

        use windows::Win32::Foundation::GENERIC_READ;
        use windows::Win32::Foundation::GENERIC_WRITE;
        use windows::Win32::Foundation::GetLastError;
        use windows::Win32::Storage::FileSystem::{
            CreateFileW, FILE_FLAGS_AND_ATTRIBUTES, FILE_SHARE_NONE, OPEN_EXISTING,
        };

        let desired_access = GENERIC_READ.0 | GENERIC_WRITE.0;
        let handle = unsafe {
            CreateFileW(
                windows::core::PCWSTR(wide_path.as_ptr()),
                desired_access,
                FILE_SHARE_NONE,
                None,
                OPEN_EXISTING,
                FILE_FLAGS_AND_ATTRIBUTES(0),
                None,
            )
        };

        match handle {
            Ok(handle) => {
                let file = unsafe { std::fs::File::from_raw_handle(handle.0 as _) };
                Ok(file)
            }
            Err(_) => {
                let last_error = unsafe { GetLastError() };
                let error_msg = match last_error.0 {
                    2 => "服务未运行 (ERROR_FILE_NOT_FOUND)".to_string(),
                    5 => "访问被拒绝 (ERROR_ACCESS_DENIED)，可能需要管理员权限".to_string(),
                    32 => "文件被其他进程占用 (ERROR_SHARING_VIOLATION)".to_string(),
                    _ => format!("Windows 错误码: {}", last_error.0),
                };
                anyhow::bail!("无法连接到服务 Named Pipe: {}", error_msg)
            }
        }
    }

    fn send_request(request: &PipeRequest) -> Result<PipeResponse> {
        let mut file = Self::connect_pipe()?;

        let payload = serde_json::to_vec(request)?;
        let length = (payload.len() as u32).to_le_bytes();

        file.write_all(&length)?;
        file.write_all(&payload)?;
        file.flush()?;

        let mut length_buf = [0u8; 4];
        file.read_exact(&mut length_buf)?;
        let response_len = u32::from_le_bytes(length_buf) as usize;

        if response_len == 0 || response_len > 64 * 1024 {
            anyhow::bail!("无效的响应大小: {}", response_len);
        }

        let mut response_buf = vec![0u8; response_len];
        file.read_exact(&mut response_buf)?;

        let response: PipeResponse = serde_json::from_slice(&response_buf)?;

        std::mem::forget(file);
        Ok(response)
    }

    fn register_parent(&self) -> Result<()> {
        Self::register_parent_request(std::process::id())
    }

    fn register_parent_request(pid: u32) -> Result<()> {
        let request = PipeRequest {
            version: 2,
            request_id: format!(
                "desktop-{}-{}",
                std::process::id(),
                time::OffsetDateTime::now_utc().unix_timestamp_nanos()
            ),
            command: "register_parent".into(),
            parent_pid: Some(pid),
        };

        let mut last_error = None;
        for attempt in 0..10 {
            match Self::send_request(&request) {
                Ok(response) => {
                    if response.ok {
                        tracing::info!(pid, attempt = attempt + 1, "已向服务注册父进程 PID");
                        return Ok(());
                    } else {
                        last_error = Some(format!("{} - {}", response.code, response.message));
                        tracing::warn!(attempt = attempt + 1, code = %response.code, message = %response.message, "注册父进程被拒绝");
                    }
                }
                Err(e) => {
                    last_error = Some(e.to_string());
                    tracing::warn!(attempt = attempt + 1, error = %e, "连接服务管道失败");
                }
            }
            if attempt < 9 {
                let delay_ms = 200 * (1_u64 << attempt.min(4));
                std::thread::sleep(Duration::from_millis(delay_ms));
            }
        }
        anyhow::bail!(
            "无法注册父进程到服务（已重试10次）: {}",
            last_error.unwrap_or_else(|| "未知错误".into())
        )
    }

    fn start_heartbeat_thread(&mut self) {
        let stop = Arc::clone(&self.stop_signal);

        let handle = std::thread::spawn(move || {
            let mut consecutive_failures = 0_u32;
            while !stop.load(Ordering::Acquire) {
                std::thread::sleep(HEARTBEAT_INTERVAL);

                if stop.load(Ordering::Acquire) {
                    break;
                }

                let request = PipeRequest {
                    version: 2,
                    request_id: format!(
                        "heartbeat-{}-{}",
                        std::process::id(),
                        time::OffsetDateTime::now_utc().unix_timestamp_nanos()
                    ),
                    command: "heartbeat".into(),
                    parent_pid: None,
                };

                match Self::send_request(&request) {
                    Ok(response) => {
                        if response.ok {
                            if consecutive_failures > 0 {
                                tracing::info!(consecutive_failures, "心跳恢复成功");
                            }
                            consecutive_failures = 0;
                        } else if response.code == "UNREGISTERED_CLIENT" {
                            tracing::warn!(
                                consecutive_failures,
                                "服务已重启或丢失桌面端租约，重新注册父进程"
                            );
                            match Self::register_parent_request(std::process::id()) {
                                Ok(()) => {
                                    consecutive_failures = 0;
                                    tracing::info!("桌面端父进程租约重新注册成功");
                                }
                                Err(error) => {
                                    consecutive_failures += 1;
                                    tracing::warn!(%error, consecutive_failures, "重新注册桌面端父进程失败");
                                }
                            }
                        } else {
                            consecutive_failures += 1;
                            tracing::warn!(
                                consecutive_failures,
                                "心跳响应异常: {}",
                                response.message
                            );
                        }
                    }
                    Err(e) => {
                        consecutive_failures += 1;
                        tracing::warn!(consecutive_failures, error = %e, "心跳发送失败");
                        if consecutive_failures > 5 {
                            std::thread::sleep(Duration::from_secs(2));
                        }
                    }
                }
            }
        });

        self.heartbeat_handle = Some(handle);
    }

    pub fn shutdown(mut self) -> Result<()> {
        self.stop_signal.store(true, Ordering::Release);

        if let Some(handle) = self.heartbeat_handle.take() {
            let _ = handle.join();
        }

        if !self.registered {
            return Ok(());
        }

        let request = PipeRequest {
            version: 2,
            request_id: format!(
                "shutdown-{}-{}",
                std::process::id(),
                time::OffsetDateTime::now_utc().unix_timestamp_nanos()
            ),
            command: "shutdown".into(),
            parent_pid: None,
        };

        match Self::send_request(&request) {
            Ok(response) => {
                if response.ok {
                    tracing::info!("已发送 Shutdown 命令到服务");
                } else {
                    tracing::warn!("Shutdown 命令响应异常: {}", response.message);
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, "发送 Shutdown 命令失败");
            }
        }

        tracing::info!("服务生命周期管理器已关闭");
        Ok(())
    }
}

impl Drop for ServiceLifecycleManager {
    fn drop(&mut self) {
        self.stop_signal.store(true, Ordering::Release);
        if let Some(handle) = self.heartbeat_handle.take() {
            let _ = handle.join();
        }
    }
}
