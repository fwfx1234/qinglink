//! 图标载荷规范化：裁剪到画面、缩放到 128×128 方形画布，再编码为 PNG base64。

use base64::Engine as _;

// ============================================================================
// Payload normalisation
//
// Everything that leaves this module — whatever source it came from, and whether
// it arrived as an `HICON` or as a bitmap file — is reshaped into one canonical
// form: a square `ICON_EDGE` canvas whose artwork is centred and touches both
// edges on its longest axis. That is what makes a launcher icon look the same
// size as its neighbours in a `contain` / `object-fit: contain` box.
// ============================================================================

/// Longest edge of every stored icon payload.
pub const ICON_EDGE: u32 = 128;

/// Edge requested from the shell. Larger than [`ICON_EDGE`] so the downscale
/// keeps the artwork crisp.
pub(super) const ICON_REQUEST_EDGE: u32 = 256;

/// Alpha above which a pixel counts as visible.
const ALPHA_THRESHOLD: u8 = 8;

/// Resampling filter used for both crops-to-size and the canonical resize.
const ICON_FILTER: image::imageops::FilterType = image::imageops::FilterType::Lanczos3;

/// Encode a raw artwork as a canonical PNG base64 payload.
///
/// The payload budget is enforced by re-encoding at a smaller edge rather than
/// by dropping the icon: a missing icon is far more visible to the user than a
/// slightly softer one.
pub(super) fn encode_icon_payload(raw: &image::RgbaImage) -> Option<String> {
    for edge in [ICON_EDGE, 96, 72, 64] {
        let normalized = normalize_icon(raw, edge)?;
        let mut buffer = Vec::new();
        let mut cursor = std::io::Cursor::new(&mut buffer);
        if image::DynamicImage::ImageRgba8(normalized)
            .write_to(&mut cursor, image::ImageFormat::Png)
            .is_err()
        {
            continue;
        }
        let encoded = base64::engine::general_purpose::STANDARD.encode(&buffer);
        if qinglink_protocol::service_ipc::icon_fits_budget(&encoded) {
            return Some(encoded);
        }
    }
    None
}

/// Crop to the artwork, scale its longest edge to `edge`, and centre it on a
/// square transparent canvas. `None` when there is no artwork to crop to.
fn normalize_icon(raw: &image::RgbaImage, edge: u32) -> Option<image::RgbaImage> {
    let (x, y, width, height) = ink_bounds(raw)?;
    let cropped = image::imageops::crop_imm(raw, x, y, width, height).to_image();
    let longest = width.max(height);
    let scaled = if longest == edge {
        cropped
    } else {
        let scale = edge as f32 / longest as f32;
        let target_width = ((width as f32 * scale).round() as u32).clamp(1, edge);
        let target_height = ((height as f32 * scale).round() as u32).clamp(1, edge);
        image::imageops::resize(&cropped, target_width, target_height, ICON_FILTER)
    };
    let mut canvas = image::RgbaImage::new(edge, edge);
    let offset_x = (edge.saturating_sub(scaled.width()) / 2) as i64;
    let offset_y = (edge.saturating_sub(scaled.height()) / 2) as i64;
    image::imageops::overlay(&mut canvas, &scaled, offset_x, offset_y);
    Some(canvas)
}

/// Bounding box of the artwork inside `image`, as `(x, y, width, height)`.
///
/// Icons whose colour bitmap kept its transparency are bounded by alpha. Icons
/// that lost it (`SHDefExtractIconW` on a mask-only resource returns an all-zero
/// alpha channel) are bounded by non-zero colour instead, so a shape is never
/// mistaken for an empty bitmap.
pub(super) fn ink_bounds(image: &image::RgbaImage) -> Option<(u32, u32, u32, u32)> {
    let (width, height) = (image.width(), image.height());
    let mut opaque = (width, height, 0u32, 0u32, false);
    let mut painted = (width, height, 0u32, 0u32, false);

    for (x, y, pixel) in image.enumerate_pixels() {
        let [red, green, blue, alpha] = pixel.0;
        if alpha > ALPHA_THRESHOLD {
            opaque.0 = opaque.0.min(x);
            opaque.1 = opaque.1.min(y);
            opaque.2 = opaque.2.max(x);
            opaque.3 = opaque.3.max(y);
            opaque.4 = true;
        } else if red != 0 || green != 0 || blue != 0 {
            painted.0 = painted.0.min(x);
            painted.1 = painted.1.min(y);
            painted.2 = painted.2.max(x);
            painted.3 = painted.3.max(y);
            painted.4 = true;
        }
    }

    let bounds = if opaque.4 { opaque } else { painted };
    bounds.4.then(|| {
        (
            bounds.0,
            bounds.1,
            bounds.2 - bounds.0 + 1,
            bounds.3 - bounds.1 + 1,
        )
    })
}

