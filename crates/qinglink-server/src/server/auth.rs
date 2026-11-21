//! 设备令牌的持久化存储：签发、校验、撤销与列举。
//!
//! 库中只保存令牌的 SHA-256 摘要；校验缓存也以摘要为键。

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use qinglink_storage::database::{DatabaseService, PooledConnection, SqlitePool};
use rusqlite::{OptionalExtension, params};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// 一条设备令牌的摘要信息，不含令牌明文。
#[derive(Clone, Debug)]
pub struct TokenInfo {
    /// 设备 ID，旧数据回退为令牌摘要。
    pub id: String,
    /// 配对设备 ID，旧版令牌为空。
    pub device_id: Option<String>,
    /// 设备显示名称。
    pub device_name: String,
    /// 创建时间（Unix 秒）。
    pub created_at: i64,
    /// 过期时间（Unix 秒），永久令牌为 `i64::MAX`。
    pub expires_at: i64,
    /// 是否为永久令牌。
    pub permanent: bool,
}

/// 令牌存储：管理 `tokens_v2` 表与短期校验缓存。
///
/// 缓存以令牌摘要为键，因此手写 `Debug` 而不派生，避免摘要随日志泄露。
pub struct TokenStore {
    pool: SqlitePool,
    validate_cache: Mutex<HashMap<String, (Instant, bool)>>,
}

impl std::fmt::Debug for TokenStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TokenStore").finish_non_exhaustive()
    }
}

const DB_KEY: &str = "qinglink/data";
const VALIDATE_CACHE_TTL: Duration = Duration::from_secs(60);
const MAX_DEVICE_PAGE_SIZE: usize = 100;

