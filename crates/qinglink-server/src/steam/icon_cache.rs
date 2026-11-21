//! Steam 封面图在 `librarycache` 里的查找。
//!
//! Steam 这几年换过三次缓存排布，三种都要认；只认第一种时新版客户端下
//! `icon_path` 恒为 `None`，Steam 条目就只能退回"猜一个 exe 再取它的图标"，
//! 于是拿到崩溃上报器的图，或者干脆没有图标。
//!
//! | 排布 | 路径 |
//! | --- | --- |
//! | 旧版平铺 | `<cache>/<appid>_icon.jpg`、`<cache>/<appid>_header.jpg` |
//! | 按 appid 分目录 | `<cache>/<appid>/header.jpg`、`library_600x900.jpg`、`logo.png` |
//! | 内容哈希 | `<cache>/<appid>/<sha1>/header.jpg`，方形图标是 `<cache>/<appid>/<sha1>.jpg` |
//!
//! 这里只看文件名与体积，不解码图片：解码由图标管线统一做。

use std::path::{Path, PathBuf};

/// 具名封面图的优先级，越靠前越适合当图标。
///
/// 方形游戏图标不在这张表里（见 [`is_hashed_icon`]）：它比任何封面都更像"图标"，
/// 所以先单独找。这里排的是找不到方形图标时的退路：横版头图信息量最大，
/// 其次是竖版胶囊，`logo` 通常是大片透明留白，放最后。
const ART_PRIORITY: [&str; 5] = [
    "header",
    "library_header",
    "library_capsule",
    "library_600x900",
    "logo",
];

/// 新版缓存的方形图标体积上限。
///
/// 游戏图标只有几十 KB；超过这个尺寸的哈希文件是封面而不是图标，拿它当图标会被
/// 图标管线裁成一团。
const HASHED_ICON_MAX_BYTES: u64 = 64 * 1024;

/// 一个 appid 在某个库里能用的封面图路径。
pub(crate) fn find(steam_root: Option<&Path>, library_path: &Path, app_id: u32) -> Option<String> {
    let app_dir_name = app_id.to_string();
    for cache in cache_roots(steam_root, library_path) {
        if !cache.is_dir() {
            continue;
        }
        if let Some(path) = in_app_dir(&cache.join(&app_dir_name)) {
            return Some(path);
        }
        for name in legacy_names(app_id) {
            let path = cache.join(name);
            if path.is_file() {
                return Some(path.to_string_lossy().to_string());
            }
        }
    }
    None
}

/// 所有可能存放 `librarycache` 的目录，去重后按固定顺序尝试。
fn cache_roots(steam_root: Option<&Path>, library_path: &Path) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(root) = steam_root {
        roots.push(root.join("appcache").join("librarycache"));
    }
    roots.push(library_path.join("appcache").join("librarycache"));
    roots.push(
        library_path
            .join("steamapps")
            .join("appcache")
            .join("librarycache"),
    );
    roots.sort();
    roots.dedup();
    roots
}

/// 旧版把图标平铺在 `librarycache` 根目录，文件名带 appid。
fn legacy_names(app_id: u32) -> [String; 4] {
    [
        format!("{app_id}_icon.jpg"),
        format!("{app_id}_icon.png"),
        format!("{app_id}_header.jpg"),
        format!("{app_id}.jpg"),
    ]
}

/// `<cache>/<appid>/` 里的封面图：方形图标优先，其次具名资源。
fn in_app_dir(app_dir: &Path) -> Option<String> {
    if !app_dir.is_dir() {
        return None;
    }
    let files = art_files(app_dir);
    if let Some(path) = files.iter().find(|path| is_hashed_icon(path)) {
        return Some(path.to_string_lossy().to_string());
    }

    let mut ranked: Vec<((usize, usize), &PathBuf)> = files
        .iter()
        .filter_map(|path| {
            let stem = path.file_stem()?.to_str()?;
            Some((art_rank(stem)?, path))
        })
        .collect();
    ranked.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(right.1)));
    ranked
        .first()
        .map(|(_, path)| path.to_string_lossy().to_string())
}

/// appid 目录下的图片，外加一层哈希子目录里的。
///
/// 新版把 `header.jpg` 这类资源放进 `<sha1>/`，而方形图标仍留在 appid 目录本身。
fn art_files(app_dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_images(app_dir, &mut files);

    let mut subdirs: Vec<PathBuf> = std::fs::read_dir(app_dir)
        .map(|entries| {
            entries
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| path.is_dir())
                .collect()
        })
        .unwrap_or_default();
    subdirs.sort();
    for subdir in subdirs.iter().take(32) {
        collect_images(subdir, &mut files);
    }

    files.sort();
    files
}

fn collect_images(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() && is_image(&path) {
            out.push(path);
        }
    }
}

fn is_image(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "jpg" | "jpeg" | "png"
            )
        })
}

/// 新版缓存里的方形游戏图标：`<40 位十六进制>.jpg`，体积只有几十 KB。
fn is_hashed_icon(path: &Path) -> bool {
    let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
        return false;
    };
    if stem.len() != 40 || !stem.chars().all(|character| character.is_ascii_hexdigit()) {
        return false;
    }
    std::fs::metadata(path).is_ok_and(|meta| meta.len() <= HASHED_ICON_MAX_BYTES)
}

/// 具名资源的排序键：`(优先级, 语言)`。
fn art_rank(stem: &str) -> Option<(usize, usize)> {
    for (index, prefix) in ART_PRIORITY.iter().enumerate() {
        if stem.eq_ignore_ascii_case(prefix) {
            return Some((index, 0));
        }
        if let Some(rest) = stem.strip_prefix(prefix)
            && let Some(language) = rest.strip_prefix('_')
        {
            return Some((index, language_rank(&language.to_ascii_lowercase())));
        }
    }
    None
}

/// 语言变体的次序：中文 > 英文原图 > 其它。界面是中文，封面也应优先中文。
fn language_rank(language: &str) -> usize {
    match language {
        "schinese" => 1,
        "english" => 2,
        _ => 3,
    }
}
