//! 本地命名管道客户端的连接建立与有界重试。

use std::os::windows::io::{FromRawHandle, OwnedHandle};
use std::sync::atomic::AtomicBool;
use std::time::Instant;

use anyhow::{Context, Result};
use windows::Win32::Foundation::{
    ERROR_FILE_NOT_FOUND, ERROR_PIPE_BUSY, GENERIC_READ, GENERIC_WRITE,
};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_FLAG_OVERLAPPED, FILE_SHARE_NONE, OPEN_EXISTING, SECURITY_IDENTIFICATION,
    SECURITY_SQOS_PRESENT,
};
use windows::core::PCWSTR;

use super::connection::PipeConnection;
use super::win32::{CONNECT_LIMIT, check_budget, create_event, wait_event};

/// 在 `deadline`（并额外受 `CONNECT_LIMIT` 约束）内打开一条本地管道连接。
pub(super) fn connect(
    path: &[u16],
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Result<PipeConnection> {
    let connect_deadline = deadline.min(Instant::now() + CONNECT_LIMIT);
    // A private unsignalled event provides bounded retry waits even when the
    // pipe does not exist. WaitNamedPipe's default timeout is never used.
    let retry_event = create_event()?;
    loop {
        check_budget(connect_deadline, cancelled)
            .context("pipe connection deadline/cancellation")?;
        // SAFETY: path is NUL-terminated and live for the call. No security
        // attributes means this newly owned handle is not inheritable. Only the
        // local namespace is accepted; no remote redirector can block an open.
        // Identification SQOS prevents a pipe server impersonating this client
        // with its full token, without logging any authentication material.
        let opened = unsafe {
            CreateFileW(
                PCWSTR(path.as_ptr()),
                GENERIC_READ.0 | GENERIC_WRITE.0,
                FILE_SHARE_NONE,
                None,
                OPEN_EXISTING,
                FILE_FLAG_OVERLAPPED | SECURITY_SQOS_PRESENT | SECURITY_IDENTIFICATION,
                None,
            )
        };
        match opened {
            Ok(handle) => {
                // SAFETY: CreateFileW returned one valid, newly owned handle;
                // no other OwnedHandle or File is constructed from this value.
                let owned = unsafe { OwnedHandle::from_raw_handle(handle.0) };
                check_budget(connect_deadline, cancelled)
                    .context("pipe connection exceeded its budget")?;
                // SAFETY: a newly opened client pipe is connected, byte-read
                // and wait mode by default; we requested overlapped I/O and
                // have neither issued I/O nor exposed aliases of the handle.
                return Ok(unsafe { PipeConnection::from_owned_handle(owned) });
            }
            Err(error)
                if error.code() == ERROR_PIPE_BUSY.to_hresult()
                    || error.code() == ERROR_FILE_NOT_FOUND.to_hresult() =>
            {
                wait_event(&retry_event, connect_deadline, cancelled)
                    .context("cannot wait to retry local pipe connection")?;
            }
            Err(error) => return Err(error).context("CreateFileW failed for local pipe"),
        }
    }
}