impl TokenStore {
    /// 从数据库连接池构造令牌存储，并确保表结构已就绪。
    ///
    /// # Panics
    /// 当 `qinglink/data` 数据库尚未注册到 [`DatabaseService`]、因而取不到连接池时
    /// panic（`QingLink database must be registered before TokenStore`）。
    pub fn new(database: &std::sync::Arc<DatabaseService>) -> Self {
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

    /// 为设备签发有效期 `ttl_seconds` 秒的令牌并返回明文，非正数表示永久。
    ///
    /// # Errors
    /// 取不到数据库连接或插入令牌记录失败时返回 `Err`。
    pub fn create_token(&self, device_name: &str, ttl_seconds: i64) -> anyhow::Result<String> {
        let token = format!("{}.{}", Uuid::new_v4(), Uuid::new_v4());
        let hash = token_hash(&token);
        let prefix = token.chars().take(8).collect::<String>();
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        let permanent = ttl_seconds <= 0;
        let expires_at = [now.saturating_add(ttl_seconds), i64::MAX][permanent as usize];
        self.connection()?.execute(
            "INSERT INTO tokens_v2 (token_hash, token_prefix, device_name, created_at, expires_at, permanent) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![hash, prefix, device_name, now, expires_at, i64::from(permanent)],
        )?;
        Ok(token)
    }

    /// 为设备签发永久令牌并返回明文。
    ///
    /// # Errors
    /// 与 [`Self::create_token`] 相同：取不到数据库连接或插入令牌记录失败时返回
    /// `Err`。
    pub fn create_permanent_token(&self, device_name: &str) -> anyhow::Result<String> {
        self.create_token(device_name, 0)
    }

    /// 为已配对设备签发永久令牌，并登记设备信息与最近在线时间。
    ///
    /// # Errors
    /// 签发永久令牌失败（见 [`Self::create_token`]）、取不到数据库连接、把令牌
    /// 绑定到 `device_id` 失败，或写入 `paired_devices` 失败时返回 `Err`。
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

    /// 校验令牌是否有效（永久或未过期），只在真正查到结果时缓存 60 秒。
    ///
    /// # Errors
    /// 取连接或查询失败时返回错误。**不能把它们并成 `false`**：数据库一次瞬时
    /// 抖动会把全部在线设备判为令牌失效，而缓存有效期 60 秒会把这次误判钉住
    /// 一分钟，期间所有设备都被心跳踢线且日志无任何 DB 错误痕迹。
    pub fn validate(&self, token: &str) -> anyhow::Result<bool> {
        let hash = token_hash(token);
        if let Ok(cache) = self.validate_cache.lock()
            && let Some((created, valid)) = cache.get(&hash)
            && created.elapsed() < VALIDATE_CACHE_TTL
        {
            return Ok(*valid);
        }
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        let connection = self.connection()?;
        let row = connection
            .query_row(
                "SELECT expires_at, permanent FROM tokens_v2 WHERE token_hash = ?1",
                [&hash],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
            )
            .optional()?;
        let valid = row.is_some_and(|(expires_at, permanent)| permanent == 1 || expires_at > now);
        // 只有"查到了"才写缓存：错误路径在上面已经 `?` 返回，不会污染缓存。
        if let Ok(mut cache) = self.validate_cache.lock() {
            cache.insert(hash.clone(), (Instant::now(), valid));
        }
        if valid {
            let _ = connection.execute(
                "UPDATE paired_devices SET last_seen_at = ?1
                     WHERE device_id = (SELECT device_id FROM tokens_v2 WHERE token_hash = ?2)",
                params![now, hash],
            );
        }
        Ok(valid)
    }

    /// 连接级校验：`None` 表示数据库不可用，**不得**据此判定令牌失效。
    ///
    /// 心跳每几秒就问一次；把一次瞬时数据库失败当成撤销，会踢掉所有在线设备。
    /// 需要区分三态的调用方用这个，只需要"是/否"的用 [`Self::validate`]。
    pub fn check_valid(&self, token: &str) -> Option<bool> {
        match self.validate(token) {
            Ok(valid) => Some(valid),
            Err(error) => {
                tracing::warn!(%error, "令牌校验失败（数据库不可用），本次不判定为失效");
                None
            }
        }
    }

    /// 撤销单个令牌。
    ///
    /// 返回 `Ok(true)` 表示确实删掉了一行，`Ok(false)` 表示该令牌本就不存在。
    ///
    /// # Errors
    /// 取连接、开事务、执行 DELETE 或提交失败时返回错误。这四种失败与
    /// "令牌不存在"必须可区分：把它们合流成一个 `false`，客户端就无法区分
    /// "撤销成功"与"撤销失败"，而后者意味着令牌仍然有效。
    pub fn revoke(&self, token: &str) -> anyhow::Result<bool> {
        self.revoke_hash(token_hash(token))
    }

    fn revoke_hash(&self, hash: String) -> anyhow::Result<bool> {
        let connection = self.connection()?;
        let transaction = connection.unchecked_transaction()?;
        let device_id: Option<String> = transaction
            .query_row(
                "SELECT device_id FROM tokens_v2 WHERE token_hash = ?1",
                [&hash],
                |row| row.get(0),
            )
            .optional()?;
        let affected =
            transaction.execute("DELETE FROM tokens_v2 WHERE token_hash = ?1", [&hash])?;
        if affected == 0 {
            // 令牌本就不存在：结束事务，返回"没删到"。
            transaction.rollback()?;
            return Ok(false);
        }
        if let Some(device_id) = device_id {
            transaction.execute(
                "DELETE FROM paired_devices WHERE device_id = ?1
                 AND NOT EXISTS (SELECT 1 FROM tokens_v2 WHERE device_id = ?1)",
                [&device_id],
            )?;
        }
        transaction.commit()?;
        self.remove_cached(std::slice::from_ref(&hash));
        Ok(true)
    }

    fn remove_cached(&self, hashes: &[String]) {
        if let Ok(mut cache) = self.validate_cache.lock() {
            for hash in hashes {
                cache.remove(hash);
            }
        }
    }

    /// 撤销某设备名下的全部令牌，返回被删除的令牌摘要列表。
    pub fn revoke_device(&self, device_id: &str) -> Vec<String> {
        self.revoke_where("device_id = ?1", device_id, true)
    }

    /// 按令牌摘要撤销旧版（未绑定设备）令牌，返回被删除的摘要列表。
    pub fn revoke_legacy(&self, token_hash_id: &str) -> Vec<String> {
        self.revoke_where(
            "token_hash = ?1 AND device_id IS NULL",
            token_hash_id,
            false,
        )
    }

    fn revoke_where(&self, predicate: &str, id: &str, paired: bool) -> Vec<String> {
        let Ok(connection) = self.connection() else {
            return Vec::new();
        };
        let Ok(transaction) = connection.unchecked_transaction() else {
            return Vec::new();
        };
        let sql = format!("SELECT token_hash FROM tokens_v2 WHERE {predicate}");
        let Ok(mut statement) = transaction.prepare(&sql) else {
            return Vec::new();
        };
        let Ok(rows) = statement.query_map([id], |row| row.get::<_, String>(0)) else {
            return Vec::new();
        };
        let hashes: Vec<String> = rows.filter_map(Result::ok).collect();
        drop(statement);
        if hashes.is_empty() {
            return hashes;
        }
        let delete_sql = format!("DELETE FROM tokens_v2 WHERE {predicate}");
        if transaction.execute(&delete_sql, [id]).unwrap_or(0) == 0 {
            return Vec::new();
        }
        if paired {
            let _ = transaction.execute("DELETE FROM paired_devices WHERE device_id = ?1", [id]);
        }
        if transaction.commit().is_err() {
            return Vec::new();
        }
        self.remove_cached(&hashes);
        hashes
    }

    /// 清空校验缓存并删除库中已过期的非永久令牌。
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

    /// 列出全部有效令牌（令牌前缀, 摘要信息），按创建时间倒序分页取完。
    pub fn list_active(&self) -> Vec<(String, TokenInfo)> {
        let mut all = Vec::new();
        let mut page = 0;
        loop {
            let chunk = self.list_devices(page, MAX_DEVICE_PAGE_SIZE);
            let count = chunk.len();
            all.extend(chunk);
            if count < MAX_DEVICE_PAGE_SIZE {
                break;
            }
            page = page.saturating_add(1);
        }
        all
    }

    /// Number of active device tokens, or `None` when the count is unknown.
    ///
    /// Unlike [`Self::list_active`] this is a single counting query, which is
    /// what the desktop's periodic snapshot can afford to call.
    ///
    /// Returns `None` rather than `0` on a database failure: reporting zero would
    /// render as "no paired devices" in the dashboard, which a user reads as
    /// "everything was unpaired" instead of "the query failed". The protocol type
    /// already models the unknown state (`Option<usize>`), so this is not a
    /// contract change.
    pub fn count_active(&self) -> Option<usize> {
        let connection = self.connection().ok()?;
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        let count = connection
            .query_row(
                "SELECT COUNT(*) FROM tokens_v2 WHERE permanent = 1 OR expires_at > ?1",
                [now],
                |row| row.get::<_, i64>(0),
            )
            .ok()?;
        Some(count.max(0) as usize)
    }

    /// 按页列出有效令牌（令牌前缀, 摘要信息），页大小上限 100。
    pub fn list_devices(&self, page: usize, page_size: usize) -> Vec<(String, TokenInfo)> {
        let page_size = page_size.clamp(1, MAX_DEVICE_PAGE_SIZE);
        self.list_devices_offset(page.saturating_mul(page_size), page_size)
    }

    /// 按偏移量分页列出有效令牌（令牌前缀, 摘要信息）。
    pub fn list_devices_offset(&self, offset: usize, page_size: usize) -> Vec<(String, TokenInfo)> {
        let page_size = page_size.clamp(1, MAX_DEVICE_PAGE_SIZE);
        let Ok(conn) = self.connection() else {
            return Vec::new();
        };
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        let Ok(mut statement) = conn.prepare(
            "SELECT token_prefix, token_hash, device_id, device_name, created_at, expires_at, permanent
             FROM tokens_v2
             WHERE permanent = 1 OR expires_at > ?1
             ORDER BY created_at DESC LIMIT ?2 OFFSET ?3",
        ) else {
            return Vec::new();
        };
        let Ok(rows) = statement.query_map(params![now, page_size as i64, offset as i64], |row| {
            let hash: String = row.get(1)?;
            let device_id: Option<String> = row.get(2)?;
            Ok((
                row.get(0)?,
                TokenInfo {
                    id: device_id.clone().unwrap_or_else(|| hash.clone()),
                    device_id,
                    device_name: row.get(3)?,
                    created_at: row.get(4)?,
                    expires_at: row.get(5)?,
                    permanent: row.get::<_, i64>(6)? == 1,
                },
            ))
        }) else {
            return Vec::new();
        };
        rows.filter_map(Result::ok).collect()
    }
}

pub(crate) fn token_hash(token: &str) -> String {
    format!("{:x}", Sha256::digest(token.as_bytes()))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests;
