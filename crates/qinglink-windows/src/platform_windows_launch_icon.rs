pub fn launch_app(path: &str, args: &[String]) -> anyhow::Result<()> {
    use windows::Win32::UI::Shell::ShellExecuteW;

    let path_wide: Vec<u16> = path.encode_utf16().chain(Some(0)).collect();
    let args_joined = build_args_string(args);
    let args_wide: Vec<u16> = args_joined.encode_utf16().chain(Some(0)).collect();
    let operation: Vec<u16> = "open".encode_utf16().chain(Some(0)).collect();

    unsafe {
        let result = ShellExecuteW(
            None,
            windows::core::PCWSTR(operation.as_ptr()),
            windows::core::PCWSTR(path_wide.as_ptr()),
            windows::core::PCWSTR(if args.is_empty() {
                std::ptr::null()
            } else {
                args_wide.as_ptr()
            }),
            None,
            SW_SHOWNORMAL,
        );
        if result.is_invalid() || (result.0 as usize) <= 32 {
            anyhow::bail!("ShellExecuteW failed with code {}", result.0 as usize);
        }
    }
    Ok(())
}

/// 从注册表查找 Steam 安装路径
fn get_steam_path_from_registry() -> Option<std::path::PathBuf> {
    unsafe {
        use windows::Win32::System::Registry::{
            HKEY, HKEY_LOCAL_MACHINE, KEY_READ, RegOpenKeyExW, RegQueryValueExW,
        };
        let subkey: Vec<u16> = "SOFTWARE\\Valve\\Steam"
            .encode_utf16()
            .chain(Some(0))
            .collect();
        let mut hkey: HKEY = HKEY::default();
        let ret = RegOpenKeyExW(
            HKEY_LOCAL_MACHINE,
            windows::core::PCWSTR(subkey.as_ptr()),
            Some(0),
            KEY_READ,
            &mut hkey,
        );
        if ret.is_err() {
            return None;
        }

        let value_name: Vec<u16> = "InstallPath".encode_utf16().chain(Some(0)).collect();
        use windows::Win32::System::Registry::REG_VALUE_TYPE;
        let mut data_type: REG_VALUE_TYPE = REG_VALUE_TYPE(0);
        let mut buf: [u16; 260] = [0u16; 260];
        let mut buf_size = (buf.len() * 2) as u32;

        let ret = RegQueryValueExW(
            hkey,
            windows::core::PCWSTR(value_name.as_ptr()),
            None,
            Some(&mut data_type),
            Some(buf.as_mut_ptr() as *mut u8),
            Some(&mut buf_size),
        );
        let _ = windows::Win32::Foundation::CloseHandle(windows::Win32::Foundation::HANDLE(hkey.0));

        if ret.is_err() || buf_size == 0 {
            return None;
        }

        let len = (buf_size as usize / 2).saturating_sub(1);
        let path_str = String::from_utf16_lossy(&buf[..len]);
        let path = std::path::PathBuf::from(&path_str);
        if path.join("Steam.exe").exists() {
            Some(path)
        } else {
            None
        }
    }
}

/// 查找 Steam 安装路径（注册表优先，回退到默认路径扫描）
fn find_steam_install_path() -> Option<std::path::PathBuf> {
    if let Some(path) = get_steam_path_from_registry() {
        return Some(path);
    }

    let default_paths = [
        std::path::PathBuf::from("C:\\Program Files (x86)\\Steam"),
        std::path::PathBuf::from("C:\\Program Files\\Steam"),
        std::path::PathBuf::from("D:\\Steam"),
        std::path::PathBuf::from("D:\\Program Files\\Steam"),
        std::path::PathBuf::from("E:\\Steam"),
        std::path::PathBuf::from("E:\\Program Files\\Steam"),
    ];

    for path in &default_paths {
        if path.join("Steam.exe").exists() {
            return Some(path.clone());
        }
    }
    None
}

