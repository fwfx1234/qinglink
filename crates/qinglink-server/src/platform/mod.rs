//! 平台抽象层。
//!
//! 在 Windows 上，委托给 Win32 实现。
//! 在其他平台上，使用返回"不支持"错误的存根。

/// 将平台函数分派给 Windows 或存根实现。
/// 适用于无参数的函数。
macro_rules! platform_fn {
    ($name:ident, $ret:ty) => {
        #[cfg(target_os = "windows")]
        pub fn $name() -> $ret {
            windows::$name()
        }
        #[cfg(not(target_os = "windows"))]
        pub fn $name() -> $ret {
            stub::$name()
        }
    };
}

/// 将带参数的平台函数分派给 Windows 或存根实现。
macro_rules! platform_fn_args {
    ($name:ident, $ret:ty, $($arg:ident : $arg_ty:ty),*) => {
        #[cfg(target_os = "windows")]
        pub fn $name($($arg: $arg_ty),*) -> $ret {
            windows::$name($($arg),*)
        }
        #[cfg(not(target_os = "windows"))]
        pub fn $name($($arg: $arg_ty),*) -> $ret {
            stub::$name($($arg),*)
        }
    };
}

#[cfg(target_os = "windows")]
use ::windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

use crate::protocol::responses::{
    AppInfo, ForegroundResponse, ProcessInfo, UnlockStatus, VolumeStatus,
};

#[cfg(not(target_os = "windows"))]
pub use stub::WindowInfo;
#[cfg(target_os = "windows")]
pub use windows::WindowInfo;

pub struct ProcessActions;

impl ProcessActions {
    pub fn suspend(pid: u32) -> anyhow::Result<()> {
        platform_impl::suspend(pid)
    }

    pub fn resume(pid: u32) -> anyhow::Result<()> {
        platform_impl::resume(pid)
    }

    pub fn kill(pid: u32) -> anyhow::Result<()> {
        platform_impl::kill(pid)
    }
}

pub struct SystemController;

impl SystemController {
    pub fn shutdown(force: bool, delay_secs: u64) -> anyhow::Result<()> {
        platform_impl::shutdown(force, delay_secs)
    }

    pub fn sleep(hibernate: bool) -> anyhow::Result<()> {
        platform_impl::sleep(hibernate)
    }

    pub fn restart(force: bool, delay_secs: u64) -> anyhow::Result<()> {
        platform_impl::restart(force, delay_secs)
    }

    pub fn logoff() -> anyhow::Result<()> {
        platform_impl::logoff()
    }

    pub fn lock() -> anyhow::Result<()> {
        platform_impl::lock()
    }
}

#[cfg(target_os = "windows")]
pub fn install_console_ctrl_handler() -> anyhow::Result<()> {
    windows::install_console_ctrl_handler()
}

#[cfg(not(target_os = "windows"))]
pub fn install_console_ctrl_handler() -> anyhow::Result<()> {
    Ok(())
}

pub fn get_foreground_window_info() -> anyhow::Result<ForegroundResponse> {
    #[cfg(target_os = "windows")]
    if qinglink_windows::secure_desktop::running_in_session_zero() {
        let window = qinglink_windows::secure_desktop::enum_windows_light()?
            .into_iter()
            .find(|window| window.is_foreground)
            .ok_or_else(|| anyhow::anyhow!("No foreground window"))?;
        let process_name = window
            .exe_path
            .as_deref()
            .map(std::path::Path::new)
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_string();
        return Ok(ForegroundResponse {
            pid: window.pid,
            title: window.title,
            path: window.exe_path,
            process_name,
            hwnd: window.hwnd,
        });
    }
    platform_impl::get_foreground_window_info().map(|value| ForegroundResponse {
        pid: value.pid,
        title: value.title,
        path: value.path,
        process_name: value.process_name,
        hwnd: value.hwnd,
    })
}

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

