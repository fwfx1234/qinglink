#[cfg(target_os = "windows")]
use windows::Win32::Foundation::{CloseHandle, WAIT_OBJECT_0};
use windows::Win32::System::Threading::{GetExitCodeProcess, INFINITE, WaitForSingleObject};
use windows::Win32::UI::Shell::{
    SEE_MASK_NO_CONSOLE, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW, ShellExecuteExW,
};
use windows::Win32::UI::WindowsAndMessaging::SW_HIDE;
use windows::core::PCWSTR;

/// 通过 UAC 启动提权进程时可能出现的错误。
#[derive(Debug, thiserror::Error)]
pub enum ElevationError {
    /// 调用 `ShellExecuteExW` 启动提权进程失败。
    #[error("启动提权进程失败: {0}")]
    LaunchFailed(String),
    /// 用户在 UAC 弹窗中拒绝了授权。
    #[error("用户取消了 UAC 授权")]
    Cancelled,
    /// 提权进程已启动但以非零退出码结束。
    #[error("进程执行失败，退出码: {0}")]
    ExecutionFailed(u32),
}

/// 提权模块的统一返回类型。
pub type Result<T> = std::result::Result<T, ElevationError>;

fn to_wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain([0]).collect()
}

/// 以管理员权限（触发 UAC）运行指定可执行文件并等待其结束。
///
/// 提权进程以隐藏窗口启动：debug 构建的助手是 console 子系统进程，用
/// `SW_SHOWNORMAL` 会在安装期间留下一个黑窗口；助手只通过 `--result` 文件回报结论，
/// 不需要任何可见窗口。UAC 同意框不受此影响。
///
/// # Errors
/// 以下情况返回 [`ElevationError`]：
/// - `ShellExecuteExW` 调用失败或返回无效进程句柄时返回
///   [`ElevationError::LaunchFailed`]；
/// - 等待进程结束时等待结果不是 `WAIT_OBJECT_0`，返回
///   [`ElevationError::ExecutionFailed`]（携带等待状态码）；
/// - 子进程退出码为 `1223`（用户取消 UAC 授权）时返回
///   [`ElevationError::Cancelled`]；
/// - 子进程以其它非零退出码结束时返回 [`ElevationError::ExecutionFailed`]
///   （携带退出码）。
pub fn run_elevated(executable: &str, args: &str) -> Result<()> {
    let verb = to_wide("runas");
    let file = to_wide(executable);
    let params = to_wide(args);

    let mut sei = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        lpVerb: PCWSTR(verb.as_ptr()),
        lpFile: PCWSTR(file.as_ptr()),
        lpParameters: PCWSTR(params.as_ptr()),
        nShow: SW_HIDE.0,
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

/// 非 Windows 平台的占位实现，始终返回“仅支持 Windows”错误。
///
/// # Errors
/// 该实现总是返回 [`ElevationError::LaunchFailed`]（消息为“仅支持 Windows”）。
#[cfg(not(target_os = "windows"))]
pub fn run_elevated(_executable: &str, _args: &str) -> Result<()> {
    Err(ElevationError::LaunchFailed("仅支持 Windows".into()))
}
