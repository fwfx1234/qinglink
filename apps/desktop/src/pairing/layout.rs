//! 提示窗口的几何与子控件：尺寸、位置、DPI 换算与控件创建。
//!
//! 布局用 96 DPI 的逻辑像素描述，建窗时统一换算成物理像素。窗口本身是 PerMonitorV2
//! 感知的（见 `qinglink.exe.manifest`），而提示窗只出现在主显示器上，因此这里用主屏
//! 的系统 DPI 就够；换成任意显示器居中时才需要按窗口的 DPI 重算。

use anyhow::{Context, Result};
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{GetDC, GetDeviceCaps, HFONT, LOGPIXELSY, ReleaseDC};
use windows::Win32::UI::WindowsAndMessaging::{
    AdjustWindowRectEx, BS_PUSHBUTTON, CreateWindowExW, GetSystemMetrics, HMENU, SM_CXSCREEN,
    SM_CYSCREEN, SPI_GETWORKAREA, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SendMessageW,
    SystemParametersInfoW, WINDOW_EX_STYLE, WINDOW_STYLE, WM_SETFONT, WS_CHILD, WS_TABSTOP,
    WS_VISIBLE,
};
use windows::core::{PCWSTR, w};

use super::fonts::Fonts;

/// 说明文字与按钮的控件 id。
pub(super) const DEVICE_ID: i32 = 1001;
pub(super) const HINT_ID: i32 = 1002;
pub(super) const PIN_ID: i32 = 1003;
pub(super) const COUNTDOWN_ID: i32 = 1004;
pub(super) const CANCEL_ID: i32 = 1005;

/// `SS_CENTER`：静态文本水平居中。
///
/// 静态样式常量在 `Win32::System::SystemServices` 下，为一条居中标志引入整块 feature
/// 不划算，因此按 Win32 头文件取值。
const SS_CENTER: u32 = 0x0000_0001;

/// 客户区逻辑尺寸（96 DPI 像素）。
const CLIENT_WIDTH: i32 = 420;
const CLIENT_HEIGHT: i32 = 244;
const MARGIN: i32 = 24;
const DEVICE_TOP: i32 = 18;
const DEVICE_HEIGHT: i32 = 24;
const HINT_TOP: i32 = 48;
const HINT_HEIGHT: i32 = 18;
const PIN_TOP: i32 = 72;
const PIN_HEIGHT: i32 = 56;
const COUNTDOWN_TOP: i32 = 134;
const COUNTDOWN_HEIGHT: i32 = 22;
const CANCEL_WIDTH: i32 = 132;
const CANCEL_HEIGHT: i32 = 32;
const CANCEL_BOTTOM_GAP: i32 = 18;

/// 提示窗口里会随配对请求变化的控件句柄。
///
/// 说明行（"请在手机上输入下面的配对码"）建好后不再变动，因此不在这里保存。
#[derive(Clone, Copy, Debug)]
pub(super) struct Controls {
    pub(super) device: HWND,
    pub(super) pin: HWND,
    pub(super) countdown: HWND,
    pub(super) cancel: HWND,
}

/// 窗口总尺寸：把期望的客户区换算成含标题栏与边框的窗口矩形。
pub(super) fn window_size(
    style: WINDOW_STYLE,
    ex_style: WINDOW_EX_STYLE,
    dpi: i32,
) -> Result<(i32, i32)> {
    let mut rect = RECT {
        left: 0,
        top: 0,
        right: scale(CLIENT_WIDTH, dpi),
        bottom: scale(CLIENT_HEIGHT, dpi),
    };
    // SAFETY: `rect` 是有效本地值，样式与创建窗口时完全一致。
    unsafe { AdjustWindowRectEx(&mut rect, style, false.into(), ex_style) }
        .context("cannot measure the pairing prompt window")?;
    Ok((rect.right - rect.left, rect.bottom - rect.top))
}

/// 在工作区里居中；读不到工作区时退回整个主屏。
pub(super) fn centered_position(width: i32, height: i32) -> (i32, i32) {
    let mut area = RECT::default();
    // SAFETY: `area` 是有效本地值；查询失败时保持全零并由下面的兜底覆盖。
    let measured = unsafe {
        SystemParametersInfoW(
            SPI_GETWORKAREA,
            0,
            Some(&mut area as *mut _ as *mut core::ffi::c_void),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
    }
    .is_ok();
    let (left, top, right, bottom) = if measured {
        (area.left, area.top, area.right, area.bottom)
    } else {
        // SAFETY: 两个调用都只读取全局度量。
        let (width, height) =
            unsafe { (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN)) };
        (0, 0, width, height)
    };
    (
        left + ((right - left) - width) / 2,
        top + ((bottom - top) - height) / 2,
    )
}

