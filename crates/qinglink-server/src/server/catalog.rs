//! 应用目录扫描与广播：扫描触发、可见目录重载、变更通知与图标回填。
//!
//! 所有入口都通过 `AppState::scan_gate` 串行化，避免多次全盘遍历互相抢磁盘带宽。

use crate::server::AppState;
use crate::server::icon_service;

impl AppState {
    /// Whether a catalog scan may start right now.
    ///
    /// A running game defers scanning so a full disk walk cannot compete with
    /// gameplay. A state-lock failure is treated as "not safe to scan".
    pub fn scan_ready(&self) -> bool {
        self.window_monitor
            .lock()
            .map(|monitor| !monitor.is_gaming())
            .unwrap_or(false)
    }

    /// Every persisted catalog entry, including hidden ones.
    ///
    /// Unlike [`reload_visible_catalog`] this is the admin view: it keeps
    /// hidden entries so the management UI can unhide them. Soft-deleted
    /// entries are excluded by the database layer.
    ///
    /// # Errors
    /// 目录数据库锁中毒（返回 `catalog database is unavailable`），或读取数据库
    /// 失败时返回 `Err`。
    pub fn admin_catalog(&self) -> Result<Vec<crate::service::app_db::DbAppEntry>, String> {
        self.app_db
            .lock()
            .map_err(|_| "catalog database is unavailable".to_string())?
            .get_all_apps()
    }

    /// Spawn a background catalog scan for `kind` and return immediately.
    ///
    /// Scans are serialized through `scan_gate`, so concurrent triggers queue
    /// behind each other rather than fighting for disk bandwidth.
    ///
    /// # Errors
    /// 当前正在游戏、不允许扫描时返回 `Err("GAME_ACTIVE")`；扫描线程创建失败时
    /// 返回 `Err("SCAN_START_FAILED")`。
    pub fn spawn_catalog_scan(&self, kind: &str) -> Result<&'static str, &'static str> {
        if !self.scan_ready() {
            return Err("GAME_ACTIVE");
        }
        let kind = normalize_catalog_kind(kind);
        let state = self.clone();
        std::thread::Builder::new()
            .name(format!("qinglink-scan-{kind}"))
            .spawn(move || {
                let _permit = crate::lock_unpoisoned(&state.scan_gate);
                if let Err(error) = scan_catalog_kind(&state, kind) {
                    tracing::error!(%error, kind, "catalog scan failed");
                }
            })
            .map_err(|error| {
                tracing::error!(%error, kind, "cannot start catalog scan thread");
                "SCAN_START_FAILED"
            })?;
        Ok(kind)
    }

    /// Publish "the catalog changed" to connected mobile clients.
    pub fn announce_catalog_change(&self, kind: &str) {
        let _ = self
            .events
            .send(crate::protocol::requests::WsEvent::LauncherCatalogReady {
                kind: kind.to_string(),
            });
    }

    /// Re-resolve every stored icon once after the extractor changed shape.
    ///
    /// Icon payloads are persisted, so shipping a new extractor is not enough:
    /// without this pass every previously scanned entry keeps its old bitmap —
    /// the tiny, corner-anchored one — until a user refreshes it by hand.
    /// Guarded by a `settings` key, so it runs once per database and a restart
    /// does not repeat the work. Safe to call when the icon pipeline has not
    /// changed: it returns without touching anything.
    pub fn backfill_catalog_icons(&self) {
        let _permit = crate::lock_unpoisoned(&self.scan_gate);
        let stored = self
            .app_db
            .lock()
            .ok()
            .and_then(|db| db.get_setting(ICON_PIPELINE_SETTING).ok().flatten());
        if stored.as_deref() == Some(ICON_PIPELINE_VERSION) {
            return;
        }
        match backfill_icons(self) {
            Ok(updated) => {
                // Only mark the pass complete on success, so a transient failure
                // is retried on the next start instead of leaving old payloads
                // in place forever.
                if let Ok(db) = self.app_db.lock() {
                    let _ = db.set_setting(ICON_PIPELINE_SETTING, ICON_PIPELINE_VERSION);
                }
                tracing::info!(updated, "icon pipeline upgrade: catalog icons refreshed");
            }
            Err(error) => tracing::warn!(%error, "icon pipeline upgrade failed"),
        }
    }
}

/// `settings` key recording which icon pipeline produced the stored payloads.
const ICON_PIPELINE_SETTING: &str = "icon.pipeline";

/// Bump whenever the extractor's output geometry changes.
const ICON_PIPELINE_VERSION: &str = "2";

