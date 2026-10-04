//! 托盘图标的渲染与 Win32 句柄创建。
//!
//! 图标是**全彩**渲染的品牌标识。早先的实现把每个像素都转成纯黑并保留 alpha，
//! 结果蓝色方块与白色链条塌缩成一块实心黑方块：深色任务栏上看不见，浅色任务栏
//! 上则是一团没有细节的色块。因此彩色是刻意为之，不是装饰——不要在这里重新引入
//! 模板/单色转换。
//!
//! 分两个独立步骤，因为它们的失败原因不同，诊断需要指出是哪一步失败：
//!
//! 1. [`rasterize`] 用 `resvg` 把 SVG 画成直通 RGBA；
//! 2. [`bitmap_from_rgba`] 把这些像素变成真正的 `HICON`——shell 要求的就是它，
//!    位图句柄不是图标句柄。

use anyhow::{Context, Result};
use resvg::usvg::{self, Transform};
use tiny_skia::Pixmap;

#[cfg(target_os = "windows")]
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateBitmap, CreateDIBSection, DIB_RGB_COLORS,
    DeleteObject, HBITMAP,
};
#[cfg(target_os = "windows")]
use windows::Win32::UI::WindowsAndMessaging::{CreateIconIndirect, DestroyIcon, HICON, ICONINFO};

/// The brand mark, compiled into the binary so a copied executable still shows
/// a tray icon.
const TRAY_SVG: &[u8] = include_bytes!("../../assets/tray-icon.svg");

/// Size used when the platform cannot report its own small-icon metric.
const FALLBACK_SIZE: u32 = 16;

/// Largest icon we will rasterize, to bound startup cost on odd metrics.
const MAX_SIZE: u32 = 64;

/// A rendered icon plus the pixel size it was drawn at.
///
/// The size is reported because "the icon is one transparent pixel" and "the
/// icon is the wrong size for this DPI" are both real failures that look
/// identical from the outside.
#[derive(Debug)]
pub struct RenderedIcon {
    /// Raw straight (non-premultiplied) RGBA, `size * size * 4` bytes.
    pub rgba: Vec<u8>,
    /// Width and height in pixels; the mark is square.
    pub size: u32,
}

/// The pixel size the shell expects for a notification-area icon.
///
/// `GetSystemMetrics(SM_CXSMICON)` reports the metric already scaled for this
/// process's DPI awareness (the manifest sets PerMonitorV2), so no separate DPI
/// query is needed.
pub fn target_size() -> u32 {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSMICON};
        // SAFETY: `GetSystemMetrics` only reads a global metric for the given
        // index; it has no preconditions and cannot fail.
        let measured = unsafe { GetSystemMetrics(SM_CXSMICON) };
        if measured > 0 {
            return (measured as u32).clamp(8, MAX_SIZE);
        }
    }
    FALLBACK_SIZE
}

/// Render the brand mark at the platform's small-icon size.
pub fn brand_icon() -> Result<RenderedIcon> {
    let size = target_size();
    Ok(RenderedIcon {
        rgba: rasterize(size)?,
        size,
    })
}

/// A deliberately simple, unmistakably visible icon used when the SVG cannot be
/// rendered or converted.
///
/// Deliberately coloured rather than black: a black glyph disappears against a
/// dark taskbar, which is the exact failure this module exists to prevent.
pub fn fallback_icon() -> RenderedIcon {
    let size = target_size();
    let mut rgba = vec![0_u8; (size * size * 4) as usize];
    let center = size as f32 / 2.0;
    let radius = size as f32 * 0.42;
    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 + 0.5 - center;
            let dy = y as f32 + 0.5 - center;
            if (dx * dx + dy * dy).sqrt() <= radius {
                let index = ((y * size + x) * 4) as usize;
                // Brand blue, fully opaque.
                rgba[index..index + 4].copy_from_slice(&[0x13, 0x7B, 0xC2, 0xFF]);
            }
        }
    }
    RenderedIcon { rgba, size }
}

