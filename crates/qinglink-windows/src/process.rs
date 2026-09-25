#[cfg(target_os = "windows")]
mod imp {
    use std::mem;

    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::System::Threading::{
        OpenProcess, PROCESS_SET_INFORMATION, PROCESS_SUSPEND_RESUME, PROCESS_TERMINATE,
        SetPriorityClass, TerminateProcess,
    };

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

#[cfg(not(target_os = "windows"))]
pub fn suspend(_pid: u32) -> anyhow::Result<()> {
    anyhow::bail!("process control is only supported on Windows")
}

#[cfg(not(target_os = "windows"))]
pub fn resume(_pid: u32) -> anyhow::Result<()> {
    anyhow::bail!("process control is only supported on Windows")
}

#[cfg(not(target_os = "windows"))]
pub fn kill(_pid: u32) -> anyhow::Result<()> {
    anyhow::bail!("process control is only supported on Windows")
}

#[cfg(not(target_os = "windows"))]
pub fn set_priority(_pid: u32, _priority: &str) -> anyhow::Result<()> {
    anyhow::bail!("process control is only supported on Windows")
}
