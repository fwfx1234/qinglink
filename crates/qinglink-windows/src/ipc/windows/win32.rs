//! 事件句柄、等待与截止时间预算的 Win32 薄封装。

use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail, ensure};
use windows::Win32::Foundation::{HANDLE, WAIT_FAILED, WAIT_OBJECT_0, WAIT_TIMEOUT};
use windows::Win32::System::Threading::{CreateEventW, WaitForSingleObject};
use windows::core::{Error as WindowsError, PCWSTR};

/// 单次等待使用的时间片；轮询间隔不超过该值。
pub(super) const WAIT_SLICE: Duration = Duration::from_millis(10);
/// 客户端连接重试占用的总时长上限，包含在调用方的整体预算内。
pub(super) const CONNECT_LIMIT: Duration = Duration::from_secs(2);

/// 检查取消标志与绝对截止时间，两者任一失效即返回错误。
pub(super) fn check_budget(deadline: Instant, cancelled: &AtomicBool) -> Result<()> {
    ensure!(
        !cancelled.load(Ordering::Acquire),
        "pipe operation cancelled"
    );
    ensure!(Instant::now() < deadline, "pipe operation timed out");
    Ok(())
}

/// 创建一个私有的手动重置、初始未触发的事件句柄。
pub(super) fn create_event() -> Result<OwnedHandle> {
    // SAFETY: unnamed manual-reset event, no inherited security attributes and
    // no borrowed pointers. On success this is a new, uniquely owned handle.
    let handle = unsafe { CreateEventW(None, true, false, PCWSTR::null()) }
        .context("cannot create pipe I/O event")?;
    // SAFETY: ownership is transferred once from the successful API result.
    Ok(unsafe { OwnedHandle::from_raw_handle(handle.0) })
}

/// 把拥有所有权的句柄借用为裸 `HANDLE`，只用于传给 Win32 调用。
pub(super) fn raw(handle: &OwnedHandle) -> HANDLE {
    HANDLE(handle.as_raw_handle())
}

/// 在剩余预算内等待事件，单次最长一个时间片；超时按正常情况返回。
pub(super) fn wait_event(
    event: &OwnedHandle,
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Result<()> {
    check_budget(deadline, cancelled)?;
    let remaining = deadline.saturating_duration_since(Instant::now());
    // Round down: a sub-millisecond remainder uses a nonblocking poll, never an
    // infinite/default timeout. Each wait remains within the remaining budget.
    let millis = remaining.min(WAIT_SLICE).as_millis() as u32;
    // SAFETY: the event is owned and remains open for the complete wait.
    let status = unsafe { WaitForSingleObject(raw(event), millis) };
    match status {
        WAIT_OBJECT_0 | WAIT_TIMEOUT => Ok(()),
        WAIT_FAILED => Err(WindowsError::from_win32()).context("cannot wait for pipe I/O event"),
        _ => bail!("unexpected pipe I/O event wait result: {}", status.0),
    }
}
