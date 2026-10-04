//! 已连接命名管道的帧读写：四字节小端长度头加 JSON 载荷。

use std::os::windows::io::OwnedHandle;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

use anyhow::{Context, Result, ensure};
use serde::{Serialize, de::DeserializeOwned};
use windows::Win32::Foundation::HANDLE;

use super::overlapped::{Buffer, PendingIo};
use super::win32::{check_budget, raw};
use crate::ipc::codec;

/// An exclusively owned, connected, overlapped byte-mode named pipe.
///
/// Operations require `&mut self`, so this wrapper never starts two I/O requests
/// concurrently. Every operation drains pending I/O before returning. A failed
/// frame operation makes the connection unusable, since partial frames cannot be
/// safely retried. Dropping the connection closes the handle exactly once.
///
/// This wrapper never flushes or disconnects the pipe. A completed write means
/// Windows accepted the bytes, not that the peer has processed them. If the server
/// needs delivery confirmation before closing, use an application acknowledgement
/// with the same deadline; do not add an unbounded `FlushFileBuffers` call.
#[derive(Debug)]
pub struct PipeConnection {
    handle: OwnedHandle,
    failed: bool,
}

impl PipeConnection {
    /// Wait for a client to connect to an exclusively owned server endpoint.
    ///
    /// # Errors
    /// Returns an error when the connect operation cannot be started, when the
    /// deadline expires or `cancelled` is already set, or when overlapped I/O
    /// fails (including cancellation on drop).
    ///
    /// # Safety
    /// The handle must be a newly created overlapped byte-mode named pipe with
    /// no outstanding I/O or other owner, as required by `from_owned_handle`.
    pub unsafe fn accept(
        handle: OwnedHandle,
        deadline: Instant,
        cancelled: &AtomicBool,
    ) -> Result<Self> {
        PendingIo::new(&handle, Buffer::Connect)?.run(deadline, cancelled)?;
        // SAFETY: connection completed; ownership and mode are the caller's contract.
        Ok(unsafe { Self::from_owned_handle(handle) })
    }

    /// Borrow the connected endpoint for peer-identity inspection only.
    pub fn raw_handle(&self) -> HANDLE {
        raw(&self.handle)
    }

    /// Take sole ownership of an already connected named-pipe endpoint.
    ///
    /// Performs no I/O and does not duplicate the handle. Authentication and
    /// local-only creation (`PIPE_REJECT_REMOTE_CLIENTS`) remain the caller's job.
    ///
    /// # Safety
    /// The handle must be a connected pipe created with `FILE_FLAG_OVERLAPPED`,
    /// `PIPE_READMODE_BYTE`, `PIPE_WAIT` and appropriate access. No I/O may be
    /// outstanding; neither the handle nor any duplicate may be closed,
    /// disconnected, reconfigured or used for I/O by another owner. Do not attach
    /// an I/O completion consumer that dereferences unrelated `OVERLAPPED` data.
    pub unsafe fn from_owned_handle(handle: OwnedHandle) -> Self {
        Self {
            handle,
            failed: false,
        }
    }

    /// Read a complete bounded frame before `deadline` or cancellation.
    ///
    /// The four-byte header and all partial body reads share `deadline`. Returned
    /// bytes have not yet been validated as JSON. An empty payload is invalid.
    ///
    /// # Errors
    /// Returns an error for cancellation, timeout, invalid length, premature EOF,
    /// I/O failure, or a previously failed connection. A failure invalidates this
    /// connection; dispose of it instead of retrying a partially consumed frame.
    pub fn read_frame(&mut self, deadline: Instant, cancelled: &AtomicBool) -> Result<Vec<u8>> {
        self.check_usable()?;
        let result = (|| {
            let mut header = [0; 4];
            self.read_exact(&mut header, deadline, cancelled)
                .context("cannot read pipe frame length header")?;
            let length = codec::decode_length(&header).context("invalid pipe frame length")?;
            let mut payload = vec![0; length];
            self.read_exact(&mut payload, deadline, cancelled)
                .context("cannot read pipe frame payload")?;
            check_budget(deadline, cancelled)?;
            Ok(payload)
        })();
        self.failed = result.is_err();
        result
    }

