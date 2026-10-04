#[cfg(target_os = "windows")]
mod imp {
    use std::mem;

    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::System::Threading::{
        OpenProcess, PROCESS_SET_INFORMATION, PROCESS_SUSPEND_RESUME, PROCESS_TERMINATE,
        SetPriorityClass, TerminateProcess,
    };

    /// 挂起指定进程的全部线程（调用 ntdll 的 `NtSuspendProcess`）。
    ///
    /// # Errors
    /// `OpenProcess` 失败或返回无效句柄（“Failed to open process {pid}”）、
    /// `CloseHandle` 失败、或 `NtSuspendProcess` 调用失败（`GetModuleHandleW`/
    /// `GetProcAddress` 失败，或返回的 NTSTATUS 为负值）时返回错误。
    pub fn suspend(pid: u32) -> anyhow::Result<()> {
        unsafe {
            let process = OpenProcess(PROCESS_SUSPEND_RESUME, false, pid)?;
            if process.is_invalid() {
                anyhow::bail!("Failed to open process {pid}");
            }
            let result = call_nt_process(process, "NtSuspendProcess");
            CloseHandle(process)?;
            result
        }
    }

    /// 恢复指定进程中先前被挂起的线程（调用 ntdll 的 `NtResumeProcess`）。
    ///
    /// # Errors
    /// `OpenProcess` 失败或返回无效句柄（“Failed to open process {pid}”）、
    /// `CloseHandle` 失败、或 `NtResumeProcess` 调用失败（`GetModuleHandleW`/
    /// `GetProcAddress` 失败，或返回的 NTSTATUS 为负值）时返回错误。
    pub fn resume(pid: u32) -> anyhow::Result<()> {
        unsafe {
            let process = OpenProcess(PROCESS_SUSPEND_RESUME, false, pid)?;
            if process.is_invalid() {
                anyhow::bail!("Failed to open process {pid}");
            }
            let result = call_nt_process(process, "NtResumeProcess");
            CloseHandle(process)?;
            result
        }
    }

    /// 强制结束指定进程。
    ///
    /// # Errors
    /// `OpenProcess` 失败或返回无效句柄（“Failed to open process {pid}”）、
    /// `TerminateProcess` 失败或 `CloseHandle` 失败时返回错误。
    pub fn kill(pid: u32) -> anyhow::Result<()> {
        unsafe {
            let process = OpenProcess(PROCESS_TERMINATE, false, pid)?;
            if process.is_invalid() {
                anyhow::bail!("Failed to open process {pid}");
            }
            let result = TerminateProcess(process, 1);
            CloseHandle(process)?;
            result?;
        }
        Ok(())
    }

    /// 设置指定进程的优先级类别，支持 idle、below_normal/low、normal、
    /// above_normal/high、realtime，无法识别的取值按 normal 处理。
    ///
    /// # Errors
    /// `OpenProcess` 失败或返回无效句柄（“Failed to open process {pid}”）、
    /// `SetPriorityClass` 失败或 `CloseHandle` 失败时返回错误；无法识别的
    /// `priority` 不返回错误，而是按 normal 处理。
    pub fn set_priority(pid: u32, priority: &str) -> anyhow::Result<()> {
        unsafe {
            let process = OpenProcess(PROCESS_SET_INFORMATION, false, pid)?;
            if process.is_invalid() {
                anyhow::bail!("Failed to open process {pid}");
            }

            let priority_class = match priority.to_lowercase().as_str() {
                "idle" => windows::Win32::System::Threading::IDLE_PRIORITY_CLASS,
                "below_normal" | "low" => {
                    windows::Win32::System::Threading::BELOW_NORMAL_PRIORITY_CLASS
                }
                "normal" => windows::Win32::System::Threading::NORMAL_PRIORITY_CLASS,
                "above_normal" | "high" => {
                    windows::Win32::System::Threading::ABOVE_NORMAL_PRIORITY_CLASS
                }
                "realtime" => windows::Win32::System::Threading::REALTIME_PRIORITY_CLASS,
                _ => windows::Win32::System::Threading::NORMAL_PRIORITY_CLASS,
            };

            let result = SetPriorityClass(process, priority_class);
            CloseHandle(process)?;
            result?;
        }
        Ok(())
    }

    unsafe fn call_nt_process(process: HANDLE, procedure: &str) -> anyhow::Result<()> {
        let ntdll = unsafe {
            windows::Win32::System::LibraryLoader::GetModuleHandleW(windows::core::w!("ntdll.dll"))?
        };
        let procedure = match procedure {
            "NtSuspendProcess" => windows::core::s!("NtSuspendProcess"),
            "NtResumeProcess" => windows::core::s!("NtResumeProcess"),
            _ => anyhow::bail!("unsupported ntdll procedure"),
        };
        let proc_addr =
            unsafe { windows::Win32::System::LibraryLoader::GetProcAddress(ntdll, procedure) };
        let Some(proc_addr) = proc_addr else {
            anyhow::bail!("ntdll procedure not found");
        };
        let function: unsafe extern "system" fn(HANDLE) -> i32 =
            unsafe { mem::transmute(proc_addr) };
        let status = unsafe { function(process) };
        if status < 0 {
            anyhow::bail!("ntdll process operation failed with status 0x{status:08X}");
        }
        Ok(())
    }
}

#[cfg(target_os = "windows")]
pub use imp::*;

/// 非 Windows 平台的占位实现，始终返回不支持错误。
#[cfg(not(target_os = "windows"))]
pub fn suspend(_pid: u32) -> anyhow::Result<()> {
    anyhow::bail!("process control is only supported on Windows")
}

/// 非 Windows 平台的占位实现，始终返回不支持错误。
#[cfg(not(target_os = "windows"))]
pub fn resume(_pid: u32) -> anyhow::Result<()> {
    anyhow::bail!("process control is only supported on Windows")
}

/// 非 Windows 平台的占位实现，始终返回不支持错误。
#[cfg(not(target_os = "windows"))]
pub fn kill(_pid: u32) -> anyhow::Result<()> {
    anyhow::bail!("process control is only supported on Windows")
}

/// 非 Windows 平台的占位实现，始终返回不支持错误。
#[cfg(not(target_os = "windows"))]
pub fn set_priority(_pid: u32, _priority: &str) -> anyhow::Result<()> {
    anyhow::bail!("process control is only supported on Windows")
}