/// Re-resolve and persist the icons of every entry that has a stored payload.
///
/// Icons the user picked by hand are only re-read from the file they chose: the
/// pass must never silently replace a deliberate choice with an automatic guess,
/// and it leaves the stored bitmap alone when the file is gone.
fn backfill_icons(state: &AppState) -> Result<usize, String> {
    crate::service::app_scanner::lower_scan_thread_priority();
    let entries = state.admin_catalog()?;
    let mut updated = 0usize;

    for entry in entries {
        // Entries without a stored payload resolve on demand through the new
        // chain already; there is nothing stale to fix.
        if entry.icon_base64.is_none() {
            continue;
        }

        let manual = entry.icon_source.as_deref() == Some("manual");
        let (icon, source, icon_path) = if manual {
            let Some(path) = entry
                .icon_path
                .as_deref()
                .map(str::trim)
                .filter(|path| std::path::Path::new(path).is_file())
            else {
                continue;
            };
            let resolution = crate::platform::resolve_icon(&entry.exe_path, Some(path));
            (
                resolution.icon_base64,
                resolution.source,
                Some(path.to_string()),
            )
        } else {
            icon_service::resolve_automatic(state, &entry)
        };

        // A source that no longer resolves must not erase a working icon; the
        // user can still clear it explicitly.
        let Some(icon) =
            icon.filter(|value| qinglink_protocol::service_ipc::icon_fits_budget(value))
        else {
            continue;
        };

        let Ok(database) = state.app_db.lock() else {
            continue;
        };
        match database.set_icon(&entry.id, Some(&icon), Some(&source), icon_path.as_deref()) {
            Ok(()) => updated += 1,
            Err(error) => {
                tracing::debug!(id = %entry.id, %error, "icon backfill skipped one entry")
            }
        }
    }

    if updated > 0 {
        reload_visible_catalog(state);
        state.announce_catalog_change("app");
        state.announce_catalog_change("game");
    }
    Ok(updated)
}

/// Normalize a caller-supplied catalog kind to the two supported values.
///
/// Anything that is not "game" is treated as "app" so a missing or misspelled
/// parameter degrades to the common case instead of erroring.
pub fn normalize_catalog_kind(kind: &str) -> &'static str {
    if kind.eq_ignore_ascii_case("game") {
        "game"
    } else {
        "app"
    }
}

/// Reload the in-memory visible catalog from the persisted catalog.
///
/// Must be called after any mutation of `scanner_apps` so mobile and desktop
/// clients observe the change without a restart.
pub fn reload_visible_catalog(state: &AppState) {
    let apps: Vec<_> = state
        .app_db
        .lock()
        .ok()
        .and_then(|db| db.get_visible_apps().ok())
        .unwrap_or_default()
        .into_iter()
        .map(Into::into)
        .collect();
    tracing::debug!(count = apps.len(), "visible catalog reloaded");
    state.app_scanner.replace_cache(apps);
}

/// Run one synchronous catalog scan and persist the result.
///
/// # Errors
/// 目录数据库锁中毒、数据库注册 / 建表失败，或写入扫描结果、Steam 游戏列表 /
/// 扫描完成标记失败时返回 `Err`。
pub fn scan_catalog_kind(state: &AppState, kind: &str) -> Result<usize, String> {
    tracing::info!(
        kind,
        "[remote-control] Starting background application catalog scan"
    );
    {
        let db = state
            .app_db
            .lock()
            .map_err(|_| "database lock is poisoned".to_string())?;
        db.initialize()?;
    }
    let apps = if kind == "game" {
        state.app_scanner.scan_games()
    } else {
        state.app_scanner.scan_apps()
    };
    let db = state
        .app_db
        .lock()
        .map_err(|_| "database lock is poisoned".to_string())?;
    db.sync_scanned_kind(kind, &apps)?;
    if kind == "game" {
        db.sync_steam_games(&state.app_scanner.get_steam_games())?;
    }
    db.mark_scan_completed(kind)?;
    drop(db);
    reload_visible_catalog(state);
    let _ = state
        .events
        .send(crate::protocol::requests::WsEvent::LauncherCatalogReady {
            kind: kind.to_string(),
        });
    tracing::info!(
        kind,
        count = apps.len(),
        "[remote-control] Application catalog scan complete"
    );
    Ok(apps.len())
}

/// 服务启动时补扫尚未完成（或 Steam 缓存尚未探测）的目录类型。
pub(super) fn scan_initial_catalog(state: &AppState) {
    let _scan_permit = crate::lock_unpoisoned(&state.scan_gate);
    for kind in ["app", "game"] {
        let already_scanned = state
            .app_db
            .lock()
            .ok()
            .and_then(|db| db.scan_completed(kind).ok())
            .unwrap_or(false);
        let steam_catalog_needs_probe = kind == "game" && !state.app_scanner.steam_cache_loaded();
        if already_scanned && !steam_catalog_needs_probe {
            continue;
        }
        if let Err(error) = scan_catalog_kind(state, kind) {
            tracing::error!(kind, %error, "[remote-control] Application catalog scan failed");
        }
    }
}