/// 解析启动 Steam 游戏所需的可执行文件与参数。
pub fn steam_game_launch_command(app_id: u32) -> anyhow::Result<(String, Vec<String>)> {
    if app_id == 0 {
        anyhow::bail!("Steam game has no valid app id; rescan the game library");
    }
    let steam_path = find_steam_install_path()
        .ok_or_else(|| anyhow::anyhow!("Steam 未安装或找不到 Steam.exe"))?;
    let steam_exe = steam_path.join("Steam.exe");
    let path = steam_exe
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Steam 路径包含无效字符"))?
        .to_string();
    Ok((path, vec!["-applaunch".to_string(), app_id.to_string()]))
}

/// 本机是否存在可用的 Steam 安装。
pub fn steam_installed() -> bool {
    find_steam_install_path().is_some()
}

/// 启动 Steam 游戏（通过 Steam.exe -applaunch）
pub fn launch_steam_game(app_id: u32) -> anyhow::Result<()> {
    // Calling Steam.exe directly is more reliable than relying on the
    // steam:// protocol association, especially when Steam is not running.
    let (path, args) = steam_game_launch_command(app_id)?;
    launch_app(&path, &args)
}

/// 请求已安装的 Steam 客户端开始其正常安装流程。
/// Steam 负责身份验证、下载位置和所有网络流量。
pub fn install_steam_game(app_id: u32) -> anyhow::Result<()> {
    if !steam_installed() {
        anyhow::bail!("Steam 未安装或找不到 Steam.exe");
    }
    let uri = format!("steam://install/{app_id}");
    launch_app(&uri, &[])
}

