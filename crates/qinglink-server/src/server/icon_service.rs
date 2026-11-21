//! 手机协议与桌面管理 IPC 共用的单条目图标解析。
//!
//! Windows 程序把图标资源放在好几个不同的地方，目录无从知道哪个才对。本模块按需
//! 解析单个条目，记录图标**来自哪里**，并持久化结果，使下一次列表（以及这台 PC 的
//! 其它所有客户端）看到同一个图标。

use crate::protocol::responses::{IconCandidateSummary, IconUpdateResponse};
use crate::server::{AppState, reload_visible_catalog};
use crate::service::app_db::DbAppEntry;
use qinglink_protocol::service_ipc::icon_fits_budget;

/// Resolve and persist one entry's icon.
///
/// `explicit` is the file the user picked:
/// - `Some(path)` selects that file and records it as the entry's preference;
/// - `None` deliberately *ignores and clears* any stored preference and re-runs
///   automatic discovery, which is what "重新扫描图标" means to a user.
///
/// # Errors
/// 条目 id 为空、目录数据库不可用、目录中不存在该条目（由 `find_entry` 判定），
/// `explicit` 指向的文件不存在，或写入图标失败时返回 `Err`。
pub fn resolve_and_store(
    state: &AppState,
    id: &str,
    explicit: Option<&str>,
) -> Result<IconUpdateResponse, String> {
    let entry = find_entry(state, id)?;

    let requested = explicit
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);

    // A bad path must not silently fall back to the automatic chain while the
    // UI claims the user's file was applied.
    if let Some(path) = requested.as_deref()
        && !std::path::Path::new(path).is_file()
    {
        return Err(format!("图标文件不存在：{path}"));
    }

    // Automatic mode: a Steam entry's own cover art is the best answer and the
    // only one the executable cannot provide, so try it before the exe.
    if requested.is_none()
        && let Some((icon_base64, artwork)) = steam_cover(state, &entry)
    {
        // Artwork discovered by the scanner is not a user preference, so it is
        // not written to `icon_path` — only an explicit choice is.
        persist(state, &entry.id, Some(icon_base64.as_str()), "steam", None)?;
        return Ok(IconUpdateResponse {
            id: entry.id,
            icon_base64: Some(icon_base64),
            source: "steam".to_string(),
            icon_path: Some(artwork),
        });
    }

    let resolution = crate::platform::resolve_icon(&entry.exe_path, requested.as_deref());
    // `manual` is only reported when the user's own file produced the bitmap;
    // otherwise the automatic chain won and its source is the honest answer.
    let explicit_won = requested.is_some() && resolution.source == "manual";
    let stored_path = explicit_won.then(|| requested.clone()).flatten();
    let source = if explicit_won {
        "manual".to_string()
    } else {
        resolution.source.clone()
    };

    let payload = resolution.icon_base64.filter(|icon| icon_fits_budget(icon));
    persist(
        state,
        &entry.id,
        payload.as_deref(),
        &source,
        stored_path.as_deref(),
    )?;

    Ok(IconUpdateResponse {
        id: entry.id,
        icon_base64: payload,
        source,
        icon_path: resolution.source_path.or(stored_path),
    })
}

/// Icon files a user can choose from for one entry.
///
/// # Errors
/// 条目 id 为空、目录数据库不可用，或目录中不存在该条目时返回 `Err`。
pub fn candidates(
    state: &AppState,
    id: &str,
) -> Result<(String, Vec<IconCandidateSummary>), String> {
    let entry = find_entry(state, id)?;
    let candidates = crate::platform::list_icon_candidates(&entry.exe_path)
        .into_iter()
        .map(|candidate| IconCandidateSummary {
            path: candidate.path,
            kind: candidate.kind,
            size_bytes: candidate.size_bytes,
        })
        .collect();
    Ok((entry.id, candidates))
}

/// Preview one icon file the user is browsing for.
///
/// Read-only: nothing is persisted and no entry is touched. Only formats the
/// icon pipeline can actually decode are accepted, so a preview never claims
/// that an arbitrary file would work as an icon.
pub fn preview_file(path: &str) -> Option<String> {
    let file = std::path::Path::new(path);
    if !file.is_file() {
        return None;
    }
    let size = std::fs::metadata(file).ok()?.len();
    if size == 0 || size > 16 * 1024 * 1024 {
        return None;
    }
    let extension = file.extension()?.to_str()?.to_ascii_lowercase();
    match extension.as_str() {
        // 位图与图标容器：走与"手动指定图标"相同的解码链。
        "png" | "jpg" | "jpeg" | "bmp" | "ico" => crate::platform::image_file_to_icon_base64(path),
        // 程序与快捷方式：预览它们的图标资源，便于用户在候选列表里辨认。
        "exe" | "lnk" => crate::platform::resolve_icon(path, Some(path)).icon_base64,
        _ => None,
    }
}

fn persist(
    state: &AppState,
    id: &str,
    icon_base64: Option<&str>,
    source: &str,
    icon_path: Option<&str>,
) -> Result<(), String> {
    {
        let database = state.app_db.lock().map_err(|_| "目录不可用".to_string())?;
        database.set_icon(id, icon_base64, Some(source), icon_path)?;
    }
    reload_visible_catalog(state);
    Ok(())
}

