//! `HICON` 到 RGBA 位图的转换；颜色位图没有可用 alpha 时回退到 AND 掩码。

use windows::Win32::Graphics::Gdi::{BITMAPINFO, BITMAPINFOHEADER, RGBQUAD};

/// 带完整调色板空间的 `BITMAPINFO`。
///
/// `BITMAPINFO` 只预留一个 `RGBQUAD`，但按索引格式（1bpp 的 AND 掩码）取位时
/// `GetDIBits` 会把整张调色板写进来：1bpp 是两项。传单项目结构会让那次写入越过
/// 结构末尾，踩坏编译器紧挨着放的局部变量——release 构建下正是它让所有图标解析
/// 都返回"没有图标"。
#[repr(C)]
struct DibInfo {
    header: BITMAPINFOHEADER,
    /// 8bpp 调色板是 256 项；这里只请求 1bpp，空间绰绰有余。
    colors: [RGBQUAD; 256],
}

impl DibInfo {
    fn new(header: BITMAPINFOHEADER) -> Self {
        Self {
            header,
            colors: [RGBQUAD::default(); 256],
        }
    }

    /// 交给 `GetDIBits` 的指针：头部在结构首位，布局与 `BITMAPINFO` 一致。
    fn as_bitmap_info(&mut self) -> *mut BITMAPINFO {
        std::ptr::addr_of_mut!(self.header).cast::<BITMAPINFO>()
    }
}

/// Convert an `HICON` into an RGBA image, applying the monochrome AND mask when
/// the color bitmap carries no alpha channel of its own.
#[allow(unsafe_op_in_unsafe_fn, clippy::missing_transmute_annotations)]
pub(super) unsafe fn icon_to_image(
    hicon: windows::Win32::UI::WindowsAndMessaging::HICON,
) -> Option<image::RgbaImage> {
    use windows::Win32::Graphics::Gdi::{GetDC, ReleaseDC};
    use windows::Win32::UI::WindowsAndMessaging::{GetIconInfo, ICONINFO};

    let mut icon_info: ICONINFO = std::mem::zeroed();
    if GetIconInfo(hicon, &mut icon_info).is_err() {
        return None;
    }

    let hdc_screen = GetDC(None);
    let hdc_mem = windows::Win32::Graphics::Gdi::CreateCompatibleDC(Some(hdc_screen));
    let _ = ReleaseDC(None, hdc_screen);

    if hdc_mem.is_invalid() {
        release_icon_resources(&icon_info, hdc_mem);
        return None;
    }

    let Some((pixels, width, height, color_lines)) = read_color_bitmap(&icon_info, hdc_mem) else {
        release_icon_resources(&icon_info, hdc_mem);
        return None;
    };
    let mask = read_and_mask(&icon_info, hdc_mem, width, height);

    release_icon_resources(&icon_info, hdc_mem);

    if color_lines != height as i32 {
        return None;
    }

    let rgba = build_rgba(&pixels, width, mask.as_ref());

    // The raw canvas is returned as-is: normalisation (crop to the artwork, fill
    // the square, centre it) happens once, in `encode_icon_payload`, so every
    // source in the chain produces the same shape of payload.
    image::RgbaImage::from_raw(width, height, rgba)
}

/// 释放 `GetIconInfo` 返回的颜色位图、掩码位图与内存 DC。
#[allow(unsafe_op_in_unsafe_fn, clippy::missing_transmute_annotations)]
unsafe fn release_icon_resources(
    info: &windows::Win32::UI::WindowsAndMessaging::ICONINFO,
    dc: windows::Win32::Graphics::Gdi::HDC,
) {
    use windows::Win32::Graphics::Gdi::{DeleteDC, DeleteObject};

    if !info.hbmColor.is_invalid() {
        let _ = DeleteObject(std::mem::transmute(info.hbmColor));
    }
    if !info.hbmMask.is_invalid() {
        let _ = DeleteObject(std::mem::transmute(info.hbmMask));
    }
    if !dc.is_invalid() {
        let _ = DeleteDC(dc);
    }
}

/// 读取颜色位图的 32bpp 像素；返回 (像素, 宽, 高, `GetDIBits` 实际写入行数)。
#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn read_color_bitmap(
    icon_info: &windows::Win32::UI::WindowsAndMessaging::ICONINFO,
    hdc_mem: windows::Win32::Graphics::Gdi::HDC,
) -> Option<(Vec<u32>, u32, u32, i32)> {
    use windows::Win32::Graphics::Gdi::{BI_RGB, BITMAPINFOHEADER, DIB_RGB_COLORS, GetDIBits};

    let mut bitmap = windows::Win32::Graphics::Gdi::BITMAP::default();
    let bitmap_size = std::mem::size_of::<windows::Win32::Graphics::Gdi::BITMAP>() as i32;
    let object_size = windows::Win32::Graphics::Gdi::GetObjectW(
        icon_info.hbmColor.into(),
        bitmap_size,
        Some(&mut bitmap as *mut _ as *mut _),
    );
    if object_size == 0 {
        return None;
    }

    let width = bitmap.bmWidth as u32;
    let height = bitmap.bmHeight as u32;
    if width == 0 || height == 0 {
        return None;
    }

    let mut color_info = DibInfo::new(BITMAPINFOHEADER {
        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
        biWidth: bitmap.bmWidth,
        biHeight: -bitmap.bmHeight, // top-down
        biPlanes: 1,
        biBitCount: 32,
        biCompression: BI_RGB.0,
        ..Default::default()
    });
    let mut pixels = vec![0u32; (width * height) as usize];
    // The bitmap must not be selected into a device context for GetDIBits.
    let color_lines = GetDIBits(
        hdc_mem,
        icon_info.hbmColor,
        0,
        height,
        Some(pixels.as_mut_ptr() as *mut _),
        color_info.as_bitmap_info(),
        DIB_RGB_COLORS,
    );

    Some((pixels, width, height, color_lines))
}

