#[cfg(target_os = "windows")]
use windows::Win32::Foundation::{CloseHandle, WAIT_OBJECT_0};
use windows::Win32::System::Threading::{GetExitCodeProcess, INFINITE, WaitForSingleObject};
use windows::Win32::UI::Shell::{
    SEE_MASK_NO_CONSOLE, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW, ShellExecuteExW,
};
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
use windows::core::PCWSTR;

#[derive(Debug, thiserror::Error)]
pub enum ElevationError {
    #[error("启动提权进程失败: {0}")]
    LaunchFailed(String),
    #[error("用户取消了 UAC 授权")]
    Cancelled,
    #[error("进程执行失败，退出码: {0}")]
    ExecutionFailed(u32),
}

pub type Result<T> = std::result::Result<T, ElevationError>;

fn to_wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain([0]).collect()
}

pub fn run_elevated(executable: &str, args: &str) -> Result<()> {
    let verb = to_wide("runas");
    let file = to_wide(executable);
    let params = to_wide(args);

    let mut sei = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        lpVerb: PCWSTR(verb.as_ptr()),
        lpFile: PCWSTR(file.as_ptr()),
        lpParameters: PCWSTR(params.as_ptr()),
        nShow: SW_SHOWNORMAL.0 as i32,
        fMask: SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NO_CONSOLE,
        ..Default::default()
    };

    unsafe {
        ShellExecuteExW(&mut sei)
            .map_err(|error| ElevationError::LaunchFailed(error.to_string()))?;

        let process = sei.hProcess;
        if process.is_invalid() {
            return Err(ElevationError::LaunchFailed("无效的进程句柄".into()));
        }

        let wait_result = WaitForSingleObject(process, INFINITE);
        if wait_result != WAIT_OBJECT_0 {
            let _ = CloseHandle(process);
            return Err(ElevationError::ExecutionFailed(wait_result.0));
        }

        let mut exit_code = 0u32;
        let _ = GetExitCodeProcess(process, &mut exit_code);
        let _ = CloseHandle(process);

        if exit_code == 0 {
            Ok(())
        } else if exit_code == 1223 {
            Err(ElevationError::Cancelled)
        } else {
            Err(ElevationError::ExecutionFailed(exit_code))
        }
    }
}

pub fn run_elevated_sc(args: &[&str]) -> Result<()> {
    let sc_args = args.join(" ");
    run_elevated("sc.exe", &sc_args)
}

#[cfg(not(target_os = "windows"))]
pub fn run_elevated(_executable: &str, _args: &str) -> Result<()> {
    Err(ElevationError::LaunchFailed("仅支持 Windows".into()))
}

#[cfg(not(target_os = "windows"))]
pub fn run_elevated_sc(_args: &[&str]) -> Result<()> {
    Err(ElevationError::LaunchFailed("仅支持 Windows".into()))
}