pub fn launch_steam_game(app_id: u32) -> anyhow::Result<()> {
    #[cfg(target_os = "windows")]
    if qinglink_windows::secure_desktop::running_in_session_zero() {
        let (path, args) = windows::steam_game_launch_command(app_id)?;
        return launch_app(&path, &args);
    }
    platform_impl::launch_steam_game(app_id)
}

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

pub fn search_installed_apps(query: &str) -> Vec<AppInfo> {
    platform_impl::search_installed_apps(query)
        .into_iter()
        .map(|value| AppInfo {
            name: value.name,
            path: value.path,
        })
        .collect()
}

pub fn extract_icon_base64(path: &str) -> Option<String> {
    platform_impl::extract_icon_base64(path)
}

pub fn get_process_info(process: &sysinfo::Process) -> ProcessInfo {
    platform_impl::get_process_info(process)
}

pub fn set_priority(pid: u32, priority: &str) -> anyhow::Result<()> {
    platform_impl::set_priority(pid, priority)
}

// === 窗口管理 ===

#[cfg(target_os = "windows")]
pub fn enum_windows() -> Vec<WindowInfo> {
    if qinglink_windows::secure_desktop::running_in_session_zero() {
        return qinglink_windows::secure_desktop::enum_windows_light().unwrap_or_default();
    }
    windows::enum_windows()
}

#[cfg(not(target_os = "windows"))]
pub fn enum_windows() -> Vec<WindowInfo> {
    stub::enum_windows()
}

#[cfg(target_os = "windows")]
pub fn enum_windows_light() -> Vec<WindowInfo> {
    if qinglink_windows::secure_desktop::running_in_session_zero() {
        return qinglink_windows::secure_desktop::enum_windows_light().unwrap_or_default();
    }
    windows::enum_windows_light()
}

#[cfg(not(target_os = "windows"))]
pub fn enum_windows_light() -> Vec<WindowInfo> {
    stub::enum_windows_light()
}

/// 获取简化窗口列表，用于 WebSocket API。
pub fn enum_windows_list() -> Vec<crate::protocol::requests::WindowInfo> {
    #[cfg(target_os = "windows")]
    if qinglink_windows::secure_desktop::running_in_session_zero() {
        if let Ok(windows) = qinglink_windows::secure_desktop::enum_windows_light() {
            return windows
                .into_iter()
                .map(|w| crate::protocol::requests::WindowInfo {
                    id: w.hwnd,
                    title: w.title,
                    pid: w.pid,
                    exe_path: w.exe_path,
                    is_visible: w.is_visible,
                    is_foreground: w.is_foreground,
                    is_fullscreen: w.is_fullscreen,
                    is_topmost: w.is_topmost,
                })
                .collect();
        }
    }
    enum_windows_light()
        .into_iter()
        .map(|w| crate::protocol::requests::WindowInfo {
            id: w.hwnd,
            title: w.title,
            pid: w.pid,
            exe_path: w.exe_path,
            is_visible: w.is_visible,
            is_foreground: w.is_foreground,
            is_fullscreen: w.is_fullscreen,
            is_topmost: w.is_topmost,
        })
        .collect()
}

pub fn is_foreground_fullscreen() -> bool {
    platform_impl::is_foreground_fullscreen()
}

/// 获取当前前台窗口句柄。
#[cfg(target_os = "windows")]
pub fn get_foreground_window() -> Option<usize> {
    if qinglink_windows::secure_desktop::running_in_session_zero() {
        return get_foreground_window_info().ok().map(|info| info.hwnd);
    }
    let hwnd = unsafe { GetForegroundWindow() };
    (!hwnd.0.is_null()).then_some(hwnd.0 as usize)
}

#[cfg(not(target_os = "windows"))]
pub fn get_foreground_window() -> Option<usize> {
    None
}

#[cfg(target_os = "windows")]
pub fn focus_window(hwnd: usize) -> anyhow::Result<()> {
    if qinglink_windows::secure_desktop::running_in_session_zero() {
        return qinglink_windows::secure_desktop::focus_window(hwnd);
    }
    windows::focus_window(hwnd)
}

