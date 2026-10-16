//! 图标解析链：按“显式指定 → 快捷方式 → 内嵌资源 → 邻近图片 → 外壳默认”
//! 依次尝试，并报告胜出的来源。

use super::ComApartment;
use super::icon_image_file::{image_file_to_icon_base64, is_image_file};
use super::icon_resource::{extract_icon_base64, extract_resource_icon_base64};
use super::icon_shell::extract_shell_icon_base64;
use super::icon_shortcut::{
    is_executable, is_shortcut, resolve_lnk_icon_location, sibling_icon_paths,
};
use super::resolve_lnk_target;

// ============================================================================
// Icon resolution
//
// Windows applications keep their artwork in four different places, and a
// single "ask the shell for the file icon" call only covers one of them:
//
//   1. a file the user picked explicitly;
//   2. the icon a `.lnk` shortcut points at (`IShellLinkW::GetIconLocation`),
//      which is how most installers ship a custom icon;
//   3. the shortcut target's embedded `RT_GROUP_ICON` resource;
//   4. an image file shipped next to the executable (`icon.png`, `<name>.ico`,
//      `logo.png`, …), which is the only artwork Electron, Unity and many
//      launchers provide.
//
// The chain below walks them in that order and reports which one won, so the
// clients can both show the result and offer the remaining choices.
// ============================================================================

/// Icon came from a file the user (or the scanner) chose.
pub const ICON_SOURCE_MANUAL: &str = "manual";
/// Icon came from the executable through a `.lnk` shortcut.
pub const ICON_SOURCE_SHORTCUT: &str = "shortcut";
/// Icon was embedded in the executable's resources.
pub const ICON_SOURCE_EXECUTABLE: &str = "executable";
/// Icon came from an artwork file shipped next to the executable.
pub const ICON_SOURCE_IMAGE: &str = "image";
/// Icon is the shell's generic icon for the path — the weakest result.
pub const ICON_SOURCE_SHELL: &str = "shell";
/// Nothing produced an icon.
pub const ICON_SOURCE_NONE: &str = "none";

/// One pickable icon source for a catalog entry.
#[derive(Clone, Debug, serde::Serialize)]
pub struct IconCandidateInfo {
    /// Absolute path of the file the icon would be read from.
    pub path: String,
    /// `executable` | `shortcut` | `shortcut_icon` | `image`.
    pub kind: String,
    /// 图标来源文件的大小（字节）。
    pub size_bytes: u64,
}

/// Outcome of resolving one entry's icon.
#[derive(Clone, Debug)]
pub struct IconResolution {
    /// PNG payload, base64 encoded. `None` when no source produced an icon.
    pub icon_base64: Option<String>,
    /// One of the `ICON_SOURCE_*` labels, persisted with the payload.
    pub source: String,
    /// File the payload was read from, kept so the UI can explain the result.
    pub source_path: Option<String>,
}

impl IconResolution {
    fn missing() -> Self {
        Self {
            icon_base64: None,
            source: ICON_SOURCE_NONE.to_string(),
            source_path: None,
        }
    }

    fn found(icon_base64: String, source: &str, path: &str) -> Self {
        Self {
            icon_base64: Some(icon_base64),
            source: source.to_string(),
            source_path: Some(path.to_string()),
        }
    }
}

/// Read a usable icon out of any file kind we understand.
///
/// The fallback matters for the odd explicit choice (a `.lnk`, a file with no
/// extension): those carry no icon resource of their own, so the shell has to be
/// asked for whatever it draws for that path.
fn icon_from_file(path: &str) -> Option<String> {
    let extension = std::path::Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    match extension.as_str() {
        "png" | "jpg" | "jpeg" | "bmp" | "ico" => image_file_to_icon_base64(path),
        _ => extract_icon_base64(path).or_else(|| extract_shell_icon_base64(path)),
    }
}

/// Resolve the best icon for one catalog entry.
///
/// `explicit` is the file the user picked (or the artwork the scanner recorded
/// for a Steam game); everything else is discovered automatically. Returns the
/// winning source so the caller can persist and explain it.
pub fn resolve_icon(path: &str, explicit: Option<&str>) -> IconResolution {
    if let Some(explicit) = explicit.map(str::trim).filter(|value| !value.is_empty())
        && let Some(icon) = icon_from_file(explicit)
    {
        return IconResolution::found(icon, ICON_SOURCE_MANUAL, explicit);
    }

    let original = std::path::Path::new(path);
    let mut anchor = original.to_path_buf();
    if is_shortcut(original) {
        let _com = ComApartment::init().ok();
        if let Some((location, index)) = resolve_lnk_icon_location(original) {
            // The resource index only means something for an icon container
            // (`.exe` / `.dll` / `.ico`); a bitmap next to the shortcut is
            // decoded directly.
            let icon = if is_image_file(std::path::Path::new(&location)) {
                image_file_to_icon_base64(&location)
            } else {
                extract_resource_icon_base64(&location, index)
                    .or_else(|| image_file_to_icon_base64(&location))
            };
            if let Some(icon) = icon {
                return IconResolution::found(icon, ICON_SOURCE_SHORTCUT, &location);
            }
        }
        let (target, _arguments) = resolve_lnk_target(original);
        let target_path = std::path::PathBuf::from(&target);
        if target_path.is_file() {
            anchor = target_path;
        }
    }

    if is_executable(&anchor)
        && let Some(icon) = extract_icon_base64(&anchor.to_string_lossy())
    {
        let source = if is_shortcut(original) {
            ICON_SOURCE_SHORTCUT
        } else {
            ICON_SOURCE_EXECUTABLE
        };
        return IconResolution::found(icon, source, &anchor.to_string_lossy());
    }

    // Artwork next to the executable, then next to the shortcut itself.
    for candidate in sibling_icon_paths(&anchor)
        .into_iter()
        .chain(sibling_icon_paths(original))
    {
        if let Some(icon) = image_file_to_icon_base64(&candidate.to_string_lossy()) {
            return IconResolution::found(icon, ICON_SOURCE_IMAGE, &candidate.to_string_lossy());
        }
    }

    // Last resort: whatever the shell draws for this path. Directories are
    // refused outright — a catalog entry that points at one (a Steam game whose
    // main executable was never found) would otherwise advertise the shell's
    // "document" icon as if it were the program's own.
    if original.is_file()
        && let Some(icon) = extract_shell_icon_base64(path)
    {
        return IconResolution::found(icon, ICON_SOURCE_SHELL, path);
    }
    IconResolution::missing()
}

/// Icon for one running process, resolved from its executable only.
///
/// The difference from [`resolve_icon`] is that this never asks the shell for
/// its default icon: every process without artwork of its own would otherwise
/// be served the same generic glyph, which is noise in a process list of a few
/// hundred rows. When nothing is found the caller keeps its own placeholder.
pub fn resolve_process_icon(path: &str) -> IconResolution {
    let anchor = std::path::Path::new(path);
    if anchor.is_file()
        && is_executable(anchor)
        && let Some(icon) = extract_icon_base64(path)
    {
        return IconResolution::found(icon, ICON_SOURCE_EXECUTABLE, path);
    }

    // Artwork shipped next to the executable, the same source the launcher
    // catalog uses for portable builds.
    for candidate in sibling_icon_paths(anchor) {
        if let Some(icon) = image_file_to_icon_base64(&candidate.to_string_lossy()) {
            return IconResolution::found(icon, ICON_SOURCE_IMAGE, &candidate.to_string_lossy());
        }
    }
    IconResolution::missing()
}
