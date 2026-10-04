//! `scanner_apps` 目录条目：读取与增删改。

use crate::service::app_scanner::AppEntry;

use super::AppDatabase;
use super::codec::{DbAppEntry, decode_args, encode_args};

impl AppDatabase {
    /// 读取全部未删除条目（含隐藏项），按显示名排序。
    ///
    /// # Errors
    /// 取不到数据库连接、预处理 / 执行查询失败，或读取结果行失败时返回 `Err`。
    pub fn get_all_apps(&self) -> Result<Vec<DbAppEntry>, String> {
        self.get_apps(false)
    }

    /// 读取全部未删除且未隐藏的条目，按显示名排序。
    ///
    /// # Errors
    /// 取不到数据库连接、预处理 / 执行查询失败，或读取结果行失败时返回 `Err`。
    pub fn get_visible_apps(&self) -> Result<Vec<DbAppEntry>, String> {
        self.get_apps(true)
    }

    fn get_apps(&self, visible_only: bool) -> Result<Vec<DbAppEntry>, String> {
        let conn = self
            .database
            .connection(&self.db_key)
            .map_err(|e| e.to_string())?;
        let sql = if visible_only {
            "SELECT id, COALESCE(NULLIF(display_name, ''), name), original_name, exe_path, icon_base64, category, source, is_hidden, is_user_added, args, icon_path, icon_source, icon_updated_at, is_favorite, is_pinned
             FROM scanner_apps WHERE is_hidden = 0 AND is_deleted = 0 ORDER BY COALESCE(NULLIF(display_name, ''), name)"
        } else {
            "SELECT id, COALESCE(NULLIF(display_name, ''), name), original_name, exe_path, icon_base64, category, source, is_hidden, is_user_added, args, icon_path, icon_source, icon_updated_at, is_favorite, is_pinned
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
                    args: decode_args(&row.get::<_, String>(9).unwrap_or_default()),
                    icon_path: row.get(10)?,
                    icon_source: row.get(11)?,
                    icon_updated_at: row.get(12)?,
                    is_favorite: row.get::<_, i64>(13)? != 0,
                    is_pinned: row.get::<_, i64>(14)? != 0,
                })
            })
            .map_err(|e| e.to_string())?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    /// 新增或覆盖一个条目；`is_user_added` 标记它是否为手动添加。
    ///
    /// # Errors
    /// 取不到数据库连接或插入 / 覆盖条目的语句执行失败时返回 `Err`。
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
             (id, name, original_name, display_name, exe_path, icon_base64, args, category, source, is_hidden, is_user_added, created_at, updated_at, icon_path, icon_source, icon_updated_at, is_favorite, is_pinned)
             VALUES (?1, ?2, ?3, NULL, ?4, ?5, ?6, ?7, ?8, 0, ?9, ?10, ?10, ?11, ?12, ?13, ?14, ?15)",
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
                entry.icon_path,
                entry.icon_source,
                entry.icon_base64.as_ref().map(|_| now),
                if entry.is_favorite { 1 } else { 0 },
                if entry.is_pinned { 1 } else { 0 },
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Persist the icon resolved for one entry, recording where it came from.
    ///
    /// `icon_base64` of `None` clears the payload (used when nothing could be
    /// extracted), so the UI can distinguish "no icon yet" from "stale icon".
    ///
    /// # Errors
    /// 取不到数据库连接或更新图标字段的语句执行失败时返回 `Err`。
    pub fn set_icon(
        &self,
        id: &str,
        icon_base64: Option<&str>,
        icon_source: Option<&str>,
        icon_path: Option<&str>,
    ) -> Result<(), String> {
        let conn = self
            .database
            .connection(&self.db_key)
            .map_err(|e| e.to_string())?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        conn.execute(
            "UPDATE scanner_apps
             SET icon_base64 = ?1, icon_source = ?2, icon_path = ?3, icon_updated_at = ?4, updated_at = ?4
             WHERE id = ?5",
            rusqlite::params![icon_base64, icon_source, icon_path, now, id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// 更新条目的显示名、路径、图标与启动参数；传 `None` 表示该项保持不变。
    ///
    /// # Errors
    /// 取不到数据库连接或更新条目的语句执行失败时返回 `Err`。
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
            "UPDATE scanner_apps SET display_name = ?1, exe_path = ?2, icon_base64 = COALESCE(?3, icon_base64),
             args = COALESCE(?4, args),
             icon_updated_at = CASE WHEN ?3 IS NOT NULL THEN ?5 ELSE icon_updated_at END,
             updated_at = ?5 WHERE id = ?6",
            rusqlite::params![name, exe_path, icon_base64, encoded_args, now, id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// 修改条目的显示名称。
    ///
    /// # Errors
    /// 取不到数据库连接或更新显示名的语句执行失败时返回 `Err`。
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

    /// 软删除条目（置 `is_deleted`，不物理删除行）。
    ///
    /// # Errors
    /// 取不到数据库连接或更新删除标记的语句执行失败时返回 `Err`。
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

    /// 设置条目是否在列表中隐藏。
    ///
    /// # Errors
    /// 取不到数据库连接或更新隐藏标记的语句执行失败时返回 `Err`。
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
