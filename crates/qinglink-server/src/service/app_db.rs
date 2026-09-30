use std::sync::Arc;

use qinglink_storage::database::{DatabaseService, DatabaseSpec};
use rusqlite::OptionalExtension;

use super::app_scanner::AppEntry;
use crate::steam::SteamGame;

/// Serialize launch arguments for the `scanner_apps.args` column.
fn encode_args(args: &[String]) -> String {
    serde_json::to_string(args).unwrap_or_else(|_| "[]".to_string())
}

/// Parse the `scanner_apps.args` column, tolerating legacy/empty values.
fn decode_args(raw: String) -> Vec<String> {
    serde_json::from_str(&raw).unwrap_or_default()
}

pub struct AppDatabase {
    db_key: String,
    database: Arc<DatabaseService>,
}

impl AppDatabase {
    pub fn database_key() -> &'static str {
        // DatabaseSpec::app("qinglink", "scanner", ...) registers
        // the logical key "qinglink/scanner"; the filename is only the
        // resolved storage path.
        "qinglink/scanner"
    }

    pub fn new(database: Arc<DatabaseService>) -> Self {
        let db_key = Self::database_key().to_string();
        let db = Self {
            db_key: db_key.clone(),
            database: database.clone(),
        };
        // 注册数据库并确保表存在
        if let Err(e) = db.register() {
            let path = db
                .database
                .path_for_key(&db.db_key)
                .map(|path| path.display().to_string())
                .unwrap_or_else(|_| "<unknown>".to_string());
            tracing::error!(
                database = %path,
                error = %e,
                "[AppDatabase] Failed to initialize scanner database"
            );
        }
        db
    }

    /// Retry registration and schema creation before a scan.  Database pools
    /// can be created before the feature is fully initialized, so startup
    /// errors must not permanently disable the catalog.
    pub fn initialize(&self) -> Result<(), String> {
        self.register()
    }

    pub fn register(&self) -> Result<(), String> {
        let spec = DatabaseSpec::app("qinglink/scanner", "scanner.db");
        self.database
            .register_database(spec)
            .map_err(|e| e.to_string())?;
        self.ensure_tables().map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<String>, String> {
        let connection = self
            .database
            .connection(&self.db_key)
            .map_err(|error| error.to_string())?;
        connection
            .query_row("SELECT value FROM settings WHERE key = ?1", [key], |row| {
                row.get(0)
            })
            .optional()
            .map_err(|error| error.to_string())
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<(), String> {
        let connection = self
            .database
            .connection(&self.db_key)
            .map_err(|error| error.to_string())?;
        connection
            .execute(
                "INSERT INTO settings(key, value, updated_at) VALUES (?1, ?2, unixepoch())
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = unixepoch()",
                rusqlite::params![key, value],
            )
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    fn ensure_tables(&self) -> Result<(), String> {
        let connection = self
            .database
            .connection(&self.db_key)
            .map_err(|e| e.to_string())?;
        connection
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at INTEGER NOT NULL DEFAULT (unixepoch())
            );
            CREATE TABLE IF NOT EXISTS scanner_apps (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                original_name TEXT NOT NULL,
                display_name TEXT,
                exe_path TEXT NOT NULL,
                icon_base64 TEXT,
                args TEXT NOT NULL DEFAULT '[]',
                category TEXT NOT NULL DEFAULT 'Application',
                source TEXT NOT NULL DEFAULT 'Unknown',
                is_hidden INTEGER NOT NULL DEFAULT 0,
                is_deleted INTEGER NOT NULL DEFAULT 0,
                is_user_added INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS launcher_usage (
                item_id TEXT PRIMARY KEY,
                last_launched_at INTEGER NOT NULL,
                launch_count INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS steam_games (
                app_id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                install_dir TEXT NOT NULL,
                install_path TEXT NOT NULL,
                library_path TEXT NOT NULL,
                icon_path TEXT,
                size_bytes INTEGER,
                last_played INTEGER
            );
            CREATE TABLE IF NOT EXISTS scanner_scan_state (
                kind TEXT PRIMARY KEY,
                completed_at INTEGER NOT NULL
            );",
            )
            .map_err(|e| e.to_string())?;
        // Migrate databases created before deletion support was introduced.
        let _ = connection.execute(
            "ALTER TABLE scanner_apps ADD COLUMN is_deleted INTEGER NOT NULL DEFAULT 0",
            [],
        );
        let _ = connection.execute("ALTER TABLE scanner_apps ADD COLUMN display_name TEXT", []);
        let _ = connection.execute(
            "ALTER TABLE scanner_apps ADD COLUMN args TEXT NOT NULL DEFAULT '[]'",
            [],
        );
        // Older builds wrote a renamed label into `name`. Preserve it as the
        // display override while restoring `name` to its scanner-owned role.
        connection
            .execute(
                "UPDATE scanner_apps
             SET display_name = name, name = original_name
             WHERE display_name IS NULL AND name <> original_name",
                [],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn scan_completed(&self, kind: &str) -> Result<bool, String> {
        let conn = self
            .database
            .connection(&self.db_key)
            .map_err(|e| e.to_string())?;
        let completed: Option<i64> = conn
            .query_row(
                "SELECT completed_at FROM scanner_scan_state WHERE kind = ?1",
                [kind],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        if completed.is_none() {
            return Ok(false);
        }
        let category_filter = if kind.eq_ignore_ascii_case("game") {
            "lower(category) = 'game'"
        } else {
            "lower(category) <> 'game'"
        };
        let has_entries: bool = conn
            .query_row(
                &format!(
                    "SELECT EXISTS(SELECT 1 FROM scanner_apps
                     WHERE is_deleted = 0 AND {category_filter})"
                ),
                [],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;
        Ok(has_entries)
    }

    pub fn mark_scan_completed(&self, kind: &str) -> Result<(), String> {
        let conn = self
            .database
            .connection(&self.db_key)
            .map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO scanner_scan_state (kind, completed_at) VALUES (?1, ?2)
             ON CONFLICT(kind) DO UPDATE SET completed_at = excluded.completed_at",
            rusqlite::params![kind, time::OffsetDateTime::now_utc().unix_timestamp()],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn record_launch(&self, item_id: &str) -> Result<(), String> {
        let conn = self
            .database
            .connection(&self.db_key)
            .map_err(|e| e.to_string())?;
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        conn.execute(
            "INSERT INTO launcher_usage (item_id, last_launched_at, launch_count)
             VALUES (?1, ?2, 1)
             ON CONFLICT(item_id) DO UPDATE SET
                last_launched_at = excluded.last_launched_at,
                launch_count = launcher_usage.launch_count + 1",
            rusqlite::params![item_id, now],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn recent_ids(&self, limit: usize) -> Result<Vec<String>, String> {
        let conn = self
            .database
            .connection(&self.db_key)
            .map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT item_id FROM launcher_usage ORDER BY last_launched_at DESC, launch_count DESC LIMIT ?1")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([limit as i64], |row| row.get(0))
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<String>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn sync_apps(&self, apps: &[AppEntry]) -> Result<(), String> {
        let conn = self
            .database
            .connection(&self.db_key)
            .map_err(|e| e.to_string())?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;

        for app in apps {
            tx.execute(
                "INSERT OR REPLACE INTO scanner_apps
                 (id, name, original_name, display_name, exe_path, icon_base64, args, category, source, is_hidden, is_user_added, created_at, updated_at)
                 VALUES (?1, ?2, ?3, (SELECT display_name FROM scanner_apps WHERE id = ?1), ?4, ?5,
                         COALESCE((SELECT args FROM scanner_apps WHERE id = ?1), '[]'), ?6, ?7,
                         COALESCE((SELECT is_hidden FROM scanner_apps WHERE id = ?1), 0),
                         COALESCE((SELECT is_user_added FROM scanner_apps WHERE id = ?1), 0),
                         COALESCE((SELECT created_at FROM scanner_apps WHERE id = ?1), ?9),
                         ?9)",
                rusqlite::params![
                    app.id,
                    app.name,
                    app.original_name,
                    app.exe_path,
                    app.icon_base64,
                    app.category,
                    app.source,
                    app.id,
                    app.id,
                    now,
                ],
            )
            .map_err(|e| e.to_string())?;
        }

        tx.commit().map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Replace entries produced by one explicit scan while retaining items the
    /// user added manually.  Software and game scans are independent so a
    /// software refresh can never discard the persisted game catalog.
    pub fn sync_scanned_kind(&self, kind: &str, apps: &[AppEntry]) -> Result<(), String> {
        let conn = self
            .database
            .connection(&self.db_key)
            .map_err(|e| e.to_string())?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        let games = kind.eq_ignore_ascii_case("game");
        let delete_sql = if games {
            "DELETE FROM scanner_apps WHERE is_user_added = 0 AND is_hidden = 0 AND is_deleted = 0 AND lower(category) = 'game'"
        } else {
            "DELETE FROM scanner_apps WHERE is_user_added = 0 AND is_hidden = 0 AND is_deleted = 0 AND lower(category) <> 'game'"
        };
        tx.execute(delete_sql, []).map_err(|e| e.to_string())?;

        for app in apps {
            let deleted: bool = tx
                .query_row(
                    "SELECT is_deleted != 0 FROM scanner_apps WHERE id = ?1",
                    [&app.id],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|e| e.to_string())?
                .unwrap_or(false);
            if deleted {
                continue;
            }
            tx.execute(
                "INSERT OR REPLACE INTO scanner_apps
                 (id, name, original_name, display_name, exe_path, icon_base64, args, category, source, is_hidden, is_deleted, is_user_added, created_at, updated_at)
                 VALUES (?1, ?2, ?3, (SELECT display_name FROM scanner_apps WHERE id = ?1), ?4, ?5,
                         COALESCE((SELECT args FROM scanner_apps WHERE id = ?1), '[]'), ?6, ?7,
                         COALESCE((SELECT is_hidden FROM scanner_apps WHERE id = ?1), 0),
                         0,
                         0,
                         COALESCE((SELECT created_at FROM scanner_apps WHERE id = ?1), ?8),
                         ?8)",
                rusqlite::params![
                    app.id,
                    app.name,
                    app.original_name,
                    app.exe_path,
                    app.icon_base64,
                    app.category,
                    app.source,
                    now,
                ],
            )
            .map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())
    }

    pub fn sync_scanned_source(&self, source: &str, apps: &[AppEntry]) -> Result<(), String> {
        let conn = self
            .database
            .connection(&self.db_key)
            .map_err(|e| e.to_string())?;
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        tx.execute(
            "DELETE FROM scanner_apps
             WHERE is_user_added = 0 AND is_hidden = 0 AND is_deleted = 0 AND source = ?1",
            [source],
        )
        .map_err(|e| e.to_string())?;
        for app in apps {
            let deleted: bool = tx
                .query_row(
                    "SELECT is_deleted != 0 FROM scanner_apps WHERE id = ?1",
                    [&app.id],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|e| e.to_string())?
                .unwrap_or(false);
            if deleted {
                continue;
            }
            tx.execute(
                "INSERT OR REPLACE INTO scanner_apps
                 (id, name, original_name, display_name, exe_path, icon_base64, args, category, source, is_hidden, is_deleted, is_user_added, created_at, updated_at)
                 VALUES (?1, ?2, ?3, (SELECT display_name FROM scanner_apps WHERE id = ?1), ?4, ?5,
                         COALESCE((SELECT args FROM scanner_apps WHERE id = ?1), '[]'), ?6, ?7,
                         COALESCE((SELECT is_hidden FROM scanner_apps WHERE id = ?1), 0),
                         0,
                         0, ?8, ?8)",
                rusqlite::params![
                    app.id,
                    app.name,
                    app.original_name,
                    app.exe_path,
                    app.icon_base64,
                    app.category,
                    app.source,
                    now,
                ],
            )
            .map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())
    }

    pub fn sync_steam_games(&self, games: &[SteamGame]) -> Result<(), String> {
        let conn = self
            .database
            .connection(&self.db_key)
            .map_err(|e| e.to_string())?;
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM steam_games", [])
            .map_err(|e| e.to_string())?;
        for game in games {
            tx.execute(
                "INSERT INTO steam_games
                 (app_id, name, install_dir, install_path, library_path, icon_path, size_bytes, last_played)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                 ON CONFLICT(app_id) DO UPDATE SET
                    name = excluded.name,
                    install_dir = excluded.install_dir,
                    install_path = excluded.install_path,
                    library_path = excluded.library_path,
                    icon_path = excluded.icon_path,
                    size_bytes = excluded.size_bytes,
                    last_played = excluded.last_played",
                rusqlite::params![
                    game.app_id,
                    game.name,
                    game.install_dir,
                    game.install_path,
                    game.library_path,
                    game.icon_path,
                    game.size_bytes,
                    game.last_played,
                ],
            )
            .map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())
    }

    pub fn get_steam_games(&self) -> Result<Vec<SteamGame>, String> {
        let conn = self
            .database
            .connection(&self.db_key)
            .map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT app_id, name, install_dir, install_path, library_path, icon_path, size_bytes, last_played
                 FROM steam_games ORDER BY name",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok(SteamGame {
                    app_id: row.get(0)?,
                    name: row.get(1)?,
                    install_dir: row.get(2)?,
                    install_path: row.get(3)?,
                    library_path: row.get(4)?,
                    icon_path: row.get(5)?,
                    size_bytes: row.get(6)?,
                    last_played: row.get(7)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn get_all_apps(&self) -> Result<Vec<DbAppEntry>, String> {
        self.get_apps(false)
    }

    pub fn get_visible_apps(&self) -> Result<Vec<DbAppEntry>, String> {
        self.get_apps(true)
    }

    fn get_apps(&self, visible_only: bool) -> Result<Vec<DbAppEntry>, String> {
        let conn = self
            .database
            .connection(&self.db_key)
            .map_err(|e| e.to_string())?;
        let sql = if visible_only {
            "SELECT id, COALESCE(NULLIF(display_name, ''), name), original_name, exe_path, icon_base64, category, source, is_hidden, is_user_added, args
             FROM scanner_apps WHERE is_hidden = 0 AND is_deleted = 0 ORDER BY COALESCE(NULLIF(display_name, ''), name)"
        } else {
            "SELECT id, COALESCE(NULLIF(display_name, ''), name), original_name, exe_path, icon_base64, category, source, is_hidden, is_user_added, args
             FROM scanner_apps WHERE is_deleted = 0 ORDER BY COALESCE(NULLIF(display_name, ''), name)"
        };
        let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                Ok(DbAppEntry {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    original_name: row.get(2)?,
                    exe_path: row.get(3)?,
                    icon_base64: row.get(4)?,
                    category: row.get(5)?,
                    source: row.get(6)?,
                    is_hidden: row.get::<_, i64>(7)? != 0,
                    is_user_added: row.get::<_, i64>(8)? != 0,
                    args: decode_args(row.get::<_, String>(9).unwrap_or_default()),
                })
            })
            .map_err(|e| e.to_string())?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn add_app(&self, entry: &AppEntry, is_user_added: bool) -> Result<(), String> {
        let conn = self
            .database
            .connection(&self.db_key)
            .map_err(|e| e.to_string())?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        conn.execute(
            "INSERT OR REPLACE INTO scanner_apps
             (id, name, original_name, display_name, exe_path, icon_base64, args, category, source, is_hidden, is_user_added, created_at, updated_at)
             VALUES (?1, ?2, ?3, NULL, ?4, ?5, ?6, ?7, ?8, 0, ?9, ?10, ?10)",
            rusqlite::params![
                entry.id,
                entry.name,
                entry.original_name,
                entry.exe_path,
                entry.icon_base64,
                encode_args(&entry.args),
                entry.category,
                entry.source,
                if is_user_added { 1 } else { 0 },
                now,
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn update_app(
        &self,
        id: &str,
        name: &str,
        exe_path: &str,
        icon_base64: Option<&str>,
        args: Option<&[String]>,
    ) -> Result<(), String> {
        let conn = self
            .database
            .connection(&self.db_key)
            .map_err(|e| e.to_string())?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        let encoded_args = args.map(encode_args);
        conn.execute(
            "UPDATE scanner_apps SET display_name = ?1, exe_path = ?2, icon_base64 = COALESCE(?3, icon_base64), args = COALESCE(?4, args), updated_at = ?5 WHERE id = ?6",
            rusqlite::params![name, exe_path, icon_base64, encoded_args, now, id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn rename_app(&self, id: &str, name: &str) -> Result<(), String> {
        let started = std::time::Instant::now();
        tracing::info!(id = %id, name = %name, "[rename] app_db: 等待数据库连接");
        let conn = self
            .database
            .connection(&self.db_key)
            .map_err(|e| e.to_string())?;
        tracing::info!(
            id = %id,
            wait_ms = started.elapsed().as_millis() as u64,
            "[rename] app_db: 已获得连接，执行 UPDATE"
        );
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        let updated = conn
            .execute(
                "UPDATE scanner_apps SET display_name = ?1, updated_at = ?2 WHERE id = ?3",
                rusqlite::params![name, now, id],
            )
            .map_err(|e| e.to_string())?;
        tracing::info!(
            id = %id,
            updated,
            total_ms = started.elapsed().as_millis() as u64,
            "[rename] app_db: UPDATE 完成"
        );
        Ok(())
    }

    pub fn delete_app(&self, id: &str) -> Result<(), String> {
        let conn = self
            .database
            .connection(&self.db_key)
            .map_err(|e| e.to_string())?;
        conn.execute(
            "UPDATE scanner_apps SET is_deleted = 1, updated_at = ?2 WHERE id = ?1",
            rusqlite::params![id, time::OffsetDateTime::now_utc().unix_timestamp()],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn set_hidden(&self, id: &str, hidden: bool) -> Result<(), String> {
        let conn = self
            .database
            .connection(&self.db_key)
            .map_err(|e| e.to_string())?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        conn.execute(
            "UPDATE scanner_apps SET is_hidden = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![if hidden { 1 } else { 0 }, now, id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct DbAppEntry {
    pub id: String,
    pub name: String,
    pub original_name: String,
    pub exe_path: String,
    pub icon_base64: Option<String>,
    pub category: String,
    pub source: String,
    pub is_hidden: bool,
    pub is_user_added: bool,
    pub args: Vec<String>,
}

impl From<DbAppEntry> for AppEntry {
    fn from(db: DbAppEntry) -> Self {
        AppEntry {
            id: db.id,
            name: db.name,
            original_name: db.original_name,
            exe_path: db.exe_path,
            icon_base64: db.icon_base64,
            category: db.category,
            source: db.source,
            args: db.args,
        }
    }
}

pub type SharedAppDatabase = Arc<AppDatabase>;
