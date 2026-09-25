// 按职责分组的 Win32 能力实现。
include!("platform_windows_shared.rs");
include!("platform_windows_power_session.rs");
include!("platform_windows_foreground_audio.rs");
include!("platform_windows_display_input.rs");
include!("platform_windows_launch_icon.rs");
include!("platform_windows_window_catalog.rs");

pub fn get_secure_desktop_status() -> anyhow::Result<crate::secure_desktop::SecureDesktopStatus> {
    crate::secure_desktop::status()
}

pub fn send_secure_ascii(text: &str) -> anyhow::Result<()> {
    crate::secure_desktop::send_secure_ascii(text)
}

pub fn send_secure_attention() -> anyhow::Result<()> {
    crate::secure_desktop::send_secure_attention()
}
