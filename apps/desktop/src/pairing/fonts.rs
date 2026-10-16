//! 提示窗口的字体。
//!
//! 两套字体都从系统的"消息字体"派生，而不是写死字号或字体名：中文界面在 125% / 150%
//! 缩放下必须跟着系统走，否则窗口里的字会又小又糊。字体在窗口销毁后才释放——被选进
//! DC 的字体不能提前删除。

use anyhow::{Context, Result};
use windows::Win32::Graphics::Gdi::{
    CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, CreateFontW, DEFAULT_CHARSET, DEFAULT_PITCH,
    DeleteObject, FF_DONTCARE, FW_NORMAL, FW_SEMIBOLD, HFONT, OUT_DEFAULT_PRECIS,
};
use windows::Win32::UI::WindowsAndMessaging::{
    NONCLIENTMETRICSW, SPI_GETNONCLIENTMETRICS, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS,
    SystemParametersInfoW,
};
use windows::core::PCWSTR;

/// 配对码字号的放大倍数：它必须在房间另一头也读得出来。
const PIN_FONT_SCALE: i32 = 2;

/// 提示窗口用到的两套字体。
#[derive(Clone, Copy, Debug)]
pub(super) struct Fonts {
    /// 说明文字与按钮。
    pub(super) normal: HFONT,
    /// 配对码：更大、更粗。
    pub(super) pin: HFONT,
}

/// 按系统消息字体创建两套字体。
///
/// # Errors
/// 读不到 `SPI_GETNONCLIENTMETRICS` 或 `CreateFontW` 返回无效句柄时返回错误；已创建的
/// 字体在本函数内释放，调用方不需要善后。
pub(super) fn create() -> Result<Fonts> {
    let mut metrics = NONCLIENTMETRICSW {
        cbSize: std::mem::size_of::<NONCLIENTMETRICSW>() as u32,
        ..Default::default()
    };
    // SAFETY: `metrics` 已按结构体自身大小初始化，调用只写入它。
    unsafe {
        SystemParametersInfoW(
            SPI_GETNONCLIENTMETRICS,
            metrics.cbSize,
            Some(&mut metrics as *mut _ as *mut core::ffi::c_void),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
    }
    .context("cannot read the system message font")?;

    let face = PCWSTR(metrics.lfMessageFont.lfFaceName.as_ptr());
    // SAFETY: 面名指向本地数组，在整个调用期间存活。
    let normal = unsafe {
        CreateFontW(
            metrics.lfMessageFont.lfHeight,
            0,
            0,
            0,
            FW_NORMAL.0 as i32,
            0,
            0,
            0,
            DEFAULT_CHARSET,
            OUT_DEFAULT_PRECIS,
            CLIP_DEFAULT_PRECIS,
            CLEARTYPE_QUALITY,
            DEFAULT_PITCH.0 as u32 | FF_DONTCARE.0 as u32,
            face,
        )
    };
    // SAFETY: 同上。
    let pin = unsafe {
        CreateFontW(
            metrics.lfMessageFont.lfHeight * PIN_FONT_SCALE,
            0,
            0,
            0,
            FW_SEMIBOLD.0 as i32,
            0,
            0,
            0,
            DEFAULT_CHARSET,
            OUT_DEFAULT_PRECIS,
            CLIP_DEFAULT_PRECIS,
            CLEARTYPE_QUALITY,
            DEFAULT_PITCH.0 as u32 | FF_DONTCARE.0 as u32,
            face,
        )
    };
    if normal.is_invalid() || pin.is_invalid() {
        destroy(normal);
        destroy(pin);
        anyhow::bail!("cannot create the pairing prompt fonts");
    }
    Ok(Fonts { normal, pin })
}

/// 释放两套字体；必须等到窗口销毁之后再调用。
pub(super) fn destroy_fonts(fonts: Fonts) {
    destroy(fonts.normal);
    destroy(fonts.pin);
}

fn destroy(font: HFONT) {
    if !font.is_invalid() {
        // SAFETY: 字体已经不再被任何 DC 选中。
        let _ = unsafe { DeleteObject(font.into()) };
    }
}