/// 1bpp AND 掩码：自上而下的字节内容与每行字节数。
struct AndMask {
    /// 逐行的掩码字节。
    bytes: Vec<u8>,
    /// 每行字节数（按 32 位对齐）。
    stride: usize,
}

/// 读取 1bpp AND 掩码；掩码缺失或写入行数不足时返回 `None`。
#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn read_and_mask(
    icon_info: &windows::Win32::UI::WindowsAndMessaging::ICONINFO,
    hdc_mem: windows::Win32::Graphics::Gdi::HDC,
    width: u32,
    height: u32,
) -> Option<AndMask> {
    use windows::Win32::Graphics::Gdi::{BI_RGB, BITMAPINFOHEADER, DIB_RGB_COLORS, GetDIBits};

    // Read the 1bpp AND mask so icons without an alpha channel stay readable.
    if icon_info.hbmMask.is_invalid() {
        return None;
    }

    let mask_stride = (width as usize).div_ceil(32) * 4;
    let mut mask_info = DibInfo::new(BITMAPINFOHEADER {
        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
        biWidth: width as i32,
        biHeight: -(height as i32),
        biPlanes: 1,
        biBitCount: 1,
        biCompression: BI_RGB.0,
        ..Default::default()
    });
    let mut mask_bytes = vec![0u8; mask_stride * height as usize];
    let mask_lines = GetDIBits(
        hdc_mem,
        icon_info.hbmMask,
        0,
        height,
        Some(mask_bytes.as_mut_ptr() as *mut _),
        mask_info.as_bitmap_info(),
        DIB_RGB_COLORS,
    );
    if mask_lines == height as i32 {
        Some(AndMask {
            bytes: mask_bytes,
            stride: mask_stride,
        })
    } else {
        None
    }
}

/// 判断颜色位图的 alpha 通道是否真的可用。
///
/// Some icons carry no usable alpha channel at all: the colour bitmap is
/// 32bpp but every alpha byte is zero, and the shape lives in the AND mask
/// (measured on e.g. `ys9.exe`, whose alpha maxes out at 0 while the mask
/// marks 72% of the canvas opaque). A handful of stray non-zero alpha bytes,
/// however, must not make us discard a mask that is doing the real work — so
/// only trust the alpha channel when it is actually populated.
fn has_usable_alpha(pixels: &[u32]) -> bool {
    let alpha_pixels = pixels
        .iter()
        .filter(|pixel| (**pixel >> 24) & 0xFF != 0)
        .count();
    let alpha_max = pixels
        .iter()
        .map(|pixel| (pixel >> 24) & 0xFF)
        .max()
        .unwrap_or(0);
    alpha_max >= 250 || alpha_pixels * 100 >= pixels.len()
}

/// 把 32bpp BGRA 像素展开成 RGBA；颜色位图没有可用 alpha 时用 AND 掩码定透明度。
fn build_rgba(pixels: &[u32], width: u32, mask: Option<&AndMask>) -> Vec<u8> {
    let has_alpha = has_usable_alpha(pixels);
    let mut rgba = Vec::with_capacity(pixels.len() * 4);
    for (index, &pixel) in pixels.iter().enumerate() {
        let blue = (pixel & 0xFF) as u8;
        let green = ((pixel >> 8) & 0xFF) as u8;
        let red = ((pixel >> 16) & 0xFF) as u8;
        let alpha = if has_alpha {
            ((pixel >> 24) & 0xFF) as u8
        } else if let Some(mask) = mask {
            and_mask_alpha(mask, index, width)
        } else {
            255
        };
        rgba.push(red);
        rgba.push(green);
        rgba.push(blue);
        rgba.push(alpha);
    }
    rgba
}

/// 按 AND 掩码取单个像素的透明度。
fn and_mask_alpha(mask: &AndMask, index: usize, width: u32) -> u8 {
    let row = index / width as usize;
    let column = index % width as usize;
    let byte = mask
        .bytes
        .get(row * mask.stride + column / 8)
        .copied()
        .unwrap_or(0xFF);
    // An AND mask bit of 1 means transparent.
    if (byte >> (7 - (column % 8))) & 1 == 0 {
        255
    } else {
        0
    }
}
