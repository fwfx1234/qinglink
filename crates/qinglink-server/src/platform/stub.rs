//! Stub implementations for non-Windows platforms.
//!
//! Each function returns an error indicating the operation is Windows-only.

use crate::protocol::responses::{AppInfo, ForegroundResponse};

/// Generate a stub function that bail! with "only supported on Windows".
macro_rules! windows_only_bail {
    ($name:ident, $ret:ty, $($arg:ident : $arg_ty:ty),*) => {
        pub fn $name($(_: $arg_ty),*) -> $ret {
            anyhow::bail!(concat!(stringify!($name), " is only supported on Windows"))
        }
    };
}

/// Generate a stub function returning Vec::new().
macro_rules! stub_empty_vec {
    ($name:ident, $item:ty) => {
        pub fn $name() -> Vec<$item> {
            Vec::new()
        }
    };
}

// === Process control ===

windows_only_bail!(suspend, anyhow::Result<()>, _pid: u32);
windows_only_bail!(resume, anyhow::Result<()>, _pid: u32);
windows_only_bail!(kill, anyhow::Result<()>, _pid: u32);

// === System power ===

windows_only_bail!(shutdown, anyhow::Result<()>, _force: bool, _delay_secs: u64);
windows_only_bail!(sleep, anyhow::Result<()>, _hibernate: bool);
windows_only_bail!(restart, anyhow::Result<()>, _force: bool, _delay_secs: u64);
windows_only_bail!(logoff, anyhow::Result<()>);
windows_only_bail!(lock, anyhow::Result<()>);
windows_only_bail!(auto_unlock, anyhow::Result<()>, _pin: Option<&str>);

// === Application & window queries ===

windows_only_bail!(
    get_foreground_window_info,
    anyhow::Result<ForegroundResponse>
);
windows_only_bail!(launch_app, anyhow::Result<()>, _path: &str, _args: &[String]);
windows_only_bail!(launch_steam_game, anyhow::Result<()>, _app_id: u32);
windows_only_bail!(install_steam_game, anyhow::Result<()>, _app_id: u32);
windows_only_bail!(set_priority, anyhow::Result<()>, _pid: u32, _priority: &str);
windows_only_bail!(
    get_unlock_status,
    anyhow::Result<crate::protocol::responses::UnlockStatus>
);
windows_only_bail!(
    get_lock_input_status,
    anyhow::Result<crate::protocol::responses::LockInputStatus>
);
windows_only_bail!(submit_lock_pin, anyhow::Result<crate::protocol::responses::LockInputResult>, _pin: &str);

pub fn search_installed_apps(_query: &str) -> Vec<AppInfo> {
    Vec::new()
}

// === Window management ===

#[derive(Debug, Clone)]
pub struct WindowInfo {
    pub hwnd: usize,
    pub title: String,
    pub pid: u32,
    pub exe_path: Option<String>,
    pub is_visible: bool,
    pub is_foreground: bool,
    pub is_fullscreen: bool,
    pub is_topmost: bool,
}

stub_empty_vec!(enum_windows, WindowInfo);
stub_empty_vec!(enum_windows_light, WindowInfo);

pub fn is_foreground_fullscreen() -> bool {
    false
}

windows_only_bail!(focus_window, anyhow::Result<()>, _hwnd: usize);
windows_only_bail!(minimize_window, anyhow::Result<()>, _hwnd: usize);
windows_only_bail!(maximize_window, anyhow::Result<()>, _hwnd: usize);
windows_only_bail!(restore_window, anyhow::Result<()>, _hwnd: usize);
windows_only_bail!(close_window, anyhow::Result<()>, _hwnd: usize);
windows_only_bail!(switch_window, anyhow::Result<()>, _direction: crate::protocol::requests::WindowSwitchDirection);
windows_only_bail!(move_window, anyhow::Result<()>, _hwnd: usize, _x: i32, _y: i32, _width: u32, _height: u32);
windows_only_bail!(set_always_on_top, anyhow::Result<()>, _hwnd: usize, _enable: bool);

// === Volume & media control ===

windows_only_bail!(
    get_volume,
    anyhow::Result<crate::protocol::responses::VolumeStatus>
);
windows_only_bail!(
    get_audio_outputs,
    anyhow::Result<crate::protocol::responses::AudioOutputsResponse>
);
windows_only_bail!(set_audio_output, anyhow::Result<()>, _device_id: &str);
windows_only_bail!(set_volume, anyhow::Result<()>, _percent: u8);
windows_only_bail!(set_muted, anyhow::Result<()>, _muted: bool);
windows_only_bail!(volume_up, anyhow::Result<()>);
windows_only_bail!(volume_down, anyhow::Result<()>);
windows_only_bail!(volume_mute, anyhow::Result<()>);
windows_only_bail!(media_play_pause, anyhow::Result<()>);
windows_only_bail!(media_stop, anyhow::Result<()>);
windows_only_bail!(media_next, anyhow::Result<()>);
windows_only_bail!(media_prev, anyhow::Result<()>);

// === Secure desktop ===

pub fn get_secure_desktop_status() -> anyhow::Result<crate::protocol::responses::SecureDesktopStatus>
{
    Ok(crate::protocol::responses::SecureDesktopStatus {
        desktop: "unknown".into(),
        locked: false,
        input_ready: false,
        sas_supported: false,
        session_id: 0,
        agent_state: "unsupported".into(),
        last_error: "secure desktop input is only supported on Windows".into(),
    })
}

windows_only_bail!(send_secure_ascii, anyhow::Result<()>, _text: &str);
windows_only_bail!(send_secure_attention, anyhow::Result<()>);

// === Display control ===

windows_only_bail!(
    get_displays,
    anyhow::Result<Vec<crate::protocol::responses::DisplayInfoResponse>>
);
windows_only_bail!(set_display_mode, anyhow::Result<()>, _display_id: &str, _width: u32, _height: u32, _refresh_millihz: u32);

// === Input control ===

windows_only_bail!(mouse_move, anyhow::Result<()>, _dx: i32, _dy: i32);
windows_only_bail!(mouse_button, anyhow::Result<()>, _button: &str);
windows_only_bail!(mouse_button_state, anyhow::Result<()>, _button: &str, _pressed: bool);
windows_only_bail!(mouse_wheel, anyhow::Result<()>, _delta: i32);
windows_only_bail!(send_text, anyhow::Result<()>, _text: &str);
windows_only_bail!(key_press, anyhow::Result<()>, _key: &str);
windows_only_bail!(key_event, anyhow::Result<()>, _usage: u8, _modifiers: u8, _pressed: bool);

pub fn is_secure_desktop() -> anyhow::Result<bool> {
    Ok(false)
}

windows_only_bail!(configure_no_password_wake, anyhow::Result<()>);
