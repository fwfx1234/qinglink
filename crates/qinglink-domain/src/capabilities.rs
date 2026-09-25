use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CapabilityError {
    #[error("capability is not supported on this platform")]
    Unsupported,
    #[error("platform operation failed: {0}")]
    Failed(String),
}

pub trait PowerControl: Send + Sync {
    fn shutdown(&self, force: bool, delay_secs: u64) -> Result<(), CapabilityError>;
    fn sleep(&self, hibernate: bool) -> Result<(), CapabilityError>;
    fn restart(&self, force: bool, delay_secs: u64) -> Result<(), CapabilityError>;
    fn logoff(&self) -> Result<(), CapabilityError>;
    fn lock(&self) -> Result<(), CapabilityError>;
}

pub trait ProcessControl: Send + Sync {
    fn suspend(&self, pid: u32) -> Result<(), CapabilityError>;
    fn resume(&self, pid: u32) -> Result<(), CapabilityError>;
    fn kill(&self, pid: u32) -> Result<(), CapabilityError>;
    fn set_priority(&self, pid: u32, priority: &str) -> Result<(), CapabilityError>;
}
