//! 进程控制动作：挂起、恢复与结束进程，转发到 `crate::process`。

/// 挂起指定进程，转发到 `crate::process::suspend`。
///
/// # Errors
/// 透传 [`crate::process::suspend`] 的错误：打开进程失败、进程句柄无效，或
/// `NtSuspendProcess` 调用失败时返回错误。
pub fn suspend(pid: u32) -> anyhow::Result<()> {
    crate::process::suspend(pid)
}

/// 恢复指定进程，转发到 `crate::process::resume`。
///
/// # Errors
/// 透传 [`crate::process::resume`] 的错误：打开进程失败、进程句柄无效，或
/// `NtResumeProcess` 调用失败时返回错误。
pub fn resume(pid: u32) -> anyhow::Result<()> {
    crate::process::resume(pid)
}

/// 结束指定进程，转发到 `crate::process::kill`。
///
/// # Errors
/// 透传 [`crate::process::kill`] 的错误：打开进程失败、进程句柄无效、
/// `TerminateProcess` 失败或关闭句柄失败时返回错误。
pub fn kill(pid: u32) -> anyhow::Result<()> {
    crate::process::kill(pid)
}

/// Set process priority (normal, high, low, etc.)
///
/// # Errors
/// 透传 [`crate::process::set_priority`] 的错误：打开进程失败、进程句柄无效、
/// `SetPriorityClass` 失败或关闭句柄失败时返回错误；无法识别的 `priority` 不报错，
/// 按 normal 处理。
pub fn set_priority(pid: u32, priority: &str) -> anyhow::Result<()> {
    crate::process::set_priority(pid, priority)
}
