//! `OVERLAPPED` 挂起 I/O 的状态机：内核可见存储的生命周期、取消与排空。

use std::os::windows::io::OwnedHandle;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

use anyhow::{Context, Result};
use windows::Win32::Foundation::{
    ERROR_IO_INCOMPLETE, ERROR_IO_PENDING, ERROR_NOT_FOUND, WAIT_OBJECT_0, WAIT_TIMEOUT,
};
use windows::Win32::Storage::FileSystem::{ReadFile, WriteFile};
use windows::Win32::System::IO::{CancelIoEx, GetOverlappedResult, OVERLAPPED};
use windows::Win32::System::Threading::WaitForSingleObject;
use windows::core::Error as WindowsError;

use super::win32::{WAIT_SLICE, check_budget, create_event, raw, wait_event};

// Keeping the slice borrow inside the guard makes buffer lifetime explicit: the
// guard's Drop drains pending I/O before any borrowed memory may be reused.
pub(super) enum Buffer<'a> {
    Connect,
    Read(&'a mut [u8]),
    Write(&'a [u8]),
}

pub(super) struct PendingIo<'a> {
    pipe: &'a OwnedHandle,
    buffer: Buffer<'a>,
    event: OwnedHandle,
    // Stable address even when this guard moves. Windows writes this memory
    // until GetOverlappedResult reports a terminal result.
    overlapped: Box<OVERLAPPED>,
    pending: bool,
}

impl<'a> PendingIo<'a> {
    pub(super) fn new(pipe: &'a OwnedHandle, buffer: Buffer<'a>) -> Result<Self> {
        let event = create_event()?;
        let overlapped = Box::new(OVERLAPPED {
            hEvent: raw(&event),
            ..OVERLAPPED::default()
        });
        Ok(Self {
            pipe,
            buffer,
            event,
            overlapped,
            pending: false,
        })
    }

    pub(super) fn run(mut self, deadline: Instant, cancelled: &AtomicBool) -> Result<usize> {
        check_budget(deadline, cancelled)?;
        self.pending = true;
        // SAFETY: the handle is an exclusively used overlapped pipe. Both the
        // Box<OVERLAPPED> and borrowed buffer remain valid and unchanged until
        // completion is observed, including on errors or unwinding via Drop.
        // The event is manual-reset, initially unsignalled and unique to this
        // operation. Transfer count is obtained through GetOverlappedResult.
        let started = unsafe {
            match &mut self.buffer {
                Buffer::Connect => windows::Win32::System::Pipes::ConnectNamedPipe(
                    raw(self.pipe),
                    Some(self.overlapped.as_mut() as *mut OVERLAPPED),
                ),
                Buffer::Read(buffer) => ReadFile(
                    raw(self.pipe),
                    Some(&mut **buffer),
                    None,
                    Some(self.overlapped.as_mut() as *mut OVERLAPPED),
                ),
                Buffer::Write(buffer) => WriteFile(
                    raw(self.pipe),
                    Some(*buffer),
                    None,
                    Some(self.overlapped.as_mut() as *mut OVERLAPPED),
                ),
            }
        };
        if let Err(error) = started {
            if matches!(self.buffer, Buffer::Connect)
                && error.code() == windows::Win32::Foundation::ERROR_PIPE_CONNECTED.to_hresult()
            {
                self.pending = false;
                return Ok(0);
            }
            if error.code() != ERROR_IO_PENDING.to_hresult() {
                // An immediate error other than IO_PENDING did not enqueue I/O.
                // Message-mode ERROR_MORE_DATA is excluded by the constructor.
                self.pending = false;
                return Err(error).context("cannot start overlapped pipe I/O");
            }
        }
        loop {
            if let Err(error) = check_budget(deadline, cancelled) {
                return self.abort(error);
            }
            match self.poll() {
                Ok(Some(transferred)) => {
                    check_budget(deadline, cancelled)?;
                    return Ok(transferred);
                }
                Ok(None) => {}
                Err(error) => return Err(error).context("overlapped pipe I/O failed"),
            }
            if let Err(error) = wait_event(&self.event, deadline, cancelled) {
                return self.abort(error);
            }
        }
    }

    fn poll(&mut self) -> windows::core::Result<Option<usize>> {
        let mut transferred = 0;
        // SAFETY: the pipe, event and OVERLAPPED are valid for the still-live
        // operation. bWait=false never performs an unbounded kernel wait.
        let result = unsafe {
            GetOverlappedResult(
                raw(self.pipe),
                self.overlapped.as_ref(),
                &mut transferred,
                false,
            )
        };
        match result {
            Err(error) if error.code() == ERROR_IO_INCOMPLETE.to_hresult() => Ok(None),
            result => {
                // Every other result is terminal (including OPERATION_ABORTED
                // or BROKEN_PIPE). The kernel no longer uses the I/O buffer.
                self.pending = false;
                result.map(|()| Some(transferred as usize))
            }
        }
    }

    fn abort(&mut self, cause: anyhow::Error) -> Result<usize> {
        let cancellation_error = self.cancel_and_drain();
        match cancellation_error {
            Some(error) => Err(cause.context(format!(
                "CancelIoEx failed ({error}); pipe I/O completion was still drained"
            ))),
            None => Err(cause),
        }
    }

    fn cancel_and_drain(&mut self) -> Option<WindowsError> {
        if !self.pending {
            return None;
        }
        // SAFETY: cancel only this guard's operation, not unrelated operations.
        // CancelIoEx is a request, not a completion fence: storage remains live.
        let cancellation = unsafe { CancelIoEx(raw(self.pipe), Some(self.overlapped.as_ref())) };
        let error = cancellation
            .err()
            .filter(|error| error.code() != ERROR_NOT_FOUND.to_hresult());
        // ERROR_NOT_FOUND is an expected completion race and still requires a
        // completion check. Neither a timeout nor another cancellation permits
        // freeing pending kernel-visible memory. This drain deliberately has no
        // deadline: a driver that delays cancellation can delay safe return.
        while self.pending {
            match self.poll() {
                Ok(None) => {
                    // SAFETY: our event stays owned until after completion.
                    let status = unsafe {
                        WaitForSingleObject(raw(&self.event), WAIT_SLICE.as_millis() as u32)
                    };
                    if status != WAIT_OBJECT_0 && status != WAIT_TIMEOUT {
                        // Defensive fallback for wait failures: do not release
                        // pending memory or spin while completion is uncertain.
                        std::thread::sleep(WAIT_SLICE);
                    }
                }
                Ok(Some(_)) | Err(_) => break,
            }
        }
        error
    }
}

impl Drop for PendingIo<'_> {
    fn drop(&mut self) {
        // Unwinding is no exception to the buffer lifetime rule. This also covers
        // future early returns introduced while an operation is outstanding.
        let _ = self.cancel_and_drain();
    }
}
