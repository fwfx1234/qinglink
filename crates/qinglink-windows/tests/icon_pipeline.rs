//! 图标负载契约的端到端检查。
//!
//! 每个存下来的图标都应当是同一种形状：一块正方形画布，图案居中且沿最长轴触及
//! 两侧边缘。这些测试锁定的缺陷是：shell 的图像列表返回的是一块大画布、里面把
//! 小图案贴在左上角，而旧的提取器保留了整块画布——于是大多数图标只渲染出几个
//! 像素宽。
#![cfg(target_os = "windows")]
#![allow(clippy::unwrap_used)]

use base64::Engine;
use qinglink_protocol::service_ipc::icon_fits_budget;
use qinglink_windows::platform_windows::{
    extract_icon_base64, image_file_to_icon_base64, resolve_icon,
};

/// Longest edge every payload is normalised to.
const ICON_EDGE: u32 = 128;

/// Bounding box of the visible artwork, mirroring the extractor's own rule:
/// alpha first, colour when the alpha channel is empty.
fn ink_bounds(image: &image::RgbaImage) -> Option<(u32, u32, u32, u32)> {
    let (width, height) = (image.width(), image.height());
    let mut opaque = (width, height, 0u32, 0u32, false);
    let mut painted = (width, height, 0u32, 0u32, false);

    for (x, y, pixel) in image.enumerate_pixels() {
        let [red, green, blue, alpha] = pixel.0;
        if alpha > 8 {
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

fn decode(encoded: &str) -> image::RgbaImage {
    assert!(
        icon_fits_budget(encoded),
        "payload of {} base64 chars exceeds the protocol budget",
        encoded.len()
    );
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .expect("payload is valid base64");
    image::load_from_memory_with_format(&bytes, image::ImageFormat::Png)
        .expect("payload is a PNG")
        .to_rgba8()
}

fn system32(name: &str) -> String {
    let root = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".to_string());
    format!(r"{root}\System32\{name}")
}

#[test]
fn resource_icons_are_square_and_filled() {
    // Components that ship icons on every Windows install. At least one must
    // resolve, and every one that does must be canonical.
    let candidates = [
        system32(r"shell32.dll"),
        system32(r"explorer.exe"),
        system32(r"notepad.exe"),
    ];

    let mut resolved = 0;
    for path in &candidates {
        let Some(encoded) = extract_icon_base64(path) else {
            continue;
        };
        resolved += 1;
        let image = decode(&encoded);
        assert_eq!(
            (image.width(), image.height()),
            (ICON_EDGE, ICON_EDGE),
            "{path}: payload must be a {ICON_EDGE}px square"
        );
        let (_, _, width, height) =
            ink_bounds(&image).unwrap_or_else(|| panic!("{path}: payload has no visible artwork"));
        assert_eq!(
            width.max(height),
            ICON_EDGE,
            "{path}: artwork must fill the canvas, got {width}x{height}"
        );
    }
    assert!(resolved > 0, "no system icon could be extracted at all");
}

#[test]
fn image_file_artwork_is_square_and_centred() {
    // A wide strip stands in for the banner/header artwork launchers ship.
    const SOURCE: (u32, u32) = (200, 40);
    let source = image::RgbaImage::from_fn(SOURCE.0, SOURCE.1, |x, y| {
        image::Rgba([(x % 251) as u8, (y % 241) as u8, 32, 255])
    });
    let path = std::env::temp_dir().join("qinglink-icon-pipeline-test.png");
    source.save(&path).expect("cannot write the fixture");

    let encoded = image_file_to_icon_base64(&path.to_string_lossy()).expect("fixture decodes");
    let _ = std::fs::remove_file(&path);

    let image = decode(&encoded);
    assert_eq!((image.width(), image.height()), (ICON_EDGE, ICON_EDGE));
    let (_, top, width, height) = ink_bounds(&image).expect("fixture has artwork");
    assert_eq!(width, ICON_EDGE, "longest edge must fill the canvas");

    let expected_height =
        ((SOURCE.1 as f32 * (ICON_EDGE as f32 / SOURCE.0 as f32)).round() as u32).max(1);
    assert!(
        height.abs_diff(expected_height) <= 1,
        "aspect ratio must be preserved: expected {expected_height}, got {height}"
    );
    assert!(
        top.abs_diff((ICON_EDGE - height) / 2) <= 1,
        "artwork must be centred: got top {top}"
    );
}

/// A catalog entry that points at a directory must not advertise the shell's
/// "document" icon as if it were the program's own (Steam games whose main
/// executable was never found store the install directory).
#[test]
fn a_directory_resolves_to_no_icon() {
    let directory = std::env::temp_dir().join("qinglink-icon-pipeline-empty-dir");
    std::fs::create_dir_all(&directory).expect("cannot create the fixture directory");

    let resolution = resolve_icon(&directory.to_string_lossy(), None);

    let _ = std::fs::remove_dir_all(&directory);
    assert_eq!(resolution.source, "none");
    assert!(resolution.icon_base64.is_none());
}