/// Launch a program as the interactive user of the active console session.
///
/// A LocalSystem service runs in Session 0; calling `ShellExecuteW` there
/// starts the process in Session 0 where the user never sees it. Services must
/// use `WTSQueryUserToken` + `CreateProcessAsUserW` so the new process runs
/// with the logged-in user's token on `winsta0\default`. Executables are started
/// directly; shortcuts are resolved to their target first and `steam://` style
/// URIs fall back to the shell's `start` command.
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
        if let Some(raw) = shortcut_arg_line.as_deref().filter(|raw| !raw.trim().is_empty()) {
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

/// 从文件提取图标并转为 PNG base64
#[allow(unsafe_op_in_unsafe_fn)]
pub fn extract_icon_base64(path: &str) -> Option<String> {
    use windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES;
    use windows::Win32::UI::Controls::IImageList;
    use windows::Win32::UI::Shell::{
        SHGFI_ICON, SHGFI_LARGEICON, SHGFI_SYSICONINDEX, SHGetFileInfoW, SHGetImageList, SHIL_JUMBO,
    };
    use windows::Win32::UI::WindowsAndMessaging::DestroyIcon;

    let wide: Vec<u16> = path.encode_utf16().chain(Some(0)).collect();

    // SHGetImageList/IImageList require COM to be initialized on this thread.
    // Custom folder scans run on worker threads, so initialize it here rather
    // than relying on the caller.
    let _com = ComApartment::init().ok();

    unsafe {
        let mut file_info = std::mem::zeroed::<windows::Win32::UI::Shell::SHFILEINFOW>();
        let result = SHGetFileInfoW(
            windows::core::PCWSTR(wide.as_ptr()),
            FILE_FLAGS_AND_ATTRIBUTES(0x80), // FILE_ATTRIBUTE_NORMAL
            Some(&mut file_info),
            std::mem::size_of::<windows::Win32::UI::Shell::SHFILEINFOW>() as u32,
            SHGFI_SYSICONINDEX,
        );

        if result == 0 {
            return None;
        }

        // Ask the shell for its jumbo image list first. This preserves the
        // executable's high-resolution icon instead of upscaling the 32px
        // `SHGFI_LARGEICON` result. Older shells may not expose the jumbo list,
        // so retain the existing large-icon path as a fallback.
        let hicon = SHGetImageList::<IImageList>(SHIL_JUMBO as i32)
            .ok()
            // ILD_TRANSPARENT (0x1) preserves the icon's alpha channel.
            .and_then(|list| list.GetIcon(file_info.iIcon, 0x1).ok())
            .or_else(|| {
                let mut fallback = std::mem::zeroed::<windows::Win32::UI::Shell::SHFILEINFOW>();
                let result = SHGetFileInfoW(
                    windows::core::PCWSTR(wide.as_ptr()),
                    FILE_FLAGS_AND_ATTRIBUTES(0x80),
                    Some(&mut fallback),
                    std::mem::size_of::<windows::Win32::UI::Shell::SHFILEINFOW>() as u32,
                    SHGFI_ICON | SHGFI_LARGEICON,
                );
                (result != 0 && !fallback.hIcon.is_invalid()).then_some(fallback.hIcon)
            })?;
        let image = icon_to_image(hicon);
        let _ = DestroyIcon(hicon);

        image
            .and_then(|image| {
                let mut buffer = Vec::new();
                let mut cursor = std::io::Cursor::new(&mut buffer);
                image.write_to(&mut cursor, image::ImageFormat::Png).ok()?;
                Some(buffer)
            })
            .map(|png_data| base64::engine::general_purpose::STANDARD.encode(&png_data))
    }
}

/// Convert an `HICON` into an RGBA image, applying the monochrome AND mask when
/// the color bitmap carries no alpha channel of its own.
#[allow(unsafe_op_in_unsafe_fn, clippy::missing_transmute_annotations)]
unsafe fn icon_to_image(
    hicon: windows::Win32::UI::WindowsAndMessaging::HICON,
) -> Option<image::RgbaImage> {
    use windows::Win32::Graphics::Gdi::{
        BI_RGB, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC,
        GetDIBits, ReleaseDC,
    };
    use windows::Win32::UI::WindowsAndMessaging::{GetIconInfo, ICONINFO};

    let mut icon_info: ICONINFO = std::mem::zeroed();
    if GetIconInfo(hicon, &mut icon_info).is_err() {
        return None;
    }

    let hdc_screen = GetDC(None);
    let hdc_mem = windows::Win32::Graphics::Gdi::CreateCompatibleDC(Some(hdc_screen));
    let _ = ReleaseDC(None, hdc_screen);

    let cleanup = |info: &ICONINFO, dc: windows::Win32::Graphics::Gdi::HDC| {
        if !info.hbmColor.is_invalid() {
            let _ = DeleteObject(std::mem::transmute(info.hbmColor));
        }
        if !info.hbmMask.is_invalid() {
            let _ = DeleteObject(std::mem::transmute(info.hbmMask));
        }
        if !dc.is_invalid() {
            let _ = DeleteDC(dc);
        }
    };

    if hdc_mem.is_invalid() {
        cleanup(&icon_info, hdc_mem);
        return None;
    }

    let mut bitmap = windows::Win32::Graphics::Gdi::BITMAP::default();
    let bitmap_size = std::mem::size_of::<windows::Win32::Graphics::Gdi::BITMAP>() as i32;
    let object_size = windows::Win32::Graphics::Gdi::GetObjectW(
        icon_info.hbmColor.into(),
        bitmap_size,
        Some(&mut bitmap as *mut _ as *mut _),
    );
    if object_size == 0 {
        cleanup(&icon_info, hdc_mem);
        return None;
    }

    let width = bitmap.bmWidth as u32;
    let height = bitmap.bmHeight as u32;
    if width == 0 || height == 0 {
        cleanup(&icon_info, hdc_mem);
        return None;
    }

    let mut color_info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: bitmap.bmWidth,
            biHeight: -bitmap.bmHeight, // top-down
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut pixels = vec![0u32; (width * height) as usize];
    // The bitmap must not be selected into a device context for GetDIBits.
    let color_lines = GetDIBits(
        hdc_mem,
        icon_info.hbmColor,
        0,
        height,
        Some(pixels.as_mut_ptr() as *mut _),
        &mut color_info as *mut _ as *mut _,
        DIB_RGB_COLORS,
    );

    // Read the 1bpp AND mask so icons without an alpha channel stay readable.
    let mask_stride = ((width as usize + 31) / 32) * 4;
    let mask = if !icon_info.hbmMask.is_invalid() {
        let mut mask_info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: bitmap.bmWidth,
                biHeight: -bitmap.bmHeight,
                biPlanes: 1,
                biBitCount: 1,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut mask_bytes = vec![0u8; mask_stride * height as usize];
        let mask_lines = GetDIBits(
            hdc_mem,
            icon_info.hbmMask,
            0,
            height,
            Some(mask_bytes.as_mut_ptr() as *mut _),
            &mut mask_info as *mut _ as *mut _,
            DIB_RGB_COLORS,
        );
        if mask_lines == height as i32 {
            Some(mask_bytes)
        } else {
            None
        }
    } else {
        None
    };

    cleanup(&icon_info, hdc_mem);

    if color_lines != height as i32 {
        return None;
    }

    let has_alpha = pixels.iter().any(|pixel| (pixel >> 24) & 0xFF != 0);
    let mut rgba = Vec::with_capacity(pixels.len() * 4);
    for (index, &pixel) in pixels.iter().enumerate() {
        let blue = (pixel & 0xFF) as u8;
        let green = ((pixel >> 8) & 0xFF) as u8;
        let red = ((pixel >> 16) & 0xFF) as u8;
        let alpha = if has_alpha {
            ((pixel >> 24) & 0xFF) as u8
        } else if let Some(mask_bytes) = mask.as_ref() {
            let row = index / width as usize;
            let column = index % width as usize;
            let byte = mask_bytes
                .get(row * mask_stride + column / 8)
                .copied()
                .unwrap_or(0xFF);
            // An AND mask bit of 1 means transparent.
            if (byte >> (7 - (column % 8))) & 1 == 0 {
                255
            } else {
                0
            }
        } else {
            255
        };
        rgba.push(red);
        rgba.push(green);
        rgba.push(blue);
        rgba.push(alpha);
    }

    let image = image::RgbaImage::from_raw(width, height, rgba)?;
    // Keep the payload small enough for the wire limit while staying crisp.
    let (current_width, current_height) = (image.width(), image.height());
    if current_width > 128 || current_height > 128 {
        let scale = 128.0 / current_width.max(current_height) as f32;
        let scaled_width = ((current_width as f32 * scale).round() as u32).max(1);
        let scaled_height = ((current_height as f32 * scale).round() as u32).max(1);
        Some(image::imageops::resize(
            &image,
            scaled_width,
            scaled_height,
            image::imageops::FilterType::Triangle,
        ))
    } else {
        Some(image)
    }
}

/// 将参数列表拼接为命令行字符串，含空格的路径自动加引号
fn build_args_string(args: &[String]) -> String {
    args.iter()
        .map(|arg| quote_windows_arg(arg))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Quote one argument using the escaping rules understood by Windows process
/// command-line parsing. In particular, backslashes immediately before a
/// quote or the closing quote must be doubled.
fn quote_windows_arg(arg: &str) -> String {
    if !arg.chars().any(|character| character.is_whitespace() || character == '"') {
        return arg.to_string();
    }
    let mut quoted = String::with_capacity(arg.len() + 2);
    quoted.push('"');
    let mut slashes = 0;
    for character in arg.chars() {
        match character {
            '\\' => slashes += 1,
            '"' => {
                quoted.extend(std::iter::repeat_n('\\', slashes * 2 + 1));
                quoted.push('"');
                slashes = 0;
            }
            other => {
                quoted.extend(std::iter::repeat_n('\\', slashes));
                quoted.push(other);
                slashes = 0;
            }
        }
    }
    quoted.extend(std::iter::repeat_n('\\', slashes * 2));
    quoted.push('"');
    quoted
}

/// 构造 `start` 命令的目标与参数，供会话外启动使用。
fn build_start_command_line(target: &str, args: &[String]) -> String {
    let mut command = format!("\"{target}\"");
    if !args.is_empty() {
        command.push(' ');
        command.push_str(&build_args_string(args));
    }
    command
}

#[cfg(test)]
mod launch_icon_tests {
    use super::{build_args_string, quote_windows_arg};

    #[test]
    fn quotes_arguments_with_spaces_and_trailing_backslashes() {
        assert_eq!(quote_windows_arg(r#"C:\\Program Files\\Game\\"#), r#""C:\\Program Files\\Game\\\\""#);
        assert_eq!(build_args_string(&["--profile".into(), "a b".into()]), "--profile \"a b\"");
    }
}