/// 主显示器的系统 DPI。布局与字体都按它从 96 DPI 的逻辑像素换算。
pub(super) fn system_dpi() -> i32 {
    // SAFETY: 屏幕 DC 由系统拥有，用完立即释放。
    let hdc = unsafe { GetDC(None) };
    if hdc.is_invalid() {
        return 96;
    }
    // SAFETY: `hdc` 有效，`LOGPIXELSY` 只读取该 DC 的度量。
    let dpi = unsafe { GetDeviceCaps(Some(hdc), LOGPIXELSY) };
    // SAFETY: `hdc` 来自 `GetDC`，只释放一次。
    unsafe {
        ReleaseDC(None, hdc);
    }
    if dpi > 0 { dpi } else { 96 }
}

fn scale(value: i32, dpi: i32) -> i32 {
    value * dpi / 96
}

/// 建齐所有子控件并装上字体；失败时调用方负责销毁父窗口。
pub(super) fn create_controls(
    parent: HWND,
    instance: HINSTANCE,
    fonts: &Fonts,
    dpi: i32,
) -> Result<Controls> {
    let text_width = scale(CLIENT_WIDTH - 2 * MARGIN, dpi);
    let centered_static = WINDOW_STYLE(SS_CENTER) | WS_CHILD | WS_VISIBLE;

    let device = create_control(
        parent,
        instance,
        ControlSpec {
            class: w!("STATIC"),
            id: DEVICE_ID,
            style: centered_static,
            position: (scale(MARGIN, dpi), scale(DEVICE_TOP, dpi)),
            size: (text_width, scale(DEVICE_HEIGHT, dpi)),
            text: w!(""),
        },
        fonts.normal,
    )?;
    create_control(
        parent,
        instance,
        ControlSpec {
            class: w!("STATIC"),
            id: HINT_ID,
            style: centered_static,
            position: (scale(MARGIN, dpi), scale(HINT_TOP, dpi)),
            size: (text_width, scale(HINT_HEIGHT, dpi)),
            text: w!("请在手机上输入下面的配对码"),
        },
        fonts.normal,
    )?;
    let pin = create_control(
        parent,
        instance,
        ControlSpec {
            class: w!("STATIC"),
            id: PIN_ID,
            style: centered_static,
            position: (scale(MARGIN, dpi), scale(PIN_TOP, dpi)),
            size: (text_width, scale(PIN_HEIGHT, dpi)),
            text: w!(""),
        },
        fonts.pin,
    )?;
    let countdown = create_control(
        parent,
        instance,
        ControlSpec {
            class: w!("STATIC"),
            id: COUNTDOWN_ID,
            style: centered_static,
            position: (scale(MARGIN, dpi), scale(COUNTDOWN_TOP, dpi)),
            size: (text_width, scale(COUNTDOWN_HEIGHT, dpi)),
            text: w!(""),
        },
        fonts.normal,
    )?;

    let cancel_width = scale(CANCEL_WIDTH, dpi);
    let cancel = create_control(
        parent,
        instance,
        ControlSpec {
            class: w!("BUTTON"),
            id: CANCEL_ID,
            style: WINDOW_STYLE(BS_PUSHBUTTON as u32) | WS_CHILD | WS_VISIBLE | WS_TABSTOP,
            position: (
                (scale(CLIENT_WIDTH, dpi) - cancel_width) / 2,
                scale(CLIENT_HEIGHT - CANCEL_BOTTOM_GAP - CANCEL_HEIGHT, dpi),
            ),
            size: (cancel_width, scale(CANCEL_HEIGHT, dpi)),
            text: w!("取消配对"),
        },
        fonts.normal,
    )?;

    Ok(Controls {
        device,
        pin,
        countdown,
        cancel,
    })
}

/// 一个子控件的创建参数。
struct ControlSpec {
    class: PCWSTR,
    id: i32,
    style: WINDOW_STYLE,
    position: (i32, i32),
    size: (i32, i32),
    text: PCWSTR,
}

/// 建一个子控件并装上字体；坐标与尺寸已按 DPI 换算。
fn create_control(
    parent: HWND,
    instance: HINSTANCE,
    spec: ControlSpec,
    font: HFONT,
) -> Result<HWND> {
    let (x, y) = spec.position;
    let (width, height) = spec.size;
    // 子控件的"菜单句柄"就是控件 id，这是 Win32 的历史约定。
    let menu = HMENU(spec.id as usize as *mut core::ffi::c_void);
    // SAFETY: 父窗口属于本线程且类已注册（STATIC / BUTTON 是系统类）。
    let control = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            spec.class,
            spec.text,
            spec.style,
            x,
            y,
            width,
            height,
            Some(parent),
            Some(menu),
            Some(instance),
            None,
        )
    }
    .with_context(|| format!("cannot create pairing prompt control {}", spec.id))?;
    // SAFETY: 控件刚创建、属于本线程；`WM_SETFONT` 只装字体，`LPARAM(1)` 表示立即重绘。
    unsafe {
        SendMessageW(
            control,
            WM_SETFONT,
            Some(WPARAM(font.0 as usize)),
            Some(LPARAM(1)),
        );
    }
    Ok(control)
}