#[cfg(test)]
mod normalize_icon_tests {
    use base64::Engine;

    use super::{ICON_EDGE, encode_icon_payload, ink_bounds, normalize_icon};

    /// A transparent canvas with one opaque block painted on it.
    fn artwork(canvas: (u32, u32), block: (u32, u32, u32, u32)) -> image::RgbaImage {
        let (block_x, block_y, block_width, block_height) = block;
        let mut image = image::RgbaImage::new(canvas.0, canvas.1);
        for y in block_y..(block_y + block_height).min(canvas.1) {
            for x in block_x..(block_x + block_width).min(canvas.0) {
                image.put_pixel(x, y, image::Rgba([200, 40, 10, 255]));
            }
        }
        image
    }

    /// The reported defect: the shell hands back a big canvas with a small
    /// artwork pasted into the top-left corner, and the old code kept the canvas.
    #[test]
    fn corner_artwork_is_cropped_and_scaled_to_fill_the_canvas() {
        let raw = artwork((128, 128), (0, 0, 24, 24));
        let normalized = normalize_icon(&raw, ICON_EDGE).expect("artwork is present");

        assert_eq!(
            (normalized.width(), normalized.height()),
            (ICON_EDGE, ICON_EDGE)
        );
        assert_eq!(
            ink_bounds(&normalized),
            Some((0, 0, ICON_EDGE, ICON_EDGE)),
            "the artwork must fill the payload edge to edge"
        );
    }

    /// A wide logo must not be stretched: it is centred on a square canvas.
    #[test]
    fn non_square_artwork_keeps_its_shape_and_is_centred() {
        let raw = artwork((160, 60), (0, 0, 160, 60));
        let normalized = normalize_icon(&raw, ICON_EDGE).expect("artwork is present");

        assert_eq!(
            (normalized.width(), normalized.height()),
            (ICON_EDGE, ICON_EDGE)
        );
        // 160x60 scaled to a 128px longest edge becomes 128x48.
        assert_eq!(ink_bounds(&normalized), Some((0, 40, ICON_EDGE, 48)));
    }

    #[test]
    fn fully_transparent_artwork_is_rejected() {
        let raw = image::RgbaImage::new(64, 64);
        assert!(ink_bounds(&raw).is_none());
        assert!(normalize_icon(&raw, ICON_EDGE).is_none());
        assert!(encode_icon_payload(&raw).is_none());
    }

    /// `SHDefExtractIconW` on a mask-only resource returns an all-zero alpha
    /// channel, so the shape has to be found in the colour bytes.
    #[test]
    fn colour_only_artwork_is_detected_when_alpha_is_empty() {
        let mut raw = image::RgbaImage::new(64, 64);
        for y in 10..30 {
            for x in 10..30 {
                raw.put_pixel(x, y, image::Rgba([10, 20, 30, 0]));
            }
        }
        assert_eq!(ink_bounds(&raw), Some((10, 10, 20, 20)));
    }

    #[test]
    fn payload_is_square_and_within_the_protocol_budget() {
        let raw = artwork((128, 128), (5, 7, 80, 20));
        let encoded = encode_icon_payload(&raw).expect("payload");

        assert!(qinglink_protocol::service_ipc::icon_fits_budget(&encoded));
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(&encoded)
            .expect("base64");
        let decoded = image::load_from_memory_with_format(&bytes, image::ImageFormat::Png)
            .expect("png")
            .to_rgba8();
        assert_eq!((decoded.width(), decoded.height()), (ICON_EDGE, ICON_EDGE));
        // The 80x20 block becomes 128x32 centred in the canvas.
        assert_eq!(ink_bounds(&decoded), Some((0, 48, ICON_EDGE, 32)));
    }
}
