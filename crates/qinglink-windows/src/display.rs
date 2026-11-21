//! 显示器枚举与显示模式设置的门面。
//!
//! Session 0 的 Windows 服务没有交互桌面：`EnumDisplayDevicesW` 在这里枚举不到任何
//! “连接到桌面”的显示器，`ChangeDisplaySettingsExW` 也不会作用到用户桌面。因此与
//! 窗口、键鼠输入一致——Session 0 下把请求转发给活动会话里的会话代理，由代理在
//! 用户桌面执行；代理进程自身直接调本地实现。

use qinglink_protocol::DisplayInfoResponse;

use crate::{Result, WindowsCapabilityError};

/// 校验当前平台是否支持显示控制能力。
///
/// # Errors
/// 当编译目标不是 Windows 时返回
/// [`WindowsCapabilityError::Unsupported`]（能力名为 `display control`）。
pub fn ensure_supported() -> Result<()> {
    super::is_supported()
        .then_some(())
        .ok_or(WindowsCapabilityError::Unsupported("display control"))
}

/// 枚举显示器、它的当前模式与手机端可选（1080p 及以上）的模式。
///
/// # Errors
/// Session 0 下会话代理不可用或返回的不是合法显示器列表（带上下文
/// `invalid display list from session agent`）；其余情况透传本地枚举的错误。
#[cfg(target_os = "windows")]
pub fn get_displays() -> anyhow::Result<Vec<DisplayInfoResponse>> {
    if crate::secure_desktop::running_in_session_zero() {
        let payload = crate::secure_desktop::agent_display_list()?;
        return serde_json::from_str(&payload)
            .map_err(|error| anyhow::anyhow!("invalid display list from session agent: {error}"));
    }
    crate::platform_windows::get_displays()
}

/// 非 Windows 平台没有显示器控制能力。
///
/// # Errors
/// 总是返回 “display control is only supported on Windows”。
#[cfg(not(target_os = "windows"))]
pub fn get_displays() -> anyhow::Result<Vec<DisplayInfoResponse>> {
    anyhow::bail!("display control is only supported on Windows")
}

/// 为指定显示器设置分辨率与刷新率。
///
/// # Errors
/// Session 0 下会话代理不可用或被拒绝时返回代理的错误；其余情况透传本地切换的错误
/// （模式不受支持、驱动拒绝等）。
#[cfg(target_os = "windows")]
pub fn set_display_mode(
    display_id: &str,
    width: u32,
    height: u32,
    refresh_millihz: u32,
) -> anyhow::Result<()> {
    if crate::secure_desktop::running_in_session_zero() {
        return crate::secure_desktop::agent_set_display_mode(
            display_id,
            width,
            height,
            refresh_millihz,
        );
    }
    crate::platform_windows::set_display_mode(display_id, width, height, refresh_millihz)
}

/// 非 Windows 平台没有显示器控制能力。
///
/// # Errors
/// 总是返回 “display control is only supported on Windows”。
#[cfg(not(target_os = "windows"))]
pub fn set_display_mode(_: &str, _: u32, _: u32, _: u32) -> anyhow::Result<()> {
    anyhow::bail!("display control is only supported on Windows")
}
