use thiserror::Error;

/// 平台能力调用的失败原因。
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CapabilityError {
    /// 当前平台不支持该能力。
    #[error("capability is not supported on this platform")]
    Unsupported,
    /// 平台调用返回失败，附原始错误描述。
    #[error("platform operation failed: {0}")]
    Failed(String),
}

/// 电源控制能力：关机、睡眠、重启、注销与锁屏。
pub trait PowerControl: Send + Sync {
    /// 关机；`force` 强制结束未响应的应用，`delay_secs` 为延迟秒数。
    ///
    /// # Errors
    ///
    /// 平台不支持该能力，或系统调用失败时返回 [`CapabilityError`]。
    fn shutdown(&self, force: bool, delay_secs: u64) -> Result<(), CapabilityError>;
    /// 进入睡眠；`hibernate` 为 `true` 时改为休眠（挂起到磁盘）。
    ///
    /// # Errors
    ///
    /// 平台不支持该能力，或系统调用失败时返回 [`CapabilityError`]。
    fn sleep(&self, hibernate: bool) -> Result<(), CapabilityError>;
    /// 重启；`force` 与 `delay_secs` 的含义同 [`PowerControl::shutdown`]。
    ///
    /// # Errors
    ///
    /// 平台不支持该能力，或系统调用失败时返回 [`CapabilityError`]。
    fn restart(&self, force: bool, delay_secs: u64) -> Result<(), CapabilityError>;
    /// 注销当前交互用户。
    ///
    /// # Errors
    ///
    /// 平台不支持该能力，或系统调用失败时返回 [`CapabilityError`]。
    fn logoff(&self) -> Result<(), CapabilityError>;
    /// 锁定当前会话。
    ///
    /// # Errors
    ///
    /// 平台不支持该能力，或系统调用失败时返回 [`CapabilityError`]。
    fn lock(&self) -> Result<(), CapabilityError>;
}

/// 进程控制能力：挂起、恢复、强制结束与调整优先级。
pub trait ProcessControl: Send + Sync {
    /// 挂起指定进程的所有线程。
    ///
    /// # Errors
    ///
    /// 平台不支持该能力，或系统调用失败时返回 [`CapabilityError`]。
    fn suspend(&self, pid: u32) -> Result<(), CapabilityError>;
    /// 恢复先前被挂起的进程。
    ///
    /// # Errors
    ///
    /// 平台不支持该能力，或系统调用失败时返回 [`CapabilityError`]。
    fn resume(&self, pid: u32) -> Result<(), CapabilityError>;
    /// 强制结束指定进程。
    ///
    /// # Errors
    ///
    /// 平台不支持该能力，或系统调用失败时返回 [`CapabilityError`]。
    fn kill(&self, pid: u32) -> Result<(), CapabilityError>;
    /// 设置进程优先级类，`priority` 取 `idle`/`below_normal`/`normal`/
    /// `above_normal`/`realtime` 等名称，无法识别时按 `normal` 处理。
    ///
    /// # Errors
    ///
    /// 平台不支持该能力，或系统调用失败时返回 [`CapabilityError`]。
    fn set_priority(&self, pid: u32, priority: &str) -> Result<(), CapabilityError>;
}