#[cfg(not(target_os = "windows"))]
pub fn focus_window(hwnd: usize) -> anyhow::Result<()> {
    stub::focus_window(hwnd)
}

#[cfg(target_os = "windows")]
pub fn minimize_window(hwnd: usize) -> anyhow::Result<()> {
    if qinglink_windows::secure_desktop::running_in_session_zero() {
        return qinglink_windows::secure_desktop::window_action("minimize_window", hwnd);
    }
    windows::minimize_window(hwnd)
}

#[cfg(not(target_os = "windows"))]
pub fn minimize_window(hwnd: usize) -> anyhow::Result<()> {
    stub::minimize_window(hwnd)
}

#[cfg(target_os = "windows")]
pub fn maximize_window(hwnd: usize) -> anyhow::Result<()> {
    if qinglink_windows::secure_desktop::running_in_session_zero() {
        return qinglink_windows::secure_desktop::window_action("maximize_window", hwnd);
    }
    windows::maximize_window(hwnd)
}

#[cfg(not(target_os = "windows"))]
pub fn maximize_window(hwnd: usize) -> anyhow::Result<()> {
    stub::maximize_window(hwnd)
}

#[cfg(target_os = "windows")]
pub fn restore_window(hwnd: usize) -> anyhow::Result<()> {
    if qinglink_windows::secure_desktop::running_in_session_zero() {
        return qinglink_windows::secure_desktop::window_action("restore_window", hwnd);
    }
    windows::restore_window(hwnd)
}

#[cfg(not(target_os = "windows"))]
pub fn restore_window(hwnd: usize) -> anyhow::Result<()> {
    stub::restore_window(hwnd)
}

#[cfg(target_os = "windows")]
pub fn close_window(hwnd: usize) -> anyhow::Result<()> {
    if qinglink_windows::secure_desktop::running_in_session_zero() {
        return qinglink_windows::secure_desktop::window_action("close_window", hwnd);
    }
    windows::close_window(hwnd)
}

#[cfg(not(target_os = "windows"))]
pub fn close_window(hwnd: usize) -> anyhow::Result<()> {
    stub::close_window(hwnd)
}

platform_fn_args!(move_window, anyhow::Result<()>, hwnd: usize, x: i32, y: i32, width: u32, height: u32);
platform_fn_args!(set_always_on_top, anyhow::Result<()>, hwnd: usize, enable: bool);

#[cfg(target_os = "windows")]
pub fn switch_window(
    direction: crate::protocol::requests::WindowSwitchDirection,
) -> anyhow::Result<()> {
    if qinglink_windows::secure_desktop::running_in_session_zero() {
        return qinglink_windows::secure_desktop::switch_window(match direction {
            crate::protocol::requests::WindowSwitchDirection::Next => {
                qinglink_protocol::WindowSwitchDirection::Next
            }
            crate::protocol::requests::WindowSwitchDirection::Previous => {
                qinglink_protocol::WindowSwitchDirection::Previous
            }
        });
    }
    windows::switch_window(match direction {
        crate::protocol::requests::WindowSwitchDirection::Next => {
            qinglink_protocol::WindowSwitchDirection::Next
        }
        crate::protocol::requests::WindowSwitchDirection::Previous => {
            qinglink_protocol::WindowSwitchDirection::Previous
        }
    })
}

#[cfg(not(target_os = "windows"))]
pub fn switch_window(
    direction: crate::protocol::requests::WindowSwitchDirection,
) -> anyhow::Result<()> {
    stub::switch_window(direction)
}

// === 窗口监控钩子 ===

#[cfg(target_os = "windows")]
pub fn set_foreground_hook(
    callback: Box<dyn Fn(isize) + Send + Sync>,
) -> anyhow::Result<::windows::Win32::UI::Accessibility::HWINEVENTHOOK> {
    windows::set_foreground_hook(callback)
}

