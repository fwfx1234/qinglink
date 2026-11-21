// 按职责分组的 Win32 能力实现。
include!("platform_windows_shared.rs");
include!("platform_windows_foreground_audio.rs");
include!("platform_windows_file_dialog.rs");

// 窗口目录、窗口状态、窗口动作与切换、已安装应用搜索，以及 COM 公寓初始化均已按职责拆成
// 真实子模块；电源与进程动作、会话与锁屏查询、注册表读取、输入服务管道、自动解锁，显示
// 枚举与模式设置、鼠标与键盘输入，以及启动与图标能力同样如此。`#[path]` 沿用
// platform_windows_*.rs 的平铺命名（子模块若不加 `#[path]` 会落到 platform_windows/
// 目录下），下面的 `pub use` 保持 `qinglink_windows::platform_windows::<符号>`
// 这一原有公开路径不变。
#[path = "platform_windows_app_catalog.rs"]
mod app_catalog;
#[path = "platform_windows_auto_unlock.rs"]
mod auto_unlock;
#[path = "platform_windows_com_apartment.rs"]
mod com_apartment;
#[path = "platform_windows_display_modes.rs"]
mod display_modes;
#[path = "platform_windows_focus_hook.rs"]
mod focus_hook;
#[path = "platform_windows_focus_text.rs"]
mod focus_text;
#[path = "platform_windows_icon_bitmap.rs"]
mod icon_bitmap;
#[path = "platform_windows_icon_candidates.rs"]
mod icon_candidates;
#[path = "platform_windows_icon_image_file.rs"]
mod icon_image_file;
#[path = "platform_windows_icon_normalize.rs"]
mod icon_normalize;
#[path = "platform_windows_icon_resolve.rs"]
mod icon_resolve;
#[path = "platform_windows_icon_resource.rs"]
mod icon_resource;
#[path = "platform_windows_icon_shell.rs"]
mod icon_shell;
#[path = "platform_windows_icon_shortcut.rs"]
mod icon_shortcut;
#[path = "platform_windows_input_routing.rs"]
mod input_routing;
#[path = "platform_windows_key_state.rs"]
mod key_state;
#[path = "platform_windows_keyboard_input.rs"]
mod keyboard_input;
#[path = "platform_windows_launch_args.rs"]
mod launch_args;
#[path = "platform_windows_launch_session.rs"]
mod launch_session;
#[path = "platform_windows_launch_shell.rs"]
mod launch_shell;
#[path = "platform_windows_lock_input_pipe.rs"]
mod lock_input_pipe;
#[path = "platform_windows_mouse_input.rs"]
mod mouse_input;
#[path = "platform_windows_power_actions.rs"]
mod power_actions;
#[path = "platform_windows_process_actions.rs"]
mod process_actions;
#[path = "platform_windows_registry.rs"]
mod registry;
#[path = "platform_windows_session_account.rs"]
mod session_account;
#[path = "platform_windows_session_lock.rs"]
mod session_lock;
#[path = "platform_windows_window_actions.rs"]
mod window_actions;
#[path = "platform_windows_window_catalog.rs"]
mod window_catalog;
#[path = "platform_windows_window_state.rs"]
mod window_state;
#[path = "platform_windows_window_switch.rs"]
mod window_switch;

