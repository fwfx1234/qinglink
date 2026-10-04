//! 进程操作：挂起、恢复、结束进程，以及进程信息读取与优先级设置。

use super::platform_impl;

use crate::protocol::responses::ProcessInfo;

/// 进程操作入口，提供挂起、恢复与结束进程的能力。
#[derive(Debug)]
pub struct ProcessActions;

impl ProcessActions {
    /// 挂起指定进程。
    ///
    /// # Errors
    ///
    /// 当底层 `OpenProcess` 失败（进程不存在或权限不足）或 `NtSuspendProcess`
    /// 返回失败状态时返回 `Err`；非 Windows 平台恒返回不支持错误。
    pub fn suspend(pid: u32) -> anyhow::Result<()> {
        platform_impl::suspend(pid)
    }

    /// 恢复被挂起的指定进程。
    ///
    /// # Errors
    ///
    /// 当底层 `OpenProcess` 失败（进程不存在或权限不足）或 `NtResumeProcess`
    /// 返回失败状态时返回 `Err`；非 Windows 平台恒返回不支持错误。
    pub fn resume(pid: u32) -> anyhow::Result<()> {
        platform_impl::resume(pid)
    }

    /// 强制结束指定进程。
    ///
    /// # Errors
    ///
    /// 当底层 `OpenProcess`（需要 `PROCESS_TERMINATE` 权限）失败或
    /// `TerminateProcess` 失败时返回 `Err`；非 Windows 平台恒返回不支持错误。
    pub fn kill(pid: u32) -> anyhow::Result<()> {
        platform_impl::kill(pid)
    }
}

/// 读取系统进程信息并转换为协议响应结构。
pub fn get_process_info(process: &sysinfo::Process) -> ProcessInfo {
    platform_impl::get_process_info(process)
}

/// 设置指定进程的调度优先级。
///
/// # Errors
///
/// 当底层 `OpenProcess` 失败（进程不存在或权限不足）或 `SetPriorityClass`
/// 失败时返回 `Err`；无法识别的优先级名称按 normal 处理，不视为错误。
/// 非 Windows 平台恒返回不支持错误。
pub fn set_priority(pid: u32, priority: &str) -> anyhow::Result<()> {
    platform_impl::set_priority(pid, priority)
}