/// Cover art Steam already downloaded for one installed game.
fn steam_artwork(state: &AppState, id: &str) -> Option<String> {
    let app_id: u32 = id.strip_prefix("steam:")?.parse().ok()?;
    let database = state.app_db.lock().ok()?;
    database
        .get_steam_games()
        .ok()?
        .into_iter()
        .find(|game| game.app_id == app_id)?
        .icon_path
}

/// The Steam cover art recorded for an entry, already encoded as a payload.
fn steam_cover(state: &AppState, entry: &DbAppEntry) -> Option<(String, String)> {
    if !entry.source.eq_ignore_ascii_case("steam") {
        return None;
    }
    let artwork = steam_artwork(state, &entry.id)?;
    let icon = crate::platform::image_file_to_icon_base64(&artwork)?;
    Some((icon, artwork))
}

/// Resolve one entry through the automatic chain and report the provenance,
/// without touching the database.
///
/// The one-shot pipeline upgrade in [`crate::server`] needs this for hundreds of
/// entries at once; it must not go through [`resolve_and_store`], which reloads
/// the whole visible catalog on every call.
pub fn resolve_automatic(
    state: &AppState,
    entry: &DbAppEntry,
) -> (Option<String>, String, Option<String>) {
    if let Some((icon, artwork)) = steam_cover(state, entry) {
        return (Some(icon), "steam".to_string(), Some(artwork));
    }
    let resolution = crate::platform::resolve_icon(&entry.exe_path, None);
    (
        resolution.icon_base64,
        resolution.source,
        resolution.source_path,
    )
}

/// 按需读取图标时使用：解析一个条目并把结果记进数据库与内存目录。
///
/// 列表接口不携带图标，客户端逐行请求。如果每次请求都重新读文件资源，同一条目
/// 会在"有图标 / 没有图标"之间摇摆（某次提取失败就没有），也会拖慢首屏。这里
/// 缓存第一次成功的结果；失败不落库，下次请求仍会重试，不会被一条负结果固化成
/// "永远没有图标"。`icon_path` 只在用户显式指定时才有值，所以缓存自动结果不会
/// 被误判成用户偏好。
pub fn resolve_and_remember(
    state: &AppState,
    id: &str,
    exe_path: &str,
    icon_path: Option<&str>,
    source: &str,
) -> Option<String> {
    let (icon_base64, icon_source) = if source.eq_ignore_ascii_case("steam") {
        match steam_cover_icon(state, id) {
            Some(icon) => (Some(icon), "steam".to_string()),
            None => {
                let resolution = crate::platform::resolve_icon(exe_path, icon_path);
                (resolution.icon_base64, resolution.source)
            }
        }
    } else {
        let resolution = crate::platform::resolve_icon(exe_path, icon_path);
        (resolution.icon_base64, resolution.source)
    };

    let icon = icon_base64.filter(|icon| icon_fits_budget(icon))?;
    if icon.is_empty() {
        return None;
    }
    // 写库失败只影响"下次是否还要重新解析"，图标本身仍能返回给调用方；
    // 但静默丢弃会让"每次取图标都重新解码"这种退化无从察觉，故留痕。
    if let Err(error) = crate::lock_unpoisoned(&state.app_db).set_icon(
        id,
        Some(icon.as_str()),
        Some(icon_source.as_str()),
        icon_path,
    ) {
        tracing::warn!(%error, id, "图标写入目录数据库失败，本次图标仅在内存缓存中生效");
    }
    state
        .app_scanner
        .set_icon(id, Some(icon.clone()), Some(icon_source));
    Some(icon)
}

/// 按 id 查找并缓存图标；内存目录只含可见条目，已屏蔽条目回退到库里查找。
///
/// 整表查找只会发生在缓存未命中的那一次，结果随后被 [`resolve_and_remember`]
/// 记住，后续请求直接命中。
pub fn resolve_and_cache(state: &AppState, id: &str) -> Option<String> {
    if let Some(entry) = state.app_scanner.entry(id) {
        return resolve_and_remember(
            state,
            &entry.id,
            &entry.exe_path,
            entry.icon_path.as_deref(),
            &entry.source,
        );
    }
    let normalized = id.replace('/', "\\").to_ascii_lowercase();
    let entry = crate::lock_unpoisoned(&state.app_db)
        .get_all_apps()
        .ok()?
        .into_iter()
        .find(|entry| {
            entry.id == id || entry.exe_path.replace('/', "\\").to_ascii_lowercase() == normalized
        })?;
    resolve_and_remember(
        state,
        &entry.id,
        &entry.exe_path,
        entry.icon_path.as_deref(),
        &entry.source,
    )
}

/// Steam 条目已下载封面图的图标载荷。
fn steam_cover_icon(state: &AppState, id: &str) -> Option<String> {
    let artwork = steam_artwork(state, id)?;
    crate::platform::image_file_to_icon_base64(&artwork)
}

/// Resolve a catalog row for an id, including entries the user blocked.
fn find_entry(state: &AppState, id: &str) -> Result<DbAppEntry, String> {
    let wanted = id.trim();
    if wanted.is_empty() {
        return Err("缺少条目 id".to_string());
    }
    let normalized = wanted.replace('/', "\\").to_ascii_lowercase();
    state
        .admin_catalog()?
        .into_iter()
        // Older clients round-trip the executable path instead of the id.
        .find(|entry| {
            entry.id == wanted
                || entry.exe_path.replace('/', "\\").to_ascii_lowercase() == normalized
        })
        .ok_or_else(|| "目录中不存在该条目".to_string())
}