#[cfg(not(target_os = "windows"))]
pub fn set_foreground_hook(_callback: Box<dyn Fn(isize) + Send + Sync>) -> anyhow::Result<usize> {
    anyhow::bail!("SetWinEventHook is only supported on Windows")
}

#[cfg(target_os = "windows")]
pub fn unhook_foreground(hook: ::windows::Win32::UI::Accessibility::HWINEVENTHOOK) {
    windows::unhook_foreground(hook)
}

#[cfg(not(target_os = "windows"))]
pub fn unhook_foreground(_hook: usize) {}

// === 音量 & 媒体控制 ===

pub fn get_volume() -> anyhow::Result<VolumeStatus> {
    platform_impl::get_volume().map(|value| VolumeStatus {
        percent: value.percent,
        muted: value.muted,
    })
}

pub fn get_audio_outputs() -> anyhow::Result<crate::protocol::responses::AudioOutputsResponse> {
    platform_impl::get_audio_outputs().map(|value| {
        crate::protocol::responses::AudioOutputsResponse {
            devices: value
                .devices
                .into_iter()
                .map(|device| crate::protocol::responses::AudioOutputDevice {
                    id: device.id,
                    name: device.name,
                    is_default: device.is_default,
                })
                .collect(),
        }
    })
}

pub fn set_audio_output(device_id: &str) -> anyhow::Result<()> {
    platform_impl::set_audio_output(device_id)
}

pub fn set_volume(percent: u8) -> anyhow::Result<()> {
    platform_impl::set_volume(percent)
}

pub fn set_muted(muted: bool) -> anyhow::Result<()> {
    platform_impl::set_muted(muted)
}

pub fn volume_up() -> anyhow::Result<()> {
    platform_impl::volume_up()
}

pub fn volume_down() -> anyhow::Result<()> {
    platform_impl::volume_down()
}

pub fn volume_mute() -> anyhow::Result<()> {
    platform_impl::volume_mute()
}

pub fn media_play_pause() -> anyhow::Result<()> {
    platform_impl::media_play_pause()
}

pub fn media_stop() -> anyhow::Result<()> {
    platform_impl::media_stop()
}

pub fn media_next() -> anyhow::Result<()> {
    platform_impl::media_next()
}

pub fn media_prev() -> anyhow::Result<()> {
    platform_impl::media_prev()
}

pub fn get_unlock_status() -> anyhow::Result<UnlockStatus> {
    platform_impl::get_unlock_status().map(|value| UnlockStatus {
        locked: value.locked,
        mode: value.mode,
        auto_login_enabled: value.auto_login_enabled,
        message: value.message,
        uac_detected: value.uac_detected,
        surface: value.surface,
    })
}

pub fn auto_unlock(pin: Option<&str>) -> anyhow::Result<()> {
    platform_impl::auto_unlock(pin)
}

pub fn get_lock_input_status() -> anyhow::Result<crate::protocol::responses::LockInputStatus> {
    platform_impl::get_lock_input_status().map(|value| {
        crate::protocol::responses::LockInputStatus {
            available: value.available,
            service_running: value.service_running,
            message: value.message,
        }
    })
}

pub fn submit_lock_pin(pin: &str) -> anyhow::Result<crate::protocol::responses::LockInputResult> {
    platform_impl::submit_lock_pin(pin).map(|value| crate::protocol::responses::LockInputResult {
        code: value.code,
        message: value.message,
    })
}

pub fn get_secure_desktop_status() -> anyhow::Result<crate::protocol::responses::SecureDesktopStatus>
{
    platform_impl::get_secure_desktop_status().map(|value| {
        crate::protocol::responses::SecureDesktopStatus {
            desktop: value.desktop,
            locked: value.locked,
            input_ready: value.input_ready,
            sas_supported: value.sas_supported,
            session_id: value.session_id,
            agent_state: value.agent_state,
            uac_detected: value.uac_detected,
            token_integrity_level: value.token_integrity_level,
            last_error: value.last_error,
        }
    })
}

pub fn send_secure_ascii(text: &str) -> anyhow::Result<()> {
    platform_impl::send_secure_ascii(text)
}