    /// Write a complete bounded frame before `deadline` or cancellation.
    ///
    /// All header/body writes share `deadline`; this does not flush the pipe or
    /// wait for a peer acknowledgement. The caller supplies serialized bytes.
    ///
    /// # Errors
    /// Returns an error for an empty/oversized payload, cancellation, timeout,
    /// I/O failure, or a previously failed connection. Any failure invalidates
    /// this connection, preventing retries after partially transmitted frames.
    pub fn write_frame(
        &mut self,
        payload: &[u8],
        deadline: Instant,
        cancelled: &AtomicBool,
    ) -> Result<()> {
        self.check_usable()?;
        let result = (|| {
            let header = codec::encode_length(payload.len())?;
            self.write_all(&header, deadline, cancelled)
                .context("cannot write pipe frame length header")?;
            self.write_all(payload, deadline, cancelled)
                .context("cannot write pipe frame payload")?;
            check_budget(deadline, cancelled)
        })();
        self.failed = result.is_err();
        result
    }

    /// Read one frame and deserialize its JSON without exposing payloads in errors.
    ///
    /// # Errors
    /// Returns frame I/O errors or a JSON decoding error. All failures invalidate
    /// the connection. Deserialization runs synchronously and is checked against
    /// the same deadline immediately afterward.
    pub fn read_json<T: DeserializeOwned>(
        &mut self,
        deadline: Instant,
        cancelled: &AtomicBool,
    ) -> Result<T> {
        let result = (|| {
            let payload = self.read_frame(deadline, cancelled)?;
            let value = codec::deserialize(&payload)?;
            check_budget(deadline, cancelled)?;
            Ok(value)
        })();
        self.failed = result.is_err();
        result
    }

    /// Serialize and write one JSON frame under the supplied absolute deadline.
    ///
    /// # Errors
    /// Returns size, serialization, deadline, cancellation or frame I/O errors.
    /// All failures invalidate the connection. Serialization is allocation-bounded
    /// to the frame limit but cannot preempt a custom `Serialize` implementation.
    pub fn write_json(
        &mut self,
        value: &(impl Serialize + ?Sized),
        deadline: Instant,
        cancelled: &AtomicBool,
    ) -> Result<()> {
        let result = (|| {
            self.check_usable()?;
            check_budget(deadline, cancelled)?;
            let payload = codec::serialize(value)?;
            self.write_frame(&payload, deadline, cancelled)
        })();
        self.failed = result.is_err();
        result
    }

    fn check_usable(&self) -> Result<()> {
        ensure!(
            !self.failed,
            "pipe connection is unusable after a previous frame error"
        );
        Ok(())
    }

    fn read_exact(
        &self,
        mut buffer: &mut [u8],
        deadline: Instant,
        cancelled: &AtomicBool,
    ) -> Result<()> {
        while !buffer.is_empty() {
            let length = buffer.len();
            let transferred =
                PendingIo::new(&self.handle, Buffer::Read(buffer))?.run(deadline, cancelled)?;
            ensure!(
                transferred != 0,
                "truncated pipe frame: peer closed before frame completion"
            );
            ensure!(
                transferred <= length,
                "pipe read returned an invalid transfer length"
            );
            buffer = &mut buffer[transferred..];
        }
        Ok(())
    }

    fn write_all(
        &self,
        mut buffer: &[u8],
        deadline: Instant,
        cancelled: &AtomicBool,
    ) -> Result<()> {
        while !buffer.is_empty() {
            let transferred =
                PendingIo::new(&self.handle, Buffer::Write(buffer))?.run(deadline, cancelled)?;
            ensure!(transferred != 0, "pipe write made no progress");
            ensure!(
                transferred <= buffer.len(),
                "pipe write returned an invalid transfer length"
            );
            buffer = &buffer[transferred..];
        }
        Ok(())
    }
}
