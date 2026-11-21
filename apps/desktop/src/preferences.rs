use std::sync::Arc;

use anyhow::Result;
use qinglink_storage::database::DatabaseService;

/// 桌面端自己的持久化偏好。
///
/// 只放桌面端进程私有的开关（调试日志）：业务数据与服务状态都不在这里，
/// 那些以服务侧的数据库与 IPC 为准；保持唤醒也已统一由服务进程持有。
#[derive(Clone)]
pub struct Preferences {
    database: Arc<DatabaseService>,
}

impl Preferences {
    pub fn new(database: Arc<DatabaseService>) -> Self {
        Self { database }
    }

    /// 调试日志是否开启。
    pub fn debug_logging(&self) -> bool {
        self.boolean("logging.debug")
    }

    /// 持久化调试日志开关。
    ///
    /// # Errors
    /// 取不到数据库连接或写入失败时返回 `Err`。
    pub fn set_debug_logging(&self, enabled: bool) -> Result<()> {
        self.set_boolean("logging.debug", enabled)
    }

    /// 读一个布尔偏好：缺失、连接失败或值不是 `true` 都按 `false` 处理。
    fn boolean(&self, key: &str) -> bool {
        self.database
            .with_registered_connection("qinglink/data", |connection| {
                Ok(connection
                    .query_row("SELECT value FROM settings WHERE key = ?1", [key], |row| {
                        row.get::<_, String>(0)
                    })
                    .ok())
            })
            .ok()
            .flatten()
            .is_some_and(|value| value == "true")
    }

    /// 写一个布尔偏好；键不存在时插入，存在时覆盖。
    fn set_boolean(&self, key: &str, enabled: bool) -> Result<()> {
        let connection = self.database.connection("qinglink/data")?;
        connection.execute(
            "INSERT INTO settings(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            (key, enabled.to_string()),
        )?;
        Ok(())
    }
}
