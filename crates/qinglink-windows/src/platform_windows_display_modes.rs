//! 显示枚举与模式设置：读取已连接显示器、筛选常用分辨率候选并切换显示模式。

use std::mem;

use windows::Win32::Graphics::Gdi::{
    CDS_TYPE, ChangeDisplaySettingsExW, DEVMODEW, DISP_CHANGE, DISP_CHANGE_BADDUALVIEW,
    DISP_CHANGE_BADFLAGS, DISP_CHANGE_BADMODE, DISP_CHANGE_BADPARAM, DISP_CHANGE_FAILED,
    DISP_CHANGE_NOTUPDATED, DISP_CHANGE_RESTART, DISP_CHANGE_SUCCESSFUL,
    DISPLAY_DEVICE_ATTACHED_TO_DESKTOP, DISPLAY_DEVICE_PRIMARY_DEVICE, DISPLAY_DEVICEW,
    DM_DISPLAYFREQUENCY, DM_PELSHEIGHT, DM_PELSWIDTH, ENUM_CURRENT_SETTINGS,
    ENUM_DISPLAY_SETTINGS_FLAGS, ENUM_DISPLAY_SETTINGS_MODE, EnumDisplayDevicesW,
    EnumDisplaySettingsExW,
};

use qinglink_protocol::{DisplayInfoResponse, DisplayModeResponse};

use super::to_wide_z;

fn wide_array_string(value: &[u16]) -> String {
    let end = value
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(value.len());
    String::from_utf16_lossy(&value[..end])
}

fn display_mode(_device_name: &str, mode: &DEVMODEW) -> DisplayModeResponse {
    DisplayModeResponse {
        width: mode.dmPelsWidth,
        height: mode.dmPelsHeight,
        refresh_millihz: mode.dmDisplayFrequency.saturating_mul(1000),
    }
}

const COMMON_DISPLAY_RESOLUTIONS: &[(u32, u32)] = &[(1920, 1080), (2560, 1440), (3840, 2160)];
const PREFERRED_REFRESH_MILLIHZ: u32 = 60_000;

fn display_change_error(result: DISP_CHANGE) -> &'static str {
    match result {
        DISP_CHANGE_RESTART => "切换显示模式需要重启 Windows",
        DISP_CHANGE_BADMODE => "显示器或显卡不支持此分辨率与刷新率",
        DISP_CHANGE_NOTUPDATED => "Windows 未能保存显示模式",
        DISP_CHANGE_BADFLAGS | DISP_CHANGE_BADPARAM => "Windows 显示 API 参数无效",
        DISP_CHANGE_BADDUALVIEW => "当前双屏模式不支持该显示模式",
        DISP_CHANGE_FAILED => "Windows 显示驱动未能应用该模式",
        _ => "Windows 拒绝了该显示模式",
    }
}

fn enumerate_supported_display_modes(device_name: &str) -> Vec<DisplayModeResponse> {
    let device = to_wide_z(device_name);
    let mut modes = Vec::new();
    for index in 0..256_u32 {
        let mut mode = DEVMODEW {
            dmSize: mem::size_of::<DEVMODEW>() as u16,
            ..Default::default()
        };
        let ok = unsafe {
            EnumDisplaySettingsExW(
                windows::core::PCWSTR(device.as_ptr()),
                ENUM_DISPLAY_SETTINGS_MODE(index),
                &mut mode,
                ENUM_DISPLAY_SETTINGS_FLAGS(0),
            )
            .as_bool()
        };
        if !ok {
            break;
        }
        if mode.dmPelsWidth == 0 || mode.dmPelsHeight == 0 {
            continue;
        }
        let item = display_mode(device_name, &mode);
        if !modes.iter().any(|candidate: &DisplayModeResponse| {
            candidate.width == item.width
                && candidate.height == item.height
                && candidate.refresh_millihz == item.refresh_millihz
        }) {
            modes.push(item);
        }
    }
    modes.sort_by_key(|mode| (mode.width, mode.height, mode.refresh_millihz));
    modes
}

fn select_common_display_modes(
    supported: &[DisplayModeResponse],
    current: &DisplayModeResponse,
) -> Vec<DisplayModeResponse> {
    COMMON_DISPLAY_RESOLUTIONS
        .iter()
        .filter_map(|&(width, height)| {
            supported
                .iter()
                .filter(|mode| mode.width == width && mode.height == height)
                .min_by_key(|mode| {
                    let is_current = mode.width == current.width
                        && mode.height == current.height
                        && mode.refresh_millihz == current.refresh_millihz;
                    (
                        !is_current,
                        mode.refresh_millihz.abs_diff(PREFERRED_REFRESH_MILLIHZ),
                        std::cmp::Reverse(mode.refresh_millihz),
                    )
                })
                .cloned()
        })
        .collect()
}

