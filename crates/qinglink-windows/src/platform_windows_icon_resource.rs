//! 读取文件资源中内嵌的图标（`SHDefExtractIconW`）。

use super::ComApartment;
use super::icon_bitmap::icon_to_image;
use super::icon_normalize::{ICON_REQUEST_EDGE, encode_icon_payload};

/// Extract the icon embedded in a file's resources as a PNG base64 payload.
///
/// This is deliberately *not* the shell image-list route. For a file whose
/// largest artwork is 96px, `SHGetImageList(SHIL_JUMBO)` hands back a 256px
/// canvas (512px on a 200% DPI machine) with that artwork pasted, unscaled,
/// into the top-left corner; keeping the canvas is what made those icons render
/// a few pixels wide. `SHDefExtractIconW` scales the artwork to the requested
/// size instead, and it reports "this file has no icon resource" rather than
/// quietly substituting the shell's generic icon — a distinction the resolution
/// chain needs in order to keep looking.
pub fn extract_icon_base64(path: &str) -> Option<String> {
    extract_resource_icon_base64(path, 0)
}

/// Same as [`extract_icon_base64`], but selects one icon out of the file by
/// resource index — how a `.lnk` points at e.g. `shell32.dll,3`.
pub fn extract_resource_icon_base64(path: &str, index: i32) -> Option<String> {
    icon_payload(extract_resource_icon(path, index))
}

pub(super) fn icon_payload(raw: Option<image::RgbaImage>) -> Option<String> {
    encode_icon_payload(&raw?)
}

/// Read one icon resource out of a file (`.exe` / `.dll` / `.ico`).
#[allow(unsafe_op_in_unsafe_fn)]
fn extract_resource_icon(path: &str, index: i32) -> Option<image::RgbaImage> {
    use windows::Win32::UI::Shell::SHDefExtractIconW;
    use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, HICON};

    // `SHDefExtractIconW` and the image lists both need COM on this thread.
    // Custom folder scans run on worker threads, so initialize it here rather
    // than relying on the caller.
    let _com = ComApartment::init().ok();

    let wide: Vec<u16> = path.encode_utf16().chain(Some(0)).collect();
    // LOWORD is the large icon size, HIWORD the small one.
    let edge = ICON_REQUEST_EDGE & 0xFFFF;
    let requested = edge | (edge << 16);
    let mut large = HICON::default();
    let mut small = HICON::default();

    unsafe {
        // Any non-zero result means "no icon here": S_FALSE for a file without
        // an icon resource, a failure HRESULT for a path that is not a file at
        // all. Both must fall through to the next source.
        let result = SHDefExtractIconW(
            windows::core::PCWSTR(wide.as_ptr()),
            index,
            0,
            Some(&mut large),
            Some(&mut small),
            requested,
        );
        if result.0 != 0 || large.is_invalid() {
            return None;
        }
        let image = icon_to_image(large);
        let _ = DestroyIcon(large);
        if !small.is_invalid() {
            let _ = DestroyIcon(small);
        }
        image
    }
}
