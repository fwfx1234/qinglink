//! `settings` 表的设置项读写。

use rusqlite::OptionalExtension;

use super::AppDatabase;

impl AppDatabase {
    /// 读取设置项；键不存在时返回 `Ok(None)`。
    ///
    /// # Errors
    /// 取不到数据库连接或查询语句执行失败时返回 `Err`。
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

    /// 写入或覆盖设置项，并刷新其更新时间。
    ///
    /// # Errors
    /// 取不到数据库连接或写入语句执行失败时返回 `Err`。
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
}
