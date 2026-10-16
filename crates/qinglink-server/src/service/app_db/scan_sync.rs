//! 扫描结果落库：按来源 / 类型替换 `scanner_apps` 行，并保留用户改动。

use crate::service::app_scanner::AppEntry;

use super::AppDatabase;
use super::codec::encode_args;

impl AppDatabase {
    /// Replace entries produced by one explicit scan while retaining every
    /// user-owned field.  Software and game scans are independent so a software
    /// refresh can never discard the persisted game catalog.
    ///
    /// # Errors
    /// 取不到数据库连接、快照用户改动失败、删除旧行 / 写入新行失败，
    /// 或事务提交失败时返回 `Err`。
    pub fn sync_scanned_kind(&self, kind: &str, apps: &[AppEntry]) -> Result<(), String> {
        let delete_sql = if kind.eq_ignore_ascii_case("game") {
            "DELETE FROM scanner_apps WHERE is_user_added = 0 AND is_hidden = 0 AND is_deleted = 0 AND lower(category) = 'game'"
        } else {
            "DELETE FROM scanner_apps WHERE is_user_added = 0 AND is_hidden = 0 AND is_deleted = 0 AND lower(category) <> 'game'"
        };
        self.replace_scanned(delete_sql, None, apps)
    }

    /// 用一次扫描的结果替换指定来源的条目，并保留用户改动。
    ///
    /// # Errors
    /// 取不到数据库连接、快照用户改动失败、删除旧行 / 写入新行失败，
    /// 或事务提交失败时返回 `Err`。
    pub fn sync_scanned_source(&self, source: &str, apps: &[AppEntry]) -> Result<(), String> {
        self.replace_scanned(
            "DELETE FROM scanner_apps
             WHERE is_user_added = 0 AND is_hidden = 0 AND is_deleted = 0 AND source = ?1",
            Some(source),
            apps,
        )
    }

    /// Delete the rows one scan owns and re-insert its results, carrying the
    /// user's overrides over from the rows that were removed.
    ///
    /// Overrides are snapshotted *before* the delete on purpose: reading them
    /// back through a correlated subquery after the delete silently reset every
    /// rename, launch argument and cached icon on each rescan.
    fn replace_scanned(
        &self,
        delete_sql: &str,
        source: Option<&str>,
        apps: &[AppEntry],
    ) -> Result<(), String> {
        let conn = self
            .database
            .connection(&self.db_key)
            .map_err(|e| e.to_string())?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        let overrides = load_overrides(&conn)?;

        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        match source {
            Some(source) => tx.execute(delete_sql, [source]),
            None => tx.execute(delete_sql, []),
        }
        .map_err(|e| e.to_string())?;

        for app in apps {
            let keep = overrides.get(&app.id);
            if keep.is_some_and(|keep| keep.is_deleted) {
                continue;
            }
            // Scanner-provided artwork (Steam covers) wins; otherwise keep the
            // payload the user already has.
            let icon_base64 = app
                .icon_base64
                .clone()
                .or_else(|| keep.and_then(|keep| keep.icon_base64.clone()));
            let icon_source = app
                .icon_source
                .clone()
                .or_else(|| keep.and_then(|keep| keep.icon_source.clone()));
            let icon_updated_at = if app.icon_base64.is_some() {
                Some(now)
            } else {
                keep.and_then(|keep| keep.icon_updated_at)
            };
            tx.execute(
                "INSERT OR REPLACE INTO scanner_apps
                 (id, name, original_name, display_name, exe_path, icon_base64, args, category, source,
                  is_hidden, is_deleted, is_user_added, created_at, updated_at, icon_path, icon_source, icon_updated_at,
                  is_favorite, is_pinned, launch_width, launch_height, launch_refresh_millihz)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 0, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
                rusqlite::params![
                    app.id,
                    app.name,
                    app.original_name,
                    keep.and_then(|keep| keep.display_name.clone()),
                    app.exe_path,
                    icon_base64,
                    keep.map(|keep| keep.args.clone())
                        .unwrap_or_else(|| encode_args(&app.args)),
                    app.category,
                    app.source,
                    keep.map(|keep| keep.is_hidden).unwrap_or(0),
                    keep.map(|keep| keep.is_user_added).unwrap_or(0),
                    keep.map(|keep| keep.created_at).unwrap_or(now),
                    now,
                    keep.and_then(|keep| keep.icon_path.clone()),
                    icon_source,
                    icon_updated_at,
                    keep.map(|keep| keep.is_favorite).unwrap_or(0),
                    keep.map(|keep| keep.is_pinned).unwrap_or(0),
                    keep.and_then(|keep| keep.launch_width),
                    keep.and_then(|keep| keep.launch_height),
                    keep.and_then(|keep| keep.launch_refresh_millihz),
                ],
            )
            .map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())
    }
}

/// User-owned columns of one catalog row, captured before a rescan deletes it.
#[derive(Debug, Clone, Default)]
struct AppOverrides {
    display_name: Option<String>,
    icon_base64: Option<String>,
    icon_path: Option<String>,
    icon_source: Option<String>,
    icon_updated_at: Option<i64>,
    args: String,
    is_hidden: i64,
    is_user_added: i64,
    is_favorite: i64,
    is_pinned: i64,
    created_at: i64,
    is_deleted: bool,
    launch_width: Option<i64>,
    launch_height: Option<i64>,
    launch_refresh_millihz: Option<i64>,
}

/// Snapshot every row's user-owned fields, keyed by catalog id.
fn load_overrides(
    conn: &rusqlite::Connection,
) -> Result<std::collections::HashMap<String, AppOverrides>, String> {
    let mut statement = conn
        .prepare(
            "SELECT id, display_name, icon_base64, icon_path, icon_source, icon_updated_at, args,
                    is_hidden, is_user_added, created_at, is_deleted, is_favorite, is_pinned,
                    launch_width, launch_height, launch_refresh_millihz
             FROM scanner_apps",
        )
        .map_err(|e| e.to_string())?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                AppOverrides {
                    display_name: row.get(1)?,
                    icon_base64: row.get(2)?,
                    icon_path: row.get(3)?,
                    icon_source: row.get(4)?,
                    icon_updated_at: row.get(5)?,
                    args: row.get::<_, Option<String>>(6)?.unwrap_or_default(),
                    is_hidden: row.get(7)?,
                    is_user_added: row.get(8)?,
                    created_at: row.get(9)?,
                    is_deleted: row.get::<_, i64>(10)? != 0,
                    is_favorite: row.get(11)?,
                    is_pinned: row.get(12)?,
                    launch_width: row.get(13)?,
                    launch_height: row.get(14)?,
                    launch_refresh_millihz: row.get(15)?,
                },
            ))
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<std::collections::HashMap<_, _>, _>>()
        .map_err(|e| e.to_string())
}
