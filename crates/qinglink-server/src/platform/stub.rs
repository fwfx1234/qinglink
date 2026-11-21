//! 非 Windows 平台上的桩实现。
//!
//! 每个函数都返回一个表示"该操作仅 Windows 支持"的错误。

use crate::protocol::responses::{AppInfo, ForegroundResponse};

/// Generate a stub function that bail! with "only supported on Windows".
///
/// The zero-argument form needs its own arm: a `ty` fragment may only be
/// followed by `=>`, `,` or `;`, so the variadic arm cannot accept a call
/// without trailing arguments.
macro_rules! windows_only_bail {
    ($name:ident, $ret:ty) => {
        #[doc = concat!("`", stringify!($name), "` 的存根实现：非 Windows 平台调用时返回不支持错误。")]
        ///
        /// # Errors
        ///
        /// 非 Windows 平台恒返回不支持错误（该操作仅支持 Windows）。
        pub fn $name() -> $ret {
            anyhow::bail!(concat!(stringify!($name), " is only supported on Windows"))
        }
    };
    ($name:ident, $ret:ty, $($arg:ident : $arg_ty:ty),*) => {
        #[doc = concat!("`", stringify!($name), "` 的存根实现：非 Windows 平台调用时返回不支持错误。")]
        ///
        /// # Errors
        ///
        /// 非 Windows 平台恒返回不支持错误（该操作仅支持 Windows）。
        pub fn $name($(_: $arg_ty),*) -> $ret {
            anyhow::bail!(concat!(stringify!($name), " is only supported on Windows"))
        }
    };
}

/// Generate a stub function returning Vec::new().
macro_rules! stub_empty_vec {
    ($name:ident, $item:ty) => {
        #[doc = concat!("`", stringify!($name), "` 的存根实现：非 Windows 平台恒返回空列表。")]
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
windows_only_bail!(set_keep_awake, anyhow::Result<()>, _enabled: bool);
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

/// 非 Windows 平台没有已安装应用清单，恒返回空列表。
pub fn search_installed_apps(_query: &str) -> Vec<AppInfo> {
    Vec::new()
}

// === Icon resolution ===
//
// Non-Windows builds keep the catalog working; only the icon sources that need
// the Windows shell are unavailable.

/// One pickable icon source for a catalog entry.
#[derive(Clone, Debug)]
pub struct IconCandidateInfo {
    /// 图标来源文件的路径
    pub path: String,
    /// 图标来源类型（如 exe、lnk、图片文件）
    pub kind: String,
    /// 来源文件的字节大小
    pub size_bytes: u64,
}

/// Outcome of resolving one entry's icon.
#[derive(Clone, Debug)]
pub struct IconResolution {
    /// 解析出的图标 PNG 数据（Base64 编码）
    pub icon_base64: Option<String>,
    /// 实际命中的图标来源标识
    pub source: String,
    /// 命中来源对应的文件路径
    pub source_path: Option<String>,
}

/// 非 Windows 平台无法提取可执行文件图标，恒返回 `None`。
pub fn extract_icon_base64(_path: &str) -> Option<String> {
    None
}

/// 非 Windows 平台没有 Windows 图标来源，返回空的解析结果。
pub fn resolve_icon(_path: &str, _explicit: Option<&str>) -> IconResolution {
    IconResolution {
        icon_base64: None,
        source: "none".to_string(),
        source_path: None,
    }
}

/// 非 Windows 平台没有进程图标来源，返回空的解析结果。
pub fn resolve_process_icon(_path: &str) -> IconResolution {
    IconResolution {
        icon_base64: None,
        source: "none".to_string(),
        source_path: None,
    }
}

/// 非 Windows 平台没有可选的图标来源，恒返回空列表。
pub fn list_icon_candidates(_path: &str) -> Vec<IconCandidateInfo> {
    Vec::new()
}

/// 非 Windows 平台无法把图片文件转换为图标，恒返回 `None`。
pub fn image_file_to_icon_base64(_path: &str) -> Option<String> {
    None
}

/// 非 Windows 平台无法规范化客户端上传的图标位图，恒返回 `None`。
pub fn base64_image_to_icon_base64(_payload: &str) -> Option<String> {
    None
}

/// Non-Windows builds cannot resolve shortcuts, so nothing is filtered out.
pub fn shortcut_target_exists(path: &str) -> bool {
    std::path::Path::new(path).exists()
}

// === Window management ===

/// 非 Windows 平台下用于占位的窗口信息结构。
#[derive(Debug, Clone)]
pub struct WindowInfo {
    /// 窗口句柄
    pub hwnd: usize,
    /// 窗口标题
    pub title: String,
    /// 所属进程 ID
    pub pid: u32,
    /// 所属进程的可执行文件路径
    pub exe_path: Option<String>,
    /// 窗口是否可见
    pub is_visible: bool,
    /// 窗口是否为前台窗口
    pub is_foreground: bool,
    /// 窗口是否全屏
    pub is_fullscreen: bool,
    /// 窗口是否置顶
    pub is_topmost: bool,
}

stub_empty_vec!(enum_windows, WindowInfo);
stub_empty_vec!(enum_windows_light, WindowInfo);

/// 非 Windows 平台没有窗口系统，恒返回 `false`。
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

/// 非 Windows 平台报告安全桌面不可用。
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

/// 非 Windows 平台不存在安全桌面，恒返回 `false`。
pub fn is_secure_desktop() -> anyhow::Result<bool> {
    Ok(false)
}

/// 非 Windows 平台没有物理鼠标状态可读，恒返回"全部抬起"。
pub fn mouse_buttons_state() -> qinglink_protocol::MouseButtonsState {
    qinglink_protocol::MouseButtonsState::default()
}

windows_only_bail!(configure_no_password_wake, anyhow::Result<()>);
