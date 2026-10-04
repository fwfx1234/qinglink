//! `steam_games` 表：Steam 游戏清单读写。

use crate::steam::SteamGame;

use super::AppDatabase;

impl AppDatabase {
    /// 用最新扫描结果整体替换 Steam 游戏表。
    ///
    /// # Errors
    /// 取不到数据库连接、清空旧表 / 写入游戏行失败，或事务提交失败时返回 `Err`。
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

    /// 读取全部 Steam 游戏，按名称排序。
    ///
    /// # Errors
    /// 取不到数据库连接、预处理 / 执行查询失败，或读取结果行失败时返回 `Err`。
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
}