pub fn send_secure_attention() -> anyhow::Result<()> {
    platform_impl::send_secure_attention()
}

pub fn get_displays() -> anyhow::Result<Vec<crate::protocol::responses::DisplayInfoResponse>> {
    platform_impl::get_displays().map(|displays| {
        displays
            .into_iter()
            .map(|display| crate::protocol::responses::DisplayInfoResponse {
                id: display.id,
                name: display.name,
                primary: display.primary,
                current_mode: crate::protocol::responses::DisplayModeResponse {
                    width: display.current_mode.width,
                    height: display.current_mode.height,
                    refresh_millihz: display.current_mode.refresh_millihz,
                },
                modes: display
                    .modes
                    .into_iter()
                    .map(|mode| crate::protocol::responses::DisplayModeResponse {
                        width: mode.width,
                        height: mode.height,
                        refresh_millihz: mode.refresh_millihz,
                    })
                    .collect(),
            })
            .collect()
    })
}

pub fn set_display_mode(
    display_id: &str,
    width: u32,
    height: u32,
    refresh_millihz: u32,
) -> anyhow::Result<()> {
    platform_impl::set_display_mode(display_id, width, height, refresh_millihz)
}

pub fn mouse_move(dx: i32, dy: i32) -> anyhow::Result<()> {
    platform_impl::mouse_move(dx, dy)
}

pub fn mouse_move_absolute(x: u32, y: u32) -> anyhow::Result<()> {
    platform_impl::mouse_move_absolute(x, y)
}

pub fn mouse_button(button: &str) -> anyhow::Result<()> {
    platform_impl::mouse_button(button)
}

pub fn mouse_button_state(button: &str, pressed: bool) -> anyhow::Result<()> {
    platform_impl::mouse_button_state(button, pressed)
}

pub fn mouse_wheel(delta: i32) -> anyhow::Result<()> {
    platform_impl::mouse_wheel(delta)
}

pub fn send_text(text: &str) -> anyhow::Result<()> {
    platform_impl::send_text(text)
}

pub fn key_press(key: &str) -> anyhow::Result<()> {
    platform_impl::key_press(key)
}

pub fn key_event(usage: u8, modifiers: u8, pressed: bool) -> anyhow::Result<()> {
    platform_impl::key_event(usage, modifiers, pressed)
}

pub fn is_secure_desktop() -> anyhow::Result<bool> {
    platform_impl::is_secure_desktop()
}

pub fn configure_no_password_wake() -> anyhow::Result<()> {
    platform_impl::configure_no_password_wake()
}

// === Platform-specific implementations ===

#[cfg(target_os = "windows")]
mod platform_impl {
    pub use super::windows::*;

    use crate::protocol::responses::ProcessInfo;
    use sysinfo::Process;

    pub fn get_process_info(process: &Process) -> ProcessInfo {
        ProcessInfo {
            pid: process.pid().as_u32(),
            name: process.name().to_string_lossy().into_owned(),
            memory_bytes: process.memory(),
            cpu_usage: process.cpu_usage(),
            path: process.exe().map(|p| p.to_string_lossy().into_owned()),
            started_at: process.start_time(),
            group: String::new(),
        }
    }
}

#[cfg(not(target_os = "windows"))]
mod platform_impl {
    pub use super::stub::*;

    use crate::protocol::responses::ProcessInfo;
    use sysinfo::Process;

    pub fn get_process_info(process: &Process) -> ProcessInfo {
        ProcessInfo {
            pid: process.pid().as_u32(),
            name: process.name().to_string_lossy().into_owned(),
            memory_bytes: process.memory(),
            cpu_usage: process.cpu_usage(),
            path: process.exe().map(|p| p.to_string_lossy().into_owned()),
            started_at: process.start_time(),
            group: String::new(),
        }
    }
}

#[cfg(target_os = "windows")]
pub use qinglink_windows::platform_windows as windows;

#[cfg(not(target_os = "windows"))]
mod stub;
