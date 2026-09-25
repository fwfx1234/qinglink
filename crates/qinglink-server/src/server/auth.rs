use std::{
    collections::HashMap,
    sync::Mutex,
    time::{Duration, Instant},
};

use qinglink_storage::database::{DatabaseService, PooledConnection, SqlitePool};
use rusqlite::params;
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct TokenInfo {
    pub device_name: String,
    pub created_at: i64,
    pub expires_at: i64,
    pub permanent: bool,
}

pub struct TokenStore {
    pool: SqlitePool,
    validate_cache: Mutex<HashMap<String, (Instant, bool)>>,
}

const DB_KEY: &str = "qinglink/data";
const VALIDATE_CACHE_TTL: Duration = Duration::from_secs(60);

impl TokenStore {
    pub fn new(database: std::sync::Arc<DatabaseService>) -> Self {
        let pool = database.pool(DB_KEY).unwrap_or_else(|error| {
            panic!("QingLink database must be registered before TokenStore: {error}")
        });
        let store = Self {
            pool,
            validate_cache: Mutex::new(HashMap::new()),
        };
        if let Err(error) = store.ensure_schema() {
            tracing::error!(%error, "cannot initialize token schema");
        }
        store
    }

    fn ensure_schema(&self) -> anyhow::Result<()> {
        self.connection()?.execute_batch(
            "CREATE TABLE IF NOT EXISTS tokens_v2 (
                token_hash TEXT PRIMARY KEY,
                token_prefix TEXT NOT NULL,
                device_name TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                expires_at INTEGER NOT NULL,
                permanent INTEGER NOT NULL DEFAULT 0
            );
            CREATE INDEX IF NOT EXISTS idx_tokens_v2_device ON tokens_v2(device_name);
            CREATE TABLE IF NOT EXISTS paired_devices (
                device_id TEXT PRIMARY KEY,
                device_name TEXT NOT NULL,
                token_prefix TEXT NOT NULL,
                paired_at INTEGER NOT NULL,
                last_seen_at INTEGER
            );",
        )?;
        let has_device_id = self
            .connection()?
            .prepare("PRAGMA table_info(tokens_v2)")?
            .query_map([], |row| row.get::<_, String>(1))?
            .filter_map(Result::ok)
            .any(|column| column == "device_id");
        if !has_device_id {
            self.connection()?
                .execute("ALTER TABLE tokens_v2 ADD COLUMN device_id TEXT", [])?;
        }
        Ok(())
    }

    fn connection(&self) -> anyhow::Result<PooledConnection> {
        self.pool.get().map_err(Into::into)
    }

    pub fn create_token(&self, device_name: &str, ttl_seconds: i64) -> anyhow::Result<String> {
        let token = format!("{}.{}", Uuid::new_v4(), Uuid::new_v4());
        let hash = token_hash(&token);
        let prefix = token.chars().take(8).collect::<String>();
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        let permanent = ttl_seconds <= 0;
        let expires_at = if permanent {
            i64::MAX
        } else {
            now + ttl_seconds
        };
        self.connection()?.execute(
            "INSERT INTO tokens_v2 (token_hash, token_prefix, device_name, created_at, expires_at, permanent) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![hash, prefix, device_name, now, expires_at, i64::from(permanent)],
        )?;
        Ok(token)
    }

    pub fn create_permanent_token(&self, device_name: &str) -> anyhow::Result<String> {
        self.create_token(device_name, 0)
    }

    pub fn create_paired_token(
        &self,
        device_id: &str,
        device_name: &str,
    ) -> anyhow::Result<String> {
        let token = self.create_permanent_token(device_name)?;
        let hash = token_hash(&token);
        let prefix = token.chars().take(8).collect::<String>();
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        let connection = self.connection()?;
        connection.execute(
            "UPDATE tokens_v2 SET device_id = ?1 WHERE token_hash = ?2",
            params![device_id, hash],
        )?;
        connection.execute(
            "INSERT INTO paired_devices(device_id, device_name, token_prefix, paired_at, last_seen_at)
             VALUES (?1, ?2, ?3, ?4, ?4)
             ON CONFLICT(device_id) DO UPDATE SET
                device_name = excluded.device_name,
                token_prefix = excluded.token_prefix,
                paired_at = excluded.paired_at,
                last_seen_at = excluded.last_seen_at",
            params![device_id, device_name, prefix, now],
        )?;
        Ok(token)
    }

    pub fn validate(&self, token: &str) -> bool {
        let hash = token_hash(token);
        if let Ok(cache) = self.validate_cache.lock()
            && let Some((created, valid)) = cache.get(&hash)
            && created.elapsed() < VALIDATE_CACHE_TTL
        {
            return *valid;
        }
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        let valid = self
            .connection()
            .ok()
            .and_then(|connection| {
                connection
                    .query_row(
                        "SELECT expires_at, permanent FROM tokens_v2 WHERE token_hash = ?1",
                        [&hash],
                        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
                    )
                    .ok()
            })
            .is_some_and(|(expires_at, permanent)| permanent == 1 || expires_at > now);
        if let Ok(mut cache) = self.validate_cache.lock() {
            cache.insert(hash, (Instant::now(), valid));
        }
        if valid && let Ok(connection) = self.connection() {
            let _ = connection.execute(
                "UPDATE paired_devices SET last_seen_at = ?1
                     WHERE device_id = (SELECT device_id FROM tokens_v2 WHERE token_hash = ?2)",
                params![now, token_hash(token)],
            );
        }
        valid
    }

    pub fn revoke(&self, token: &str) -> bool {
        let hash = token_hash(token);
        if let Ok(mut cache) = self.validate_cache.lock() {
            cache.remove(&hash);
        }
        self.connection()
            .ok()
            .and_then(|conn| {
                conn.execute("DELETE FROM tokens_v2 WHERE token_hash = ?1", [&hash])
                    .ok()
            })
            .is_some_and(|affected| affected > 0)
    }

    pub fn cleanup(&self) {
        if let Ok(mut cache) = self.validate_cache.lock() {
            cache.clear();
        }
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        if let Ok(conn) = self.connection()
            && let Err(error) = conn.execute(
                "DELETE FROM tokens_v2 WHERE permanent = 0 AND expires_at < ?1",
                [now],
            )
        {
            tracing::warn!(%error, "cannot remove expired tokens");
        }
    }

    pub fn list_active(&self) -> Vec<(String, TokenInfo)> {
        let Ok(conn) = self.connection() else {
            return Vec::new();
        };
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        let Ok(mut statement) = conn.prepare(
            "SELECT token_prefix, device_name, created_at, expires_at, permanent FROM tokens_v2 WHERE permanent = 1 OR expires_at > ?1 ORDER BY created_at DESC"
        ) else { return Vec::new(); };
        let Ok(rows) = statement.query_map([now], |row| {
            Ok((
                row.get(0)?,
                TokenInfo {
                    device_name: row.get(1)?,
                    created_at: row.get(2)?,
                    expires_at: row.get(3)?,
                    permanent: row.get::<_, i64>(4)? == 1,
                },
            ))
        }) else {
            return Vec::new();
        };
        rows.filter_map(Result::ok).collect()
    }

    pub fn revoke_by_name(&self, device_name: &str) -> bool {
        if let Ok(mut cache) = self.validate_cache.lock() {
            cache.clear();
        }
        self.connection().ok().is_some_and(|conn| {
            let transaction = conn.unchecked_transaction();
            let Ok(transaction) = transaction else {
                return false;
            };
            let affected = transaction
                .execute(
                    "DELETE FROM tokens_v2 WHERE device_name = ?1",
                    [device_name],
                )
                .unwrap_or(0);
            let _ = transaction.execute(
                "DELETE FROM paired_devices WHERE device_name = ?1",
                [device_name],
            );
            transaction.commit().is_ok() && affected > 0
        })
    }
}

fn token_hash(token: &str) -> String {
    format!("{:x}", Sha256::digest(token.as_bytes()))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use qinglink_storage::{
        AppPaths,
        database::{DatabaseService, DatabaseSpec},
    };

    fn store() -> (TokenStore, tempfile::TempDir) {
        let directory = tempfile::tempdir().expect("tempdir");
        let database =
            std::sync::Arc::new(DatabaseService::new(AppPaths::for_test(directory.path())));
        database
            .register_database(DatabaseSpec::app(DB_KEY, "qinglink.db"))
            .expect("register");
        (TokenStore::new(database), directory)
    }

    #[test]
    fn stores_only_hash_and_validates_token() {
        let (store, _directory) = store();
        let token = store.create_permanent_token("phone").expect("token");
        assert!(store.validate(&token));
        let conn = store.connection().expect("connection");
        let raw_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM tokens_v2 WHERE token_hash = ?1",
                [&token],
                |row| row.get(0),
            )
            .expect("query");
        assert_eq!(raw_count, 0);
    }
}