pub use app_catalog::search_installed_apps;
pub use auto_unlock::auto_unlock;
pub use display_modes::{get_displays, set_display_mode};
pub use focus_hook::{FOCUS_NOTIFY_EVENT_NAME, set_focus_hook, unhook_focus};
pub use focus_text::{has_focus_input, read_focus_text};
pub use icon_candidates::list_icon_candidates;
pub use icon_image_file::{base64_image_to_icon_base64, image_file_to_icon_base64};
pub use icon_normalize::ICON_EDGE;
pub use icon_resolve::{
    ICON_SOURCE_EXECUTABLE, ICON_SOURCE_IMAGE, ICON_SOURCE_MANUAL, ICON_SOURCE_NONE,
    ICON_SOURCE_SHELL, ICON_SOURCE_SHORTCUT, IconCandidateInfo, IconResolution, resolve_icon,
    resolve_process_icon,
};
pub use icon_resource::{extract_icon_base64, extract_resource_icon_base64};
pub use icon_shell::extract_shell_icon_base64;
pub use icon_shortcut::shortcut_target_exists;
pub use key_state::pressed_buttons;
pub use keyboard_input::{key_event, key_press, send_text};
pub use launch_session::launch_in_active_session;
pub use launch_shell::{
    install_steam_game, launch_app, launch_steam_game, steam_game_launch_command, steam_installed,
};
pub use lock_input_pipe::{get_lock_input_status, submit_lock_pin};
pub use mouse_input::{
    mouse_button, mouse_button_state, mouse_move, mouse_move_absolute, mouse_wheel,
};
pub use power_actions::{
    console_ctrl_handler, install_console_ctrl_handler, lock, logoff, restart, set_keep_awake,
    shutdown, sleep,
};
pub use process_actions::{kill, resume, set_priority, suspend};
pub use session_account::configure_no_password_wake;
pub use session_lock::{get_unlock_status, is_secure_desktop};
pub use window_actions::{
    close_window, focus_process, focus_window, maximize_window, minimize_window, move_window,
    restore_window, set_always_on_top,
};
pub use window_catalog::{WindowInfo, enum_windows, enum_windows_light};
pub use window_state::is_foreground_fullscreen;
pub use window_switch::switch_window;

// 窗口与应用搜索原先同处一个 `include!` 文件，这些辅助项对同族的其他子模块以
// `super::<符号>` 可见；改造成真实子模块后在门面重新绑定，保持它们的路径不变。
use app_catalog::{read_wide_string, resolve_lnk_target};
use com_apartment::ComApartment;

/// 查询安全桌面状态（锁屏/UAC/桌面名与输入服务可用性）。
///
/// # Errors
/// 透传 `crate::secure_desktop::status()` 的错误：在非 Session 0 路径上附加输入
/// 桌面（`OpenInputDesktop`/`SetThreadDesktop`）失败时返回错误。
pub fn get_secure_desktop_status() -> anyhow::Result<crate::secure_desktop::SecureDesktopStatus> {
    crate::secure_desktop::status()
}

/// 向安全桌面注入一段 ASCII 文本（例如锁屏 PIN）。
///
/// # Errors
/// 透传 `crate::secure_desktop::send_secure_ascii` 的错误：文本未通过校验、会话代理
/// 不可用、附加输入桌面失败、字符无法在当前键盘布局下映射或 `SendInput` 注入
/// 事件数不符时返回错误。
pub fn send_secure_ascii(text: &str) -> anyhow::Result<()> {
    crate::secure_desktop::send_secure_ascii(text)
}

/// 向安全桌面发送一次确认操作（相当于按下回车）。
///
/// # Errors
/// 当前实现不返回 `Err`：`crate::secure_desktop::send_secure_attention` 忽略注册表
/// 读写与 `SendSAS` 调用的失败并总是返回 `Ok(())`。返回值保留 `Result` 以维持接口稳定。
pub fn send_secure_attention() -> anyhow::Result<()> {
    crate::secure_desktop::send_secure_attention()
}

/// 读取 PC 上焦点输入框的已有文本，供手机端回填输入框。
///
/// Session 0（服务进程）下 UIA 看不到用户桌面，必须经会话代理在登录会话里读；
/// 普通桌面进程直接读本会话。返回 `Ok(None)` 表示没有可回填的输入框——焦点不在编辑框上、
/// 密码框、只读、控件不提供 Value 模式，或取到的值是空白/占位提示。
///
/// # Errors
/// COM/UIA 初始化失败、会话代理不可用或代理拒绝请求时返回错误。
pub fn read_focus_input_text() -> anyhow::Result<Option<String>> {
    if crate::secure_desktop::running_in_session_zero() {
        let text = crate::secure_desktop::agent_request_message("focus_text", None)?;
        return Ok((!text.is_empty()).then_some(text));
    }
    focus_text::read_focus_text()
}

/// 读取物理鼠标左右中键的按下状态；Session 0 下经会话代理读登录会话。
///
/// 读不到时返回"全部抬起"：这条信息只用于界面提示与去重，不该让调用方处理错误。
pub fn mouse_buttons_state() -> qinglink_protocol::MouseButtonsState {
    if crate::secure_desktop::running_in_session_zero() {
        return crate::secure_desktop::agent_request_message("input_state", None)
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default();
    }
    key_state::pressed_buttons()
}
