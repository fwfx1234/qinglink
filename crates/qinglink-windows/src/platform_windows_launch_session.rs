//! 交互式会话内启动：以活动控制台会话的用户令牌调用 `CreateProcessAsUserW`。

use super::launch_args::{build_args_string, build_start_command_line};
use super::{ComApartment, resolve_lnk_target};

/// Launch a program as the interactive user of the active console session.
///
/// A LocalSystem service runs in Session 0; calling `ShellExecuteW` there
/// starts the process in Session 0 where the user never sees it. Services must
/// use `WTSQueryUserToken` + `CreateProcessAsUserW` so the new process runs
/// with the logged-in user's token on `winsta0\default`. Executables are started
/// directly; shortcuts are resolved to their target first and `steam://` style
/// URIs fall back to the shell's `start` command.
///
/// # Errors
/// Returns an error when there is no active console session ("没有活动的控制台会话"),
/// when `WTSQueryUserToken` fails ("无法获取交互式用户令牌: …"), or when the launch
/// itself fails: `CreateProcessAsUserW` or the `cmd /c start` fallback cannot start
/// the target. A failed direct launch of a resolved `.exe` is not fatal by itself;
/// it is retried through the shell `start` path.
#[cfg(target_os = "windows")]
pub fn launch_in_active_session(path: &str, args: &[String]) -> anyhow::Result<()> {
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::System::RemoteDesktop::{WTSGetActiveConsoleSessionId, WTSQueryUserToken};

    // `canonicalize` yields `\\?\` verbatim paths which `cmd.exe` and
    // `CreateProcess` do not understand. Normalize before launching.
    let path = strip_verbatim_prefix(path);
    let session = unsafe { WTSGetActiveConsoleSessionId() };
    if session == u32::MAX {
        anyhow::bail!("没有活动的控制台会话");
    }

    let mut token = HANDLE::default();
    unsafe {
        WTSQueryUserToken(session, &mut token)
            .map_err(|error| anyhow::anyhow!("无法获取交互式用户令牌: {error}"))?;
    }
    tracing::info!(session, path = %path, args = ?args, "launching program in active session");

    let result = launch_with_token(token, &path, args);
    unsafe {
        let _ = CloseHandle(token);
    }
    match &result {
        Ok(()) => tracing::info!(path = %path, "program launch handed to active session"),
        Err(error) => tracing::warn!(path = %path, %error, "program launch failed"),
    }
    result
}

#[cfg(target_os = "windows")]
fn launch_with_token(
    token: windows::Win32::Foundation::HANDLE,
    path: &str,
    args: &[String],
) -> anyhow::Result<()> {
    if is_uri_like(path) {
        return launch_via_shell_start(token, path, args);
    }

    // Resolve shortcuts so the real executable can be started directly.
    let mut target = path.to_string();
    let mut shortcut_arg_line: Option<String> = None;
    if std::path::Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("lnk"))
    {
        let _com = ComApartment::init().ok();
        let (resolved, raw_arguments) = resolve_lnk_target(std::path::Path::new(path));
        target = resolved;
        shortcut_arg_line = raw_arguments;
    }

    let target_path = std::path::Path::new(&target);
    let working_dir = target_path
        .parent()
        .filter(|_| target_path.is_file())
        .map(|parent| parent.to_string_lossy().to_string());

    if target_path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
        && target_path.is_file()
    {
        let mut command = format!("\"{target}\"");
        if let Some(raw) = shortcut_arg_line
            .as_deref()
            .filter(|raw| !raw.trim().is_empty())
        {
            command.push(' ');
            command.push_str(raw.trim());
        }
        if !args.is_empty() {
            command.push(' ');
            command.push_str(&build_args_string(args));
        }
        tracing::info!(
            shortcut = %path,
            target = %target,
            args = ?args,
            shortcut_args = ?shortcut_arg_line,
            "resolved shortcut; launching executable in active session"
        );
        match create_process_as_user(token, Some(&target), &command, working_dir.as_deref()) {
            Ok(()) => return Ok(()),
            Err(error) => {
                tracing::warn!(target = %target, %error, "direct launch failed; falling back to shell start");
            }
        }
    }

    launch_via_shell_start(token, path, args)
}

/// Start a target through `cmd /c start`, which understands `.lnk` files and
/// protocol URIs exactly like the interactive shell does.
#[cfg(target_os = "windows")]
fn launch_via_shell_start(
    token: windows::Win32::Foundation::HANDLE,
    path: &str,
    args: &[String],
) -> anyhow::Result<()> {
    let comspec =
        std::env::var("ComSpec").unwrap_or_else(|_| "C:\\Windows\\System32\\cmd.exe".to_string());
    let mut start_arguments = String::new();
    if let Some(directory) = std::path::Path::new(path)
        .parent()
        .filter(|_| std::path::Path::new(path).is_file())
        .map(|parent| parent.to_string_lossy().to_string())
    {
        start_arguments.push_str(&format!("/D \"{directory}\" "));
    }
    start_arguments.push_str(&build_start_command_line(path, args));
    let command_line = format!("\"{comspec}\" /c start \"\" {start_arguments}");
    create_process_as_user(token, None, &command_line, None)
}