/// Rasterize the SVG to straight (non-premultiplied) RGBA.
pub fn rasterize(size: u32) -> Result<Vec<u8>> {
    let options = usvg::Options::default();
    let tree = usvg::Tree::from_data(TRAY_SVG, &options)
        .map_err(|error| anyhow::anyhow!("cannot parse the tray SVG: {error}"))?;

    let source = tree.size();
    let longest = source.width().max(source.height());
    anyhow::ensure!(longest > 0.0, "the tray SVG has no drawable area");

    // Preserve a small margin so the mark does not touch the icon bounds.
    let scale = size as f32 / longest * 0.92;
    let offset_x = (size as f32 - source.width() * scale) * 0.5;
    let offset_y = (size as f32 - source.height() * scale) * 0.5;

    let mut pixmap = Pixmap::new(size, size)
        .ok_or_else(|| anyhow::anyhow!("cannot allocate a {size}x{size} icon bitmap"))?;
    pixmap.fill(tiny_skia::Color::TRANSPARENT);
    resvg::render(
        &tree,
        Transform::from_translate(offset_x, offset_y).pre_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    Ok(unpremultiply_rgba(pixmap.data()))
}

/// Convert tiny-skia's premultiplied output into the straight RGBA that the
/// Win32 bitmap expects. Skipping this makes semi-transparent edges appear as
/// dark fringes.
fn unpremultiply_rgba(premultiplied: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity(premultiplied.len());
    for chunk in premultiplied.chunks_exact(4) {
        let [red, green, blue, alpha] = [chunk[0], chunk[1], chunk[2], chunk[3]];
        if alpha == 0 {
            output.extend_from_slice(&[0, 0, 0, 0]);
            continue;
        }
        let scale = alpha as f32 / 255.0;
        output.push((red as f32 / scale).round().min(255.0) as u8);
        output.push((green as f32 / scale).round().min(255.0) as u8);
        output.push((blue as f32 / scale).round().min(255.0) as u8);
        output.push(alpha);
    }
    output
}

/// Build a real `HICON` from straight RGBA pixels.
///
/// The shell needs an icon handle with an alpha channel; handing it a raw
/// bitmap produces an invisible or black square. The mask bitmap must be all
/// zeroes, which is exactly what an alpha-blended 32bpp icon requires.
#[cfg(target_os = "windows")]
pub fn bitmap_from_rgba(rgba: &[u8], size: u32) -> Result<HICON> {
    anyhow::ensure!(size > 0, "the tray icon has zero size");
    anyhow::ensure!(
        rgba.len() >= (size as usize) * (size as usize) * 4,
        "the tray icon buffer is smaller than {}x{size} RGBA",
        size
    );

    // SAFETY: every call below is passed a size or pointer consistent with the
    // buffer we allocated; each handle is either owned by the returned icon or
    // deleted before this function returns.
    unsafe {
        let header = BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: size as i32,
            biHeight: -(size as i32),
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..BITMAPINFOHEADER::default()
        };
        let info = BITMAPINFO {
            bmiHeader: header,
            ..BITMAPINFO::default()
        };
        let mut pixels: *mut core::ffi::c_void = std::ptr::null_mut();
        let colour: HBITMAP = CreateDIBSection(None, &info, DIB_RGB_COLORS, &mut pixels, None, 0)
            .context("cannot allocate the tray icon colour bitmap")?;
        if pixels.is_null() {
            let _ = DeleteObject(colour.into());
            anyhow::bail!("the tray icon colour bitmap has no pixel storage");
        }
        // The DIB section is top-down (`biHeight` negative), so a straight copy
        // keeps the image upright and gives the shell its alpha channel.
        std::ptr::copy_nonoverlapping(
            rgba.as_ptr(),
            pixels as *mut u8,
            (size as usize) * (size as usize) * 4,
        );

        // All-zero mask: the alpha channel decides visibility. A non-zero mask
        // would punch holes in the mark.
        let mask: HBITMAP = CreateBitmap(size as i32, size as i32, 1, 1, None);
        if mask.is_invalid() {
            let _ = DeleteObject(colour.into());
            anyhow::bail!("cannot allocate the tray icon mask bitmap");
        }

        let info_struct = ICONINFO {
            fIcon: true.into(),
            xHotspot: 0,
            yHotspot: 0,
            hbmMask: mask,
            hbmColor: colour,
        };
        let icon = CreateIconIndirect(&info_struct);
        // The icon copies both bitmaps, so ours can go immediately.
        let _ = DeleteObject(colour.into());
        let _ = DeleteObject(mask.into());
        icon.context("cannot assemble the tray icon from its bitmaps")
    }
}

/// Release an icon created by [`bitmap_from_rgba`].
///
/// # Safety
/// `icon` must be a handle returned by [`bitmap_from_rgba`] and not already
/// destroyed.
#[cfg(target_os = "windows")]
pub unsafe fn destroy(icon: HICON) {
    if !icon.is_invalid() {
        // SAFETY: the caller contract guarantees a live icon from this module.
        let _ = unsafe { DestroyIcon(icon) };
    }
}
