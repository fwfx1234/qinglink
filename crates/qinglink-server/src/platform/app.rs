//! 应用启动（含 Steam 与 Session 0 转交）与图标解析。

use super::platform_impl;

use crate::protocol::responses::AppInfo;

#[cfg(target_os = "windows")]
use super::windows;

#[cfg(not(target_os = "windows"))]
pub use super::stub::{IconCandidateInfo, IconResolution};
#[cfg(target_os = "windows")]
pub use super::windows::{IconCandidateInfo, IconResolution};

/// 启动指定路径的程序，并传入命令行参数。
///
/// 服务运行在 Session 0 时会把启动请求转交给交互式会话。
///
/// # Errors
///
/// 当 `ShellExecuteW` 返回失败码（路径不存在、无打开权限等），或在 Session 0
/// 下转交交互式会话失败（没有活动控制台会话、获取交互式用户令牌失败、
/// `CreateProcessAsUserW` 失败等）时返回 `Err`；非 Windows 平台恒返回不支持错误。
pub fn launch_app(path: &str, args: &[String]) -> anyhow::Result<()> {
    // The WebSocket server runs inside the LocalSystem service (Session 0).
    // Launching there would create the process in Session 0 where the user
    // cannot see it, so hand program launches to the interactive session.
    #[cfg(target_os = "windows")]
    if qinglink_windows::secure_desktop::running_in_session_zero() {
        return windows::launch_in_active_session(path, args);
    }
    platform_impl::launch_app(path, args)
}

/// 通过 Steam 启动指定应用 ID 的游戏。
///
/// # Errors
///
/// 当 `app_id` 为 0、找不到有效的 `Steam.exe`、Steam 路径含无效字符，或最终
/// 启动进程失败时返回 `Err`；非 Windows 平台恒返回不支持错误。
pub fn launch_steam_game(app_id: u32) -> anyhow::Result<()> {
    #[cfg(target_os = "windows")]
    if qinglink_windows::secure_desktop::running_in_session_zero() {
        let (path, args) = windows::steam_game_launch_command(app_id)?;
        return launch_app(&path, &args);
    }
    platform_impl::launch_steam_game(app_id)
}

/// 通过 Steam 触发指定应用 ID 的安装。
///
/// # Errors
///
/// 当找不到 `Steam.exe`（Steam 未安装），或启动 `steam://install/<app_id>`
/// 失败时返回 `Err`；非 Windows 平台恒返回不支持错误。
pub fn install_steam_game(app_id: u32) -> anyhow::Result<()> {
    #[cfg(target_os = "windows")]
    if qinglink_windows::secure_desktop::running_in_session_zero() {
        if !windows::steam_installed() {
            anyhow::bail!("Steam 未安装或找不到 Steam.exe");
        }
        return launch_app(&format!("steam://install/{app_id}"), &[]);
    }
    platform_impl::install_steam_game(app_id)
}

/// 按名称关键字搜索已安装的应用程序。
pub fn search_installed_apps(query: &str) -> Vec<AppInfo> {
    platform_impl::search_installed_apps(query)
        .into_iter()
        .map(|value| AppInfo {
            name: value.name,
            path: value.path,
        })
        .collect()
}

/// 提取指定可执行文件的图标并编码为 Base64。
pub fn extract_icon_base64(path: &str) -> Option<String> {
    platform_impl::extract_icon_base64(path)
}

/// Resolve an entry's icon through every source Windows offers.
///
/// `explicit` is a user-chosen file (or a scanner-recorded artwork path). See
/// [`qinglink_windows::platform_windows::resolve_icon`] for the full chain.
pub fn resolve_icon(path: &str, explicit: Option<&str>) -> IconResolution {
    platform_impl::resolve_icon(path, explicit)
}

/// Icon sources a user can pick from for one entry, best guess first.
pub fn list_icon_candidates(path: &str) -> Vec<IconCandidateInfo> {
    platform_impl::list_icon_candidates(path)
}

/// Decode a standalone image file into the bounded PNG payload the catalog stores.
pub fn image_file_to_icon_base64(path: &str) -> Option<String> {
    platform_impl::image_file_to_icon_base64(path)
}

/// Whether a catalog path still resolves to something launchable.
pub fn shortcut_target_exists(path: &str) -> bool {
    platform_impl::shortcut_target_exists(path)
}
