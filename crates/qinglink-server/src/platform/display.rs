//! 显示器枚举与显示模式设置。

use super::platform_impl;

/// 枚举系统中的显示器及其支持的分辨率模式。
///
/// # Errors
///
/// 当检测到已连接到桌面的显示器却读不到任何当前显示模式，或完全没有检测到
/// 显示器时返回 `Err`；非 Windows 平台恒返回不支持错误。
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

/// 为指定显示器设置分辨率与刷新率。
///
/// # Errors
///
/// 当无法读取当前显示模式、目标模式不在常用分辨率候选列表中，或 Windows 拒绝
/// 该显示模式时返回 `Err`；非 Windows 平台恒返回不支持错误。
pub fn set_display_mode(
    display_id: &str,
    width: u32,
    height: u32,
    refresh_millihz: u32,
) -> anyhow::Result<()> {
    platform_impl::set_display_mode(display_id, width, height, refresh_millihz)
}
