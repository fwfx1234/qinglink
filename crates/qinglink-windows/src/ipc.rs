//! 有界、可取消的本地 Windows 命名管道 JSON 传输。
//!
//! 帧由四字节小端负载长度加 UTF-8 JSON 组成。两个方向的负载都不得超过
//! [`MAX_FRAME_SIZE`] 字节。传输错误里不包含任何负载或 serde 错误文本：它们可能
//! 含有凭据。本传输不认证对端；管道 ACL 与应用层授权仍由调用方负责。
//!
//! Windows 操作使用重叠 I/O、单一绝对截止时间与短暂的事件等待。取消会请求
//! `CancelIoEx`，然后先观察到完成，再释放任何内核可见内存。若 Windows 迟迟不完成
//! 取消，这项必须执行的清理可能超出截止时间；但不会有工作线程被 detach，也不会有
//! 缓冲区或句柄泄漏。序列化/反序列化同步执行，无法抢占用户提供的任意 serde 实现。

use std::sync::atomic::AtomicBool;
use std::time::Duration;

use anyhow::Result;
use serde::{Serialize, de::DeserializeOwned};

mod codec;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "windows")]
pub use windows::PipeConnection;

/// Maximum JSON payload size in bytes, excluding the four-byte frame header.
pub const MAX_FRAME_SIZE: usize = 64 * 1024;

/// Send one request and deserialize one response over a new local pipe connection.
///
/// `path` must have the form `\\.\pipe\NAME`, with a nonempty single-component
/// name. Remote UNC paths and alternate device namespaces are rejected. Connect
/// retries take at most two seconds and are included in `timeout`; serialization,
/// connection, every partial write/read, and deserialization share that budget.
/// Set `cancelled` to `true` to stop the operation; polling waits are at most 10 ms.
/// Kernel cancellation completion is always drained before returning (see the
/// module documentation). There is no `FlushFileBuffers` call.
///
/// # Errors
/// Returns an error for an invalid path, cancellation, timeout, unavailable pipe,
/// malformed/oversized/truncated frames, JSON conversion failure, or I/O failure.
/// On non-Windows platforms this returns an unsupported-platform error.
pub fn request<T: DeserializeOwned>(
    path: &str,
    request: &impl Serialize,
    timeout: Duration,
    cancelled: &AtomicBool,
) -> Result<T> {
    #[cfg(target_os = "windows")]
    {
        windows::request(path, request, timeout, cancelled)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (path, request, timeout, cancelled);
        anyhow::bail!("local Windows named-pipe transport is only supported on Windows")
    }
}
