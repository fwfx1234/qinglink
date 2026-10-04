//! qinglink 的 Windows 平台能力实现：电源、进程、窗口、显示、音频、输入与会话控制。

/// 音频输出设备枚举与音量控制。
pub mod audio;
/// 开机自启动注册项的读写。
pub mod autostart;
pub mod browser;
/// 显示器模式查询与切换。
pub mod display;
/// 进程提权状态探测。
pub mod elevation;
/// Windows 防火墙规则管理。
pub mod firewall;
/// 键鼠输入注入。
pub mod input;
pub mod integrity;
pub mod ipc;
/// 基于 Win32 API 的 Windows 平台能力实现聚合模块。
#[cfg(target_os = "windows")]
pub mod platform_windows;
/// 系统电源操作（关机、重启、休眠、锁定）。
pub mod power;
/// 进程控制（挂起、恢复、结束、优先级）。
pub mod process;
pub mod secure_desktop;
/// 用户会话查询与解锁状态。
pub mod session;
pub mod single_instance;
/// 窗口枚举与操作。
pub mod window;

/// 通过 `qinglink-domain` 的 `PowerControl` 暴露 Windows 电源操作的门面类型。
#[derive(Debug)]
pub struct WindowsPower;

impl qinglink_domain::PowerControl for WindowsPower {
    fn shutdown(
        &self,
        force: bool,
        delay_secs: u64,
    ) -> std::result::Result<(), qinglink_domain::CapabilityError> {
        power::shutdown(force, delay_secs)
            .map_err(|error| qinglink_domain::CapabilityError::Failed(error.to_string()))
    }

    fn sleep(&self, hibernate: bool) -> std::result::Result<(), qinglink_domain::CapabilityError> {
        power::sleep(hibernate)
            .map_err(|error| qinglink_domain::CapabilityError::Failed(error.to_string()))
    }

    fn restart(
        &self,
        force: bool,
        delay_secs: u64,
    ) -> std::result::Result<(), qinglink_domain::CapabilityError> {
        power::restart(force, delay_secs)
            .map_err(|error| qinglink_domain::CapabilityError::Failed(error.to_string()))
    }

    fn logoff(&self) -> std::result::Result<(), qinglink_domain::CapabilityError> {
        power::logoff().map_err(|error| qinglink_domain::CapabilityError::Failed(error.to_string()))
    }

    fn lock(&self) -> std::result::Result<(), qinglink_domain::CapabilityError> {
        power::lock().map_err(|error| qinglink_domain::CapabilityError::Failed(error.to_string()))
    }
}

/// 通过 `qinglink-domain` 的 `ProcessControl` 暴露 Windows 进程操作的门面类型。
#[derive(Debug)]
pub struct WindowsProcess;

impl qinglink_domain::ProcessControl for WindowsProcess {
    fn suspend(&self, pid: u32) -> std::result::Result<(), qinglink_domain::CapabilityError> {
        process::suspend(pid)
            .map_err(|error| qinglink_domain::CapabilityError::Failed(error.to_string()))
    }

    fn resume(&self, pid: u32) -> std::result::Result<(), qinglink_domain::CapabilityError> {
        process::resume(pid)
            .map_err(|error| qinglink_domain::CapabilityError::Failed(error.to_string()))
    }

    fn kill(&self, pid: u32) -> std::result::Result<(), qinglink_domain::CapabilityError> {
        process::kill(pid)
            .map_err(|error| qinglink_domain::CapabilityError::Failed(error.to_string()))
    }

    fn set_priority(
        &self,
        pid: u32,
        priority: &str,
    ) -> std::result::Result<(), qinglink_domain::CapabilityError> {
        process::set_priority(pid, priority)
            .map_err(|error| qinglink_domain::CapabilityError::Failed(error.to_string()))
    }
}

/// Windows 平台能力调用可能返回的错误。
#[derive(Debug, thiserror::Error)]
pub enum WindowsCapabilityError {
    /// 当前能力在非 Windows 平台上不受支持，携带该能力的名称。
    #[error("{0} is only supported on Windows")]
    Unsupported(&'static str),
    /// Windows API 调用失败，携带底层错误描述。
    #[error("Windows operation failed: {0}")]
    Platform(String),
}

/// 本 crate 各 Windows 能力函数的统一返回类型。
pub type Result<T> = std::result::Result<T, WindowsCapabilityError>;

/// 判断当前编译目标是否为 Windows；非 Windows 目标下所有能力均不可用。
pub const fn is_supported() -> bool {
    cfg!(target_os = "windows")
}
