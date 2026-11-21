//! 显示枚举与模式设置：读取已连接显示器、它的当前模式与手机端可选的显示模式，并按需切换。

use std::collections::HashMap;
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

/// 枚举显示模式的上限——这是防止驱动异常时死循环的安全阀，不是窗口。
///
/// 早期实现把它写成 `0..256`，看似"够用"，实际会在到点时**静默截断**：Windows 按分辨率
/// 从小到大返回模式，4K@120 的面板实测能枚举出 705 条，256 条只覆盖到 720p 以下，
/// 1080p/1440p/4K 全部被截掉——手机端因此只剩两个低分辨率可选。
const MAX_ENUMERATED_DISPLAY_MODES: u32 = 4096;

fn enumerate_supported_display_modes(device_name: &str) -> Vec<DisplayModeResponse> {
    let device = to_wide_z(device_name);
    let mut modes = Vec::new();
    for index in 0..MAX_ENUMERATED_DISPLAY_MODES {
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

/// 把枚举到的模式整理成给客户端展示的顺序。
///
/// 按"分辨率面积从大到小、同分辨率下刷新率从高到低"排序，让手机端的列表与 Windows
/// 显示设置的观感一致；同时保证同一分辨率的条目连续，因为客户端按 `宽x高` 分组。
fn ordered_display_modes(supported: &[DisplayModeResponse]) -> Vec<DisplayModeResponse> {
    let mut modes = supported.to_vec();
    modes.sort_by(|left, right| {
        let left_area = u64::from(left.width) * u64::from(left.height);
        let right_area = u64::from(right.width) * u64::from(right.height);
        right_area
            .cmp(&left_area)
            .then_with(|| right.width.cmp(&left.width))
            .then_with(|| right.refresh_millihz.cmp(&left.refresh_millihz))
    });
    modes
}

/// 手机端可选模式的最低分辨率：1080p。
///
/// 手机端切分辨率的目的只有"看得更清楚"，720p/900p 这类档位只会把列表灌满，
/// 因此按 `宽 ≥ 1920 且高 ≥ 1080` 过滤，不设上限（4K、带鱼屏、16:10 都是真实模式）。
const MIN_DISPLAY_WIDTH: u32 = 1920;
const MIN_DISPLAY_HEIGHT: u32 = 1080;

/// 同一分辨率下最多保留的刷新率档位数，从高到低取。
const MAX_REFRESH_PER_RESOLUTION: usize = 4;

/// 毫赫兹刷新率归一到整 Hz。
///
/// 取整口径与 `set_display_mode` 交给 Win32 的 `(refresh_millihz + 500) / 1000` 保持一致：
/// `59940` 与 `60000` 都归到 60，`119880` 归到 120，`143981` 归到 144。
/// 用 `saturating_add` 是因为异常驱动值下 `dmDisplayFrequency * 1000` 可能贴近 `u32::MAX`。
fn refresh_hertz(refresh_millihz: u32) -> u32 {
    refresh_millihz.saturating_add(500) / 1000
}

/// 是否为 1080p 及以上的模式。
fn is_hd_or_above(mode: &DisplayModeResponse) -> bool {
    mode.width >= MIN_DISPLAY_WIDTH && mode.height >= MIN_DISPLAY_HEIGHT
}

/// 两条模式是否属于同一个"分辨率 + 刷新率"档位。
///
/// 刷新率按整 Hz 归档，且允许 **±1Hz** 的差：`DEVMODEW.dmDisplayFrequency` 是整数，
/// 59.94Hz 的面板会被 Windows 报成 `59`、119.88Hz 报成 `119`，而同一块屏通常还枚举出
/// `60` / `120`。不合并的话手机端会出现"120Hz / 119Hz / 60Hz / 59Hz"四颗几乎一样的胶囊。
fn same_mode_bucket(left: &DisplayModeResponse, right: &DisplayModeResponse) -> bool {
    left.width == right.width
        && left.height == right.height
        && refresh_hertz(left.refresh_millihz).abs_diff(refresh_hertz(right.refresh_millihz)) <= 1
}

/// 同一档位里 `candidate` 是否比 `existing` 更该保留。
///
/// 当前模式优先：客户端用逐位相等判断"当前"（`DisplayInfo.isCurrent`），一旦把当前模式
/// 换成相邻的 `60000`，弹层里的高亮就会消失。没有当前模式参与时取毫赫兹数值更大者，
/// 因此 `60000` 会胜过 `59940`。
fn keep_over(
    candidate: &DisplayModeResponse,
    existing: &DisplayModeResponse,
    current: &DisplayModeResponse,
) -> bool {
    if *existing == *current {
        return false;
    }
    if *candidate == *current {
        return true;
    }
    candidate.refresh_millihz > existing.refresh_millihz
}

/// 同一分辨率下最多保留 [`MAX_REFRESH_PER_RESOLUTION`] 档刷新率。
///
/// 一条 4K@120 的枚举结果里，同一分辨率会带出 120/60/50/30/25/24 六档**真实**刷新率，
/// 全塞进手机端只会变成噪音；保留最高的几档即可，当前生效的那条永远不被裁掉。
fn trim_refresh_rates(
    mut modes: Vec<DisplayModeResponse>,
    current: &DisplayModeResponse,
) -> Vec<DisplayModeResponse> {
    modes.sort_by(|left, right| {
        (right.width, right.height)
            .cmp(&(left.width, left.height))
            .then_with(|| right.refresh_millihz.cmp(&left.refresh_millihz))
    });
    let mut counts: HashMap<(u32, u32), usize> = HashMap::new();
    modes.retain(|mode| {
        let count = counts.entry((mode.width, mode.height)).or_insert(0);
        if *count < MAX_REFRESH_PER_RESOLUTION || *mode == *current {
            *count += 1;
            true
        } else {
            false
        }
    });
    modes
}

/// 把枚举结果裁剪成手机端可选的模式列表。
///
/// 规则：
/// 1. 只保留 1080p 及以上的模式（[`is_hd_or_above`]）；整台显示器连 1080p 都拿不出来时
///    取消这层过滤——老设备仍然要能切到自己那块屏的低分辨率，否则列表会空。
/// 2. 每条候选都必须是该显示器**真实枚举到**的组合，绝不凭空合成刷新率：枚举里只有
///    `59940` 时就返回 `59940`，造一个 `60000` 会让 `set_display_mode` 的全量校验拒绝设置。
/// 3. 同一分辨率、同一整 Hz 档只留一条；同一分辨率的刷新率档位另有数量上限。
/// 4. 当前生效的模式一定在结果里：哪怕它是 720p、1680×1050 或 5120×1440，也会原样追加，
///    否则弹层无法高亮，用户切走之后就切不回来。
fn selectable_display_modes(
    supported: &[DisplayModeResponse],
    current: &DisplayModeResponse,
) -> Vec<DisplayModeResponse> {
    let has_hd_mode = supported.iter().any(is_hd_or_above);
    let mut modes: Vec<DisplayModeResponse> = Vec::new();
    for candidate in supported
        .iter()
        .filter(|mode| !has_hd_mode || is_hd_or_above(mode))
    {
        match modes
            .iter_mut()
            .find(|mode| same_mode_bucket(mode, candidate))
        {
            Some(existing) => {
                if keep_over(candidate, existing, current) {
                    *existing = candidate.clone();
                }
            }
            None => modes.push(candidate.clone()),
        }
    }
    let mut modes = trim_refresh_rates(modes, current);
    if !modes.contains(current) {
        modes.push(current.clone());
    }
    ordered_display_modes(&modes)
}

/// 把毫赫兹刷新率格式化成人类可读的 Hz，用于错误文案。
fn refresh_text(refresh_millihz: u32) -> String {
    let hz = f64::from(refresh_millihz) / 1000.0;
    if (hz - hz.round()).abs() < 0.005 {
        format!("{:.0}Hz", hz)
    } else {
        format!("{hz:.2}Hz")
    }
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
        let modes =
            selectable_display_modes(&enumerate_supported_display_modes(&id), &current_mode);
        displays.push(DisplayInfoResponse {
            id: id.clone(),
            name: wide_array_string(&device.DeviceString),
            primary: device.StateFlags.contains(DISPLAY_DEVICE_PRIMARY_DEVICE),
            modes,
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
/// 读不到当前显示模式（“无法读取当前显示模式”）；目标模式不在该显示器枚举到的模式集合里
/// （“该显示器不支持 2560×1440@144Hz”）；或 `ChangeDisplaySettingsExW` 未返回成功时返回错误。
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
    // 校验对象是显卡/显示器枚举到的"全量"模式，而不是 get_displays 裁剪后的可选模式：
    // 游戏启动前的快照恢复、旧版本保存的低分辨率模式、以及注入列表的当前模式都必须仍能设置。
    let supported = enumerate_supported_display_modes(display_id);
    anyhow::ensure!(
        supported.iter().any(|candidate| {
            candidate.width == width
                && candidate.height == height
                && candidate.refresh_millihz == refresh_millihz
        }),
        "该显示器不支持 {width}×{height}@{}",
        refresh_text(refresh_millihz)
    );
    mode.dmPelsWidth = width;
    mode.dmPelsHeight = height;
    mode.dmDisplayFrequency = (refresh_millihz + 500) / 1000;
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
    fn selectable_modes_keep_1080p_and_above_with_bounded_refresh() {
        // 640×480 / 1024×768 / 1280×720 都在 1080p 以下，必须被丢掉；
        // 3440×1440 这类带鱼屏属于"1080p 及以上"，要保留；
        // 3840×2160 有六档真实刷新率，只留最高的四档。
        let supported = vec![
            mode(640, 480, 60_000),
            mode(1024, 768, 60_000),
            mode(1280, 720, 60_000),
            mode(1920, 1080, 60_000),
            mode(1920, 1080, 75_000),
            mode(1920, 1080, 144_000),
            mode(2560, 1440, 120_000),
            mode(2560, 1440, 165_000),
            mode(3440, 1440, 100_000),
            mode(3840, 2160, 120_000),
            mode(3840, 2160, 60_000),
            mode(3840, 2160, 50_000),
            mode(3840, 2160, 30_000),
            mode(3840, 2160, 25_000),
            mode(3840, 2160, 24_000),
        ];
        let current = mode(1920, 1080, 144_000);

        let actual: Vec<_> = selectable_display_modes(&supported, &current)
            .iter()
            .map(|item| (item.width, item.height, item.refresh_millihz))
            .collect();

        assert_eq!(
            actual,
            vec![
                (3840, 2160, 120_000),
                (3840, 2160, 60_000),
                (3840, 2160, 50_000),
                (3840, 2160, 30_000),
                (3440, 1440, 100_000),
                (2560, 1440, 165_000),
                (2560, 1440, 120_000),
                (1920, 1080, 144_000),
                (1920, 1080, 75_000),
                (1920, 1080, 60_000),
            ]
        );
    }

    #[test]
    fn selectable_modes_merge_decimal_refresh_and_keep_current_mode() {
        // 1920×1080 同时有 59.94 与 60，按整 Hz 去重后只留 60000；
        // 2560×1440 只枚举到 59.94，必须原样保留，不能凭空造 60；
        // 当前模式 1680×1050 低于 1080p，仍要追加进列表以便高亮。
        let supported = vec![
            mode(1920, 1080, 59_940),
            mode(1920, 1080, 60_000),
            mode(2560, 1440, 59_940),
            mode(3840, 2160, 119_880),
        ];
        let current = mode(1680, 1050, 59_940);

        let actual: Vec<_> = selectable_display_modes(&supported, &current)
            .iter()
            .map(|item| (item.width, item.height, item.refresh_millihz))
            .collect();

        assert_eq!(
            actual,
            vec![
                (3840, 2160, 119_880),
                (2560, 1440, 59_940),
                (1920, 1080, 60_000),
                (1680, 1050, 59_940),
            ]
        );
    }
}
