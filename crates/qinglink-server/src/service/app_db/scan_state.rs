//! `scanner_scan_state`：扫描完成状态读写。

use rusqlite::OptionalExtension;

use super::AppDatabase;

impl AppDatabase {
    /// 该类型是否已扫描完成且仍有对应条目。
    ///
    /// # Errors
    /// 取不到数据库连接，或读取扫描完成时间 / 统计条目数量的查询失败时返回 `Err`。
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

    /// Completion stamp of the last scan for `kind`, in Unix seconds.
    ///
    /// `None` means "never scanned". The desktop polls this through the service
    /// snapshot to notice a background scan finishing.
    ///
    /// # Errors
    /// 取不到数据库连接或查询扫描完成时间失败时返回 `Err`。
    pub fn scan_state(&self, kind: &str) -> Result<Option<i64>, String> {
        let conn = self
            .database
            .connection(&self.db_key)
            .map_err(|e| e.to_string())?;
        conn.query_row(
            "SELECT completed_at FROM scanner_scan_state WHERE kind = ?1",
            [kind],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())
    }

    /// 记录某类型扫描的完成时间。
    ///
    /// # Errors
    /// 取不到数据库连接或写入完成时间的语句执行失败时返回 `Err`。
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
}
