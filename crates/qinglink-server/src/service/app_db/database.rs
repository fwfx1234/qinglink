//! 数据库注册、表结构与迁移。

use std::sync::Arc;

use qinglink_storage::database::{DatabaseService, DatabaseSpec};

use super::AppDatabase;

impl AppDatabase {
    /// 该数据库在 [`DatabaseService`] 中注册的逻辑键。
    pub fn database_key() -> &'static str {
        // DatabaseSpec::app("qinglink", "scanner", ...) registers
        // the logical key "qinglink/scanner"; the filename is only the
        // resolved storage path.
        "qinglink/scanner"
    }

    /// 创建访问对象并尽力完成注册建表，失败时只记录日志。
    pub fn new(database: Arc<DatabaseService>) -> Self {
        let db_key = Self::database_key().to_string();
        let db = Self { db_key, database };
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
    ///
    /// # Errors
    /// 数据库未注册或取不到连接、建表 / 迁移 SQL 执行失败时返回 `Err`。
    pub fn initialize(&self) -> Result<(), String> {
        self.register()
    }

    /// 注册数据库并确保表结构存在，可重复调用。
    ///
    /// # Errors
    /// 注册数据库失败（例如连接池创建失败或该键已注册为不兼容的配置），
    /// 或建表 / 迁移 SQL 执行失败时返回 `Err`。
    pub fn register(&self) -> Result<(), String> {
        let spec = DatabaseSpec::app("qinglink/scanner", "scanner.db");
        self.database
            .register_database(spec)
            .map_err(|e| e.to_string())?;
        self.ensure_tables()?;
        Ok(())
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
                updated_at INTEGER NOT NULL,
                icon_path TEXT,
                icon_source TEXT,
                icon_updated_at INTEGER,
                is_favorite INTEGER NOT NULL DEFAULT 0,
                is_pinned INTEGER NOT NULL DEFAULT 0,
                launch_width INTEGER,
                launch_height INTEGER,
                launch_refresh_millihz INTEGER
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
        // Icon provenance: which file the payload came from, and when it was
        // last resolved. Databases created before per-entry icon management
        // simply have no provenance recorded.
        let _ = connection.execute("ALTER TABLE scanner_apps ADD COLUMN icon_path TEXT", []);
        let _ = connection.execute("ALTER TABLE scanner_apps ADD COLUMN icon_source TEXT", []);
        let _ = connection.execute(
            "ALTER TABLE scanner_apps ADD COLUMN icon_updated_at INTEGER",
            [],
        );
        // Curation flags. They used to live in the phone's SharedPreferences,
        // which meant "我的常用" did not follow the PC or the user's other
        // devices; the service is now their only owner.
        let _ = connection.execute(
            "ALTER TABLE scanner_apps ADD COLUMN is_favorite INTEGER NOT NULL DEFAULT 0",
            [],
        );
        let _ = connection.execute(
            "ALTER TABLE scanner_apps ADD COLUMN is_pinned INTEGER NOT NULL DEFAULT 0",
            [],
        );
        let _ = connection.execute(
            "ALTER TABLE scanner_apps ADD COLUMN launch_width INTEGER",
            [],
        );
        let _ = connection.execute(
            "ALTER TABLE scanner_apps ADD COLUMN launch_height INTEGER",
            [],
        );
        let _ = connection.execute(
            "ALTER TABLE scanner_apps ADD COLUMN launch_refresh_millihz INTEGER",
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
}