#[cfg(target_os = "windows")]
fn create_process_as_user(
    token: windows::Win32::Foundation::HANDLE,
    application: Option<&str>,
    command_line: &str,
    working_dir: Option<&str>,
) -> anyhow::Result<()> {
    use windows::Win32::Foundation::{CloseHandle, WAIT_OBJECT_0};
    use windows::Win32::System::Environment::{CreateEnvironmentBlock, DestroyEnvironmentBlock};
    use windows::Win32::System::Threading::{
        CREATE_NO_WINDOW, CREATE_UNICODE_ENVIRONMENT, CreateProcessAsUserW, GetExitCodeProcess,
        PROCESS_INFORMATION, STARTF_USESHOWWINDOW, STARTUPINFOW, WaitForSingleObject,
    };
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let mut command: Vec<u16> = command_line.encode_utf16().chain([0]).collect();
    let desktop: Vec<u16> = "winsta0\\default".encode_utf16().chain([0]).collect();
    let application_wide: Option<Vec<u16>> =
        application.map(|value| value.encode_utf16().chain([0]).collect());
    let working_wide: Option<Vec<u16>> =
        working_dir.map(|value| value.encode_utf16().chain([0]).collect());
    let application_ptr = application_wide
        .as_ref()
        .map(|wide| windows::core::PCWSTR(wide.as_ptr()))
        .unwrap_or(windows::core::PCWSTR::null());
    let working_ptr = working_wide
        .as_ref()
        .map(|wide| windows::core::PCWSTR(wide.as_ptr()))
        .unwrap_or(windows::core::PCWSTR::null());
    let startup = STARTUPINFOW {
        cb: std::mem::size_of::<STARTUPINFOW>() as u32,
        lpDesktop: windows::core::PWSTR(desktop.as_ptr() as *mut u16),
        // Explicitly request a normal first show. Without this, some GUI
        // frameworks inherit a hidden/minimized startup state from the
        // service-created process even though they run in the user session.
        dwFlags: STARTF_USESHOWWINDOW,
        wShowWindow: SW_SHOWNORMAL.0 as u16,
        ..Default::default()
    };
    let mut info = PROCESS_INFORMATION::default();

    // Build the *target user's* environment block. Passing NULL makes the child
    // inherit the LocalSystem service environment, so `%LOCALAPPDATA%`,
    // `%APPDATA%`, `%TEMP%` and `%USERPROFILE%` would point at the system
    // profile. Browsers (Edge/Chrome) and Steam rely on these paths and start
    // in the wrong profile or exit immediately otherwise.
    let mut env_block: *mut core::ffi::c_void = std::ptr::null_mut();
    let env_created = unsafe {
        CreateEnvironmentBlock(&mut env_block, Some(token), false).is_ok() && !env_block.is_null()
    };
    let environment = if env_created {
        Some(env_block as *const core::ffi::c_void)
    } else {
        tracing::warn!("CreateEnvironmentBlock 失败，回退到继承的服务环境");
        None
    };
    let creation_flags = if env_created {
        CREATE_NO_WINDOW | CREATE_UNICODE_ENVIRONMENT
    } else {
        CREATE_NO_WINDOW
    };

    let result = unsafe {
        CreateProcessAsUserW(
            Some(token),
            application_ptr,
            Some(windows::core::PWSTR(command.as_mut_ptr())),
            None,
            None,
            false,
            creation_flags,
            environment,
            working_ptr,
            &startup,
            &mut info,
        )
    };
    if env_created {
        unsafe {
            let _ = DestroyEnvironmentBlock(env_block);
        }
    }
    result.map_err(|error| anyhow::anyhow!("CreateProcessAsUserW 失败: {error}"))?;

    let pid = info.dwProcessId;
    tracing::info!(
        pid,
        application = application.unwrap_or("<command line>"),
        unicode_env = env_created,
        "process created in active session"
    );
    unsafe {
        let _ = CloseHandle(info.hThread);
    }

    // A service can create the process in the user's session but cannot make
    // its window foreground from Session 0. Ask the session agent to wait for
    // the GUI window and activate it on the user's desktop.
    std::thread::spawn(move || {
        if let Err(error) = crate::secure_desktop::focus_process(pid) {
            tracing::debug!(pid, %error, "launched process has no activatable window");
        }
    });

    // The launcher may hand off to an existing instance or fail after creation.
    // Probe briefly so early exits are visible in the log instead of silently
    // reporting success to the phone.
    let process_raw = info.hProcess.0 as usize;
    std::thread::spawn(move || {
        let process = windows::Win32::Foundation::HANDLE(process_raw as *mut core::ffi::c_void);
        let wait = unsafe { WaitForSingleObject(process, 3000) };
        if wait == WAIT_OBJECT_0 {
            let mut exit_code = 0u32;
            let _ = unsafe { GetExitCodeProcess(process, &mut exit_code) };
            tracing::warn!(pid, exit_code, "launched process exited within 3s");
        }
        let _ = unsafe { CloseHandle(process) };
    });
    Ok(())
}

/// Remove the Windows verbatim (`\\?\`) prefix from a path string.
fn strip_verbatim_prefix(path: &str) -> String {
    if let Some(rest) = path.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{rest}")
    } else if let Some(rest) = path.strip_prefix(r"\\?\") {
        rest.to_string()
    } else {
        path.to_string()
    }
}

fn is_uri_like(path: &str) -> bool {
    path.contains("://")
}
