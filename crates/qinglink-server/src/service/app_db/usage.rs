//! 启动历史（`launcher_usage`）与常用 / 置顶标记。

use super::AppDatabase;

impl AppDatabase {
    /// 记录一次启动：刷新最近启动时间并累加启动次数。
    ///
    /// # Errors
    /// 取不到数据库连接或写入使用记录的语句执行失败时返回 `Err`。
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

    /// 按最近启动时间倒序返回条目 ID，最多 `limit` 条。
    ///
    /// # Errors
    /// 取不到数据库连接、预处理 / 执行查询失败，或读取结果行失败时返回 `Err`。
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

    /// Put an entry in / out of 常用. Clearing it also clears the pin, because a
    /// pinned-but-not-favorite entry is a state the UI never offers.
    ///
    /// # Errors
    /// 取不到数据库连接或更新常用标记的语句执行失败时返回 `Err`。
    ///
    /// 返回受影响行数：`0` 表示该 id 不存在，调用方应回 `NOT_FOUND`。
    pub fn set_favorite(&self, id: &str, favorite: bool) -> Result<usize, String> {
        let conn = self
            .database
            .connection(&self.db_key)
            .map_err(|e| e.to_string())?;
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        conn.execute(
            "UPDATE scanner_apps
             SET is_favorite = ?1, is_pinned = CASE WHEN ?1 = 0 THEN 0 ELSE is_pinned END, updated_at = ?2
             WHERE id = ?3",
            rusqlite::params![if favorite { 1 } else { 0 }, now, id],
        )
        .map_err(|e| e.to_string())
    }

    /// Pin / unpin an entry on the phone's home page.
    ///
    /// # Errors
    /// 取不到数据库连接或更新置顶标记的语句执行失败时返回 `Err`。
    ///
    /// 返回受影响行数：`0` 表示该 id 不存在，调用方应回 `NOT_FOUND`。
    pub fn set_pinned(&self, id: &str, pinned: bool) -> Result<usize, String> {
        let conn = self
            .database
            .connection(&self.db_key)
            .map_err(|e| e.to_string())?;
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        // Pinning implies 常用: an entry you cannot see in the library must not
        // suddenly appear on the home page.
        conn.execute(
            "UPDATE scanner_apps
             SET is_pinned = ?1, is_favorite = CASE WHEN ?1 = 1 THEN 1 ELSE is_favorite END, updated_at = ?2
             WHERE id = ?3",
            rusqlite::params![if pinned { 1 } else { 0 }, now, id],
        )
        .map_err(|e| e.to_string())
    }
}
