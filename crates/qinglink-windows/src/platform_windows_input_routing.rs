//! 输入投递路由：安全桌面路径判定与经由输入服务的 HID 回退。

use super::lock_input_pipe::{LockInputPipeRequest, lock_input_pipe};

/// 校验当前桌面允许远端输入注入。
///
/// 安全桌面限制被刻意关闭：Windows 锁屏期间仍允许远端输入，因此这里总是成功。
pub(super) fn ensure_remote_input_desktop() -> anyhow::Result<()> {
    // Secure-desktop restrictions are intentionally disabled: remote input is
    // allowed even while Windows is locked.
    Ok(())
}

/// 通过输入服务管道重放一次输入请求，并把服务返回的失败翻译成错误。
///
/// # Errors
/// 管道不可用/读写或解析失败、或服务返回 `ok == false`（“{code}: {message}”）时
/// 返回错误。
pub(super) fn require_hid_input(request: &LockInputPipeRequest<'_>) -> anyhow::Result<()> {
    let response = lock_input_pipe(request)?;
    anyhow::ensure!(response.ok, "{}: {}", response.code, response.message);
    Ok(())
}
