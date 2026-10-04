//! 图片文件（png / jpg / bmp / ico）解码为规范化的图标载荷。

use super::icon_normalize::encode_icon_payload;
use super::icon_resource::extract_resource_icon_base64;

/// Decode a bitmap file (png / jpg / bmp / ico) into a canonical PNG payload.
pub fn image_file_to_icon_base64(path: &str) -> Option<String> {
    let file = std::path::Path::new(path);
    if !file.is_file() {
        return None;
    }
    // A multi-megabyte texture is never a usable launcher icon, and decoding it
    // would cost more than the icon is worth.
    let size = std::fs::metadata(file).ok()?.len();
    if size == 0 || size > 16 * 1024 * 1024 {
        return None;
    }

    // An `.ico` is a container of icon resources, and many of them only hold
    // 1/4/8/24bpp entries that the `image` crate refuses to decode (it needs an
    // RGBA8 entry) — it also ranks entries by bit depth before size, so a
    // 16x16@32bpp entry can beat the 256x256 artwork next to it. Letting the
    // shell pick and scale the entry sidesteps both, so try it first and only
    // fall back to the decoder for the rare file the shell declines.
    if is_icon_file(file)
        && let Some(icon) = extract_resource_icon_base64(path, 0)
    {
        return Some(icon);
    }

    let decoded = image::open(file).ok()?;
    encode_icon_payload(&decoded.to_rgba8())
}

fn is_icon_file(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("ico"))
}

/// A bitmap file we decode ourselves, as opposed to an icon container the shell
/// can pick an entry out of (`.ico`, `.exe`, `.dll`).
pub(super) fn is_image_file(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "png" | "jpg" | "jpeg" | "bmp"
            )
        })
}
