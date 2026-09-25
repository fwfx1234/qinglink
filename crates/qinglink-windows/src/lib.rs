pub mod audio;
pub mod autostart;
pub mod display;
pub mod elevation;
pub mod firewall;
pub mod input;
#[cfg(target_os = "windows")]
pub mod platform_windows;
pub mod power;
pub mod process;
pub mod secure_desktop;
pub mod session;
pub mod single_instance;
pub mod window;

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

#[derive(Debug, thiserror::Error)]
pub enum WindowsCapabilityError {
    #[error("{0} is only supported on Windows")]
    Unsupported(&'static str),
    #[error("Windows operation failed: {0}")]
    Platform(String),
}

pub type Result<T> = std::result::Result<T, WindowsCapabilityError>;

pub const fn is_supported() -> bool {
    cfg!(target_os = "windows")
}
