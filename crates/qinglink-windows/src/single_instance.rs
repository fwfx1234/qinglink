//! 会话内的桌面归属与激活，不依赖窗口创建。

#[cfg(not(target_os = "windows"))]
use anyhow::Result;

#[cfg(target_os = "windows")]
mod native {
    use anyhow::{Context, Result, ensure};
    use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
    use windows::{
        Win32::{
            Foundation::{ERROR_ALREADY_EXISTS, GetLastError, HANDLE, WAIT_OBJECT_0},
            System::Threading::{
                CreateEventW, CreateMutexW, OpenProcess, PROCESS_SYNCHRONIZE, SetEvent,
                WaitForSingleObject,
            },
        },
        core::w,
    };

    /// Holds both the instance mutex and an auto-reset activation event.
    #[derive(Debug)]
    pub struct SingleInstance {
        _mutex: OwnedHandle,
        activation: OwnedHandle,
    }

    fn activation_event() -> Result<OwnedHandle> {
        // SAFETY: The name is static, and the new handle has one owner.
        let handle =
            unsafe { CreateEventW(None, false, false, w!("Local\\QingLink.Desktop.Activate")) }
                .context("cannot create desktop activation event")?;
        // SAFETY: CreateEventW returned a valid owned handle.
        Ok(unsafe { OwnedHandle::from_raw_handle(handle.0) })
    }

    impl SingleInstance {
        /// Acquire desktop ownership. Create the event first so an early second
        /// launch cannot signal a nonexistent window or event.
        ///
        /// # Errors
        /// Returns an error when the activation event cannot be created
        /// ("cannot create desktop activation event") or when `CreateMutexW`
        /// fails ("cannot create QingLink instance mutex"). An already existing
        /// mutex is not an error: the call returns `Ok(None)`.
        pub fn acquire() -> Result<Option<Self>> {
            let activation = activation_event()?;
            // SAFETY: The static name is valid for the duration of the call.
            let raw = unsafe { CreateMutexW(None, false, w!("Local\\QingLink.Desktop.Instance")) }
                .context("cannot create QingLink instance mutex")?;
            // SAFETY: Read the thread-local error immediately after CreateMutexW.
            let exists = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;
            // SAFETY: The newly returned handle is transferred exactly once.
            let mutex = unsafe { OwnedHandle::from_raw_handle(raw.0) };
            if exists {
                return Ok(None);
            }
            Ok(Some(Self {
                _mutex: mutex,
                activation,
            }))
        }

        /// Consume one pending activation without blocking the GUI thread.
        pub fn take_activation(&self) -> bool {
            // SAFETY: The event remains owned by self; a zero timeout never waits.
            unsafe {
                WaitForSingleObject(HANDLE(self.activation.as_raw_handle()), 0) == WAIT_OBJECT_0
            }
        }
    }

    /// Record activation even when the first process has not created a window.
    ///
    /// # Errors
    /// Returns an error when the activation event cannot be created ("cannot
    /// create desktop activation event") or when `SetEvent` fails ("cannot
    /// activate existing desktop").
    pub fn activate_existing() -> Result<()> {
        let event = activation_event()?;
        // SAFETY: event owns a valid event handle throughout this call.
        unsafe { SetEvent(HANDLE(event.as_raw_handle())) }
            .context("cannot activate existing desktop")
    }

    /// Wait for a restart parent to exit. Never blocks longer than ten seconds.
    ///
    /// # Errors
    /// Returns an error when `pid` is the current process ("restart parent is
    /// the current process"), when `OpenProcess` fails for a reason other than
    /// `ERROR_INVALID_PARAMETER` ("cannot wait for restart parent"), or when the
    /// parent is still alive after ten seconds ("restart parent did not exit
    /// within ten seconds"). A PID that no longer exists (`OpenProcess` error
    /// 87) is treated as success.
    pub fn wait_for_restart_parent(pid: u32) -> Result<()> {
        ensure!(
            pid != std::process::id(),
            "restart parent is the current process"
        );
        // SAFETY: OpenProcess validates the supplied PID and grants only wait access.
        let raw = match unsafe { OpenProcess(PROCESS_SYNCHRONIZE, false, pid) } {
            Ok(handle) => handle,
            Err(error) if error.code() == windows::core::HRESULT::from_win32(87) => return Ok(()),
            Err(error) => return Err(error).context("cannot wait for restart parent"),
        };
        // SAFETY: OpenProcess returned a new owned process handle.
        let process = unsafe { OwnedHandle::from_raw_handle(raw.0) };
        // SAFETY: process stays alive until the wait completes.
        ensure!(
            unsafe { WaitForSingleObject(HANDLE(process.as_raw_handle()), 10_000) }
                == WAIT_OBJECT_0,
            "restart parent did not exit within ten seconds"
        );
        Ok(())
    }
}

#[cfg(target_os = "windows")]
pub use native::{SingleInstance, activate_existing, wait_for_restart_parent};

#[cfg(not(target_os = "windows"))]
#[derive(Debug)]
pub struct SingleInstance;

#[cfg(not(target_os = "windows"))]
impl SingleInstance {
    pub fn acquire() -> Result<Option<Self>> {
        Ok(Some(Self))
    }
    pub fn take_activation(&self) -> bool {
        false
    }
}

#[cfg(not(target_os = "windows"))]
pub fn activate_existing() -> Result<()> {
    Ok(())
}
#[cfg(not(target_os = "windows"))]
pub fn wait_for_restart_parent(_pid: u32) -> Result<()> {
    Ok(())
}
