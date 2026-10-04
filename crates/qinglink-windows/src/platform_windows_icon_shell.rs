//! 外壳默认图标：`SHGetFileInfoW` 与图像列表（`SHGetImageList`）。

use super::ComApartment;
use super::icon_bitmap::icon_to_image;
use super::icon_normalize::ink_bounds;
use super::icon_resource::icon_payload;

/// The shell's generic icon for a path. This is the last resort in the
/// resolution chain: the artwork is not in the file, it is whatever the shell
/// draws for that kind of path.
pub fn extract_shell_icon_base64(path: &str) -> Option<String> {
    icon_payload(extract_shell_icon(path))
}

/// The shell's generic icon, taken from whichever image list carries the most
/// artwork for this path.
#[allow(unsafe_op_in_unsafe_fn)]
fn extract_shell_icon(path: &str) -> Option<image::RgbaImage> {
    use windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES;
    use windows::Win32::UI::Controls::IImageList;
    use windows::Win32::UI::Shell::{
        SHGFI_SYSICONINDEX, SHGetFileInfoW, SHGetImageList, SHIL_EXTRALARGE, SHIL_JUMBO, SHIL_LARGE,
    };

    let wide: Vec<u16> = path.encode_utf16().chain(Some(0)).collect();
    let _com = ComApartment::init().ok();

    unsafe {
        let mut file_info = std::mem::zeroed::<windows::Win32::UI::Shell::SHFILEINFOW>();
        let result = SHGetFileInfoW(
            windows::core::PCWSTR(wide.as_ptr()),
            FILE_FLAGS_AND_ATTRIBUTES(0x80), // FILE_ATTRIBUTE_NORMAL
            Some(&mut file_info),
            std::mem::size_of::<windows::Win32::UI::Shell::SHFILEINFOW>() as u32,
            SHGFI_SYSICONINDEX,
        );
        if result == 0 {
            return extract_large_icon(&wide);
        }

        // Every list is normalised before it is compared, so the comparison has
        // to happen on the raw artwork: the canvas sizes are not comparable
        // (jumbo pastes a 96px artwork into a much larger canvas, extralarge
        // scales it to fit its own), but the artwork's longest edge is.
        let mut best: Option<image::RgbaImage> = None;
        let mut best_edge = 0u32;
        for list_kind in [SHIL_JUMBO, SHIL_EXTRALARGE, SHIL_LARGE] {
            let Ok(list) = SHGetImageList::<IImageList>(list_kind as i32) else {
                continue;
            };
            // ILD_TRANSPARENT (0x1) preserves the icon's alpha channel.
            let Ok(hicon) = list.GetIcon(file_info.iIcon, 0x1) else {
                continue;
            };
            let raw = icon_to_image(hicon);
            let _ = windows::Win32::UI::WindowsAndMessaging::DestroyIcon(hicon);
            let Some(raw) = raw else { continue };
            let Some((_, _, width, height)) = ink_bounds(&raw) else {
                continue;
            };
            let edge = width.max(height);
            if best.is_none() || edge > best_edge {
                best_edge = edge;
                best = Some(raw);
            }
        }
        best.or_else(|| extract_large_icon(&wide))
    }
}

/// The 32px `SHGFI_LARGEICON` result, used when the image lists are unavailable.
#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn extract_large_icon(wide: &[u16]) -> Option<image::RgbaImage> {
    use windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES;
    use windows::Win32::UI::Shell::{SHGFI_ICON, SHGFI_LARGEICON, SHGetFileInfoW};
    use windows::Win32::UI::WindowsAndMessaging::DestroyIcon;

    let mut info = std::mem::zeroed::<windows::Win32::UI::Shell::SHFILEINFOW>();
    let result = SHGetFileInfoW(
        windows::core::PCWSTR(wide.as_ptr()),
        FILE_FLAGS_AND_ATTRIBUTES(0x80),
        Some(&mut info),
        std::mem::size_of::<windows::Win32::UI::Shell::SHFILEINFOW>() as u32,
        SHGFI_ICON | SHGFI_LARGEICON,
    );
    if result == 0 || info.hIcon.is_invalid() {
        return None;
    }
    let image = icon_to_image(info.hIcon);
    let _ = DestroyIcon(info.hIcon);
    image
}
