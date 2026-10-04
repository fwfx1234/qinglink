//! 外壳启动：通过 `ShellExecuteW` 打开目标，并解析 Steam 安装路径与启动参数。

use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

use super::launch_args::build_args_string;

/// 以 Shell 方式启动指定可执行文件或文档，并附加命令行参数。
///
/// # Errors
/// `ShellExecuteW` 返回无效句柄或返回值不大于 32（Shell 的错误码约定）时返回
/// “ShellExecuteW failed with code …”。
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
///
/// # Errors
/// `app_id` 为 0（“Steam game has no valid app id; rescan the game library”）；
/// 注册表与默认路径都找不到带 `Steam.exe` 的安装目录
/// （“Steam 未安装或找不到 Steam.exe”）；或 Steam 路径含非 UTF-8 字符
/// （“Steam 路径包含无效字符”）时返回错误。
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
///
/// # Errors
/// 透传 [`steam_game_launch_command`] 的错误（app id 无效、找不到 Steam 安装或路径
/// 含无效字符），以及 [`launch_app`] 的 `ShellExecuteW` 失败错误。
pub fn launch_steam_game(app_id: u32) -> anyhow::Result<()> {
    // Calling Steam.exe directly is more reliable than relying on the
    // steam:// protocol association, especially when Steam is not running.
    let (path, args) = steam_game_launch_command(app_id)?;
    launch_app(&path, &args)
}

/// 请求已安装的 Steam 客户端开始其正常安装流程。
/// Steam 负责身份验证、下载位置和所有网络流量。
///
/// # Errors
/// 找不到 Steam 安装（“Steam 未安装或找不到 Steam.exe”），或经 [`launch_app`] 打开
/// `steam://install/{app_id}` 时 `ShellExecuteW` 失败（“ShellExecuteW failed with
/// code …”）返回错误。
pub fn install_steam_game(app_id: u32) -> anyhow::Result<()> {
    if !steam_installed() {
        anyhow::bail!("Steam 未安装或找不到 Steam.exe");
    }
    let uri = format!("steam://install/{app_id}");
    launch_app(&uri, &[])
}
