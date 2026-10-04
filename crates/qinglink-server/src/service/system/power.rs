//! 电源控制：关机、重启与睡眠 / 休眠，全部委托给平台层实现。

use crate::platform::SystemController;

use super::SystemService;

impl SystemService {
    /// 关机；`force` 为真时强制结束未保存的程序，`delay_secs` 为延迟秒数。
    ///
    /// # Errors
    /// `delay_secs` 超出 Windows 的延迟上限、无法启用 `SeShutdownPrivilege`
    /// （权限不足），或 `InitiateSystemShutdownExW` 调用失败时返回 `Err`；
    /// 非 Windows 平台恒返回“仅 Windows 支持”错误。
    pub fn shutdown(&self, force: bool, delay_secs: u64) -> anyhow::Result<()> {
        SystemController::shutdown(force, delay_secs)
    }

    /// 进入睡眠（`hibernate` 为假）或休眠（为真）。
    ///
    /// # Errors
    /// `SetSuspendState` 调用失败时返回 `Err`；非 Windows 平台恒返回
    /// “仅 Windows 支持”错误。
    pub fn sleep(&self, hibernate: bool) -> anyhow::Result<()> {
        SystemController::sleep(hibernate)
    }

    /// 重启系统，`force` 与 `delay_secs` 语义同 [`Self::shutdown`]。
    ///
    /// # Errors
    /// 与 [`Self::shutdown`] 相同：`delay_secs` 超出延迟上限、无法启用
    /// `SeShutdownPrivilege`，或 `InitiateSystemShutdownExW`（重启）调用失败时
    /// 返回 `Err`；非 Windows 平台恒返回“仅 Windows 支持”错误。
    pub fn restart(&self, force: bool, delay_secs: u64) -> anyhow::Result<()> {
        SystemController::restart(force, delay_secs)
    }
}