/// 枚举当前连接到桌面的显示器及其分辨率、刷新率等信息。
///
/// # Errors
/// 检测到有显示器连接到桌面、却读不出其中任何一台的当前显示模式时返回
/// “无法读取任何已连接显示器的当前显示模式”；完全没有可用显示器时返回
/// “未检测到连接到 Windows 桌面的显示器”。
pub fn get_displays() -> anyhow::Result<Vec<DisplayInfoResponse>> {
    let mut displays = Vec::new();
    let mut attached_displays = 0usize;
    for index in 0..32_u32 {
        let mut device = DISPLAY_DEVICEW {
            cb: mem::size_of::<DISPLAY_DEVICEW>() as u32,
            ..Default::default()
        };
        let ok = unsafe { EnumDisplayDevicesW(None, index, &mut device, 0).as_bool() };
        if !ok {
            break;
        }
        if !device
            .StateFlags
            .contains(DISPLAY_DEVICE_ATTACHED_TO_DESKTOP)
        {
            continue;
        }
        attached_displays += 1;
        let id = wide_array_string(&device.DeviceName);
        if id.is_empty() {
            continue;
        }
        let device_name = to_wide_z(&id);
        let mut current = DEVMODEW {
            dmSize: mem::size_of::<DEVMODEW>() as u16,
            ..Default::default()
        };
        let has_current = unsafe {
            EnumDisplaySettingsExW(
                windows::core::PCWSTR(device_name.as_ptr()),
                ENUM_CURRENT_SETTINGS,
                &mut current,
                ENUM_DISPLAY_SETTINGS_FLAGS(0),
            )
            .as_bool()
        };
        if !has_current {
            continue;
        }
        let current_mode = display_mode(&id, &current);
        displays.push(DisplayInfoResponse {
            id: id.clone(),
            name: wide_array_string(&device.DeviceString),
            primary: device.StateFlags.contains(DISPLAY_DEVICE_PRIMARY_DEVICE),
            modes: select_common_display_modes(
                &enumerate_supported_display_modes(&id),
                &current_mode,
            ),
            current_mode,
        });
    }
    anyhow::ensure!(
        attached_displays == 0 || !displays.is_empty(),
        "无法读取任何已连接显示器的当前显示模式"
    );
    anyhow::ensure!(!displays.is_empty(), "未检测到连接到 Windows 桌面的显示器");
    Ok(displays)
}

/// 为指定显示器切换分辨率与刷新率；会先校验目标模式是否受支持。
///
/// # Errors
/// 读不到当前显示模式（“无法读取当前显示模式”）；目标模式不在常用分辨率候选列表中
/// （“该显示模式不在常用分辨率候选列表中”）；或 `ChangeDisplaySettingsExW` 未返回
/// `ChangeDisplaySettingsExW` 未能应用目标模式时返回错误。
pub fn set_display_mode(
    display_id: &str,
    width: u32,
    height: u32,
    refresh_millihz: u32,
) -> anyhow::Result<()> {
    let device = to_wide_z(display_id);
    let mut mode = DEVMODEW {
        dmSize: mem::size_of::<DEVMODEW>() as u16,
        ..Default::default()
    };
    let ok = unsafe {
        EnumDisplaySettingsExW(
            windows::core::PCWSTR(device.as_ptr()),
            ENUM_CURRENT_SETTINGS,
            &mut mode,
            ENUM_DISPLAY_SETTINGS_FLAGS(0),
        )
        .as_bool()
    };
    anyhow::ensure!(ok, "无法读取当前显示模式");
    let current_mode = display_mode(display_id, &mode);
    let supported = enumerate_supported_display_modes(display_id);
    let allowed = select_common_display_modes(&supported, &current_mode);
    anyhow::ensure!(
        allowed.iter().any(|candidate| {
            candidate.width == width
                && candidate.height == height
                && candidate.refresh_millihz == refresh_millihz
        }),
        "该显示模式不在常用分辨率候选列表中"
    );
    mode.dmPelsWidth = width;
    mode.dmPelsHeight = height;
    mode.dmDisplayFrequency = refresh_millihz / 1000;
    mode.dmFields = DM_PELSWIDTH | DM_PELSHEIGHT | DM_DISPLAYFREQUENCY;
    let result = unsafe {
        ChangeDisplaySettingsExW(
            windows::core::PCWSTR(device.as_ptr()),
            Some(&mode),
            None,
            CDS_TYPE(0),
            None,
        )
    };
    anyhow::ensure!(
        result == DISP_CHANGE_SUCCESSFUL,
        "{}",
        display_change_error(result)
    );
    Ok(())
}

#[cfg(test)]
mod display_mode_tests {
    use super::*;

    fn mode(width: u32, height: u32, refresh_millihz: u32) -> DisplayModeResponse {
        DisplayModeResponse {
            width,
            height,
            refresh_millihz,
        }
    }

    #[test]
    fn common_mode_selection_prefers_current_then_nearest_sixty_hz() {
        let supported = vec![
            mode(1280, 720, 60_000),
            mode(1920, 1080, 60_000),
            mode(1920, 1080, 144_000),
            mode(2560, 1440, 59_000),
            mode(2560, 1440, 60_000),
            mode(3840, 2160, 30_000),
            mode(3840, 2160, 120_000),
        ];

        let selected = select_common_display_modes(&supported, &mode(1920, 1080, 144_000));
        let actual: Vec<_> = selected
            .iter()
            .map(|item| (item.width, item.height, item.refresh_millihz))
            .collect();

        assert_eq!(
            actual,
            vec![
                (1920, 1080, 144_000),
                (2560, 1440, 60_000),
                (3840, 2160, 30_000),
            ]
        );
    }

    #[test]
    fn common_mode_selection_excludes_unsupported_and_nonstandard_resolutions() {
        let supported = vec![
            mode(1280, 720, 60_000),
            mode(1366, 768, 60_000),
            mode(1920, 1080, 60_000),
            mode(3440, 1440, 100_000),
        ];

        let selected = select_common_display_modes(&supported, &mode(1366, 768, 60_000));

        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].width, 1920);
        assert_eq!(selected[0].height, 1080);
        assert_eq!(selected[0].refresh_millihz, 60_000);
    }
}
