use std::sync::Arc;

use anyhow::Result;
use qinglink_storage::database::DatabaseService;

/// Desktop-owned settings only. Service state remains authoritative in IPC;
/// this store is retained for logging and backwards-compatible UI preferences.
#[derive(Clone)]
pub struct Preferences {
    database: Arc<DatabaseService>,
}

impl Preferences {
    pub fn new(database: Arc<DatabaseService>) -> Self {
        Self { database }
    }

    pub fn debug_logging(&self) -> bool {
        self.database
            .with_registered_connection("qinglink/data", |connection| {
                Ok(connection
                    .query_row(
                        "SELECT value FROM settings WHERE key = 'logging.debug'",
                        [],
                        |row| row.get::<_, String>(0),
                    )
                    .ok())
            })
            .ok()
            .flatten()
            .is_some_and(|value| value == "true")
    }

    pub fn set_debug_logging(&self, enabled: bool) -> Result<()> {
        let connection = self.database.connection("qinglink/data")?;
        connection.execute(
            "INSERT INTO settings(key,value) VALUES('logging.debug',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            [enabled.to_string()],
        )?;
        Ok(())
    }
}
