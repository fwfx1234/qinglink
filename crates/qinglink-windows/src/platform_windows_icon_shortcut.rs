//! 快捷方式与路径定位：解析 `.lnk` 的目标与图标位置、判断目标是否仍存在，
//! 并发现随程序附带、可当作图标的图片。

use super::ComApartment;
use super::read_wide_string;
use super::resolve_lnk_target;

pub(super) fn is_executable(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("exe") || extension.eq_ignore_ascii_case("dll")
        })
}

pub(super) fn is_shortcut(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("lnk"))
}

/// Expand `%VAR%` references the way the shell does when it reads a shortcut's
/// icon location. Unknown variables are left untouched instead of dropped.
fn expand_environment_variables(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    let mut rest = value;
    while let Some(start) = rest.find('%') {
        result.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        match after.find('%') {
            Some(end) if end > 0 => {
                let name = &after[..end];
                match std::env::var(name) {
                    Ok(replacement) => result.push_str(&replacement),
                    Err(_) => {
                        result.push('%');
                        result.push_str(name);
                        result.push('%');
                    }
                }
                rest = &after[end + 1..];
            }
            _ => {
                result.push('%');
                rest = after;
            }
        }
    }
    result.push_str(rest);
    result
}

/// Artwork files shipped next to an executable, in decreasing preference.
///
/// Generic names are only searched in the executable's own directory: looking
/// for `logo.png` in the parent would pick up an unrelated product's artwork.
/// Name-derived files are also searched one level up, because portable builds
/// often place `Game.exe` beside a `Media/Game.ico`.
pub(super) fn sibling_icon_paths(anchor: &std::path::Path) -> Vec<std::path::PathBuf> {
    const GENERIC_NAMES: &[&str] = &[
        "icon.png",
        "icon.ico",
        "icon.jpg",
        "icon.jpeg",
        "logo.png",
        "logo.ico",
        "app.png",
        "app.ico",
        "cover.png",
        "header.png",
        "banner.png",
    ];
    const EXTENSIONS: &[&str] = &["png", "ico", "jpg", "jpeg", "bmp"];

    let mut candidates = Vec::new();
    let Some(directory) = anchor.parent() else {
        return candidates;
    };
    for name in GENERIC_NAMES {
        candidates.push(directory.join(name));
    }
    if let Some(stem) = anchor.file_stem().and_then(|stem| stem.to_str())
        && !stem.is_empty()
    {
        for extension in EXTENSIONS {
            candidates.push(directory.join(format!("{stem}.{extension}")));
            if let Some(parent) = directory.parent() {
                candidates.push(parent.join(format!("{stem}.{extension}")));
            }
        }
    }
    candidates
}

/// Read the icon location a `.lnk` shortcut points at, with its resource index.
///
/// The index matters: plenty of shortcuts point at `some.dll,3`, and ignoring it
/// silently returns the wrong artwork.
pub(super) fn resolve_lnk_icon_location(path: &std::path::Path) -> Option<(String, i32)> {
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::System::Com::{
        CLSCTX_INPROC_SERVER, CoCreateInstance, IPersistFile, STGM_READ,
    };
    use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};
    use windows::core::{Interface, PCWSTR};

    let _com = ComApartment::init().ok();
    unsafe {
        let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).ok()?;
        let persist: IPersistFile = link.cast().ok()?;
        let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        persist.Load(PCWSTR(wide.as_ptr()), STGM_READ).ok()?;

        let mut icon_buffer = vec![0u16; 32768];
        let mut icon_index = 0i32;
        link.GetIconLocation(&mut icon_buffer, &mut icon_index)
            .ok()?;
        let location = expand_environment_variables(&read_wide_string(&icon_buffer)?);
        std::path::Path::new(&location)
            .is_file()
            .then_some((location, icon_index))
    }
}

/// Whether a catalog path can still be launched.
///
/// For shortcuts this resolves the target, so Start Menu entries whose program
/// was uninstalled are dropped instead of being offered to the phone.
pub fn shortcut_target_exists(path: &str) -> bool {
    let path = std::path::Path::new(path);
    if !is_shortcut(path) {
        return path.exists();
    }
    let _com = ComApartment::init().ok();
    let (target, _arguments) = resolve_lnk_target(path);
    std::path::Path::new(&target).exists()
}
