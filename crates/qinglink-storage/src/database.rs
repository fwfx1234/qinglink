use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
    time::Duration,
};

use anyhow::{Context, Result};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Connection;

use crate::AppPaths;

/// 共享的 SQLite 连接池类型。
pub type SqlitePool = Pool<SqliteConnectionManager>;
/// 从 [`SqlitePool`] 借出的连接，析构时自动归还。
pub type PooledConnection = r2d2::PooledConnection<SqliteConnectionManager>;

/// 数据库文件的解析位置。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DatabaseLocation {
    /// 应用数据目录下的数据库。
    App {
        /// 期望的文件名；当前统一落到应用数据库文件。
        filename: String,
    },
    /// 某个功能私有状态的数据库。
    FeatureState {
        /// 功能标识，用于拼出注册键。
        feature: String,
        /// 期望的文件名；当前统一落到应用数据库文件。
        filename: String,
    },
    /// 显式指定的路径。
    Path(PathBuf),
}

impl DatabaseLocation {
    fn resolve(&self, paths: &AppPaths) -> PathBuf {
        match self {
            Self::Path(path) => path.clone(),
            Self::App { .. } | Self::FeatureState { .. } => paths.database("qinglink.db"),
        }
    }
}

/// 一个数据库的注册描述：唯一键加落盘位置。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DatabaseSpec {
    /// 注册键，取连接池与路径时用它定位数据库。
    pub key: String,
    /// 该数据库的文件位置。
    pub location: DatabaseLocation,
}

impl DatabaseSpec {
    /// 用应用数据目录下的文件构造描述。
    pub fn app(key: impl Into<String>, filename: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            location: DatabaseLocation::App {
                filename: filename.into(),
            },
        }
    }

    /// 用功能名与名称构造描述，注册键为 `feature/name`。
    pub fn feature(
        feature: impl Into<String>,
        name: impl Into<String>,
        filename: impl Into<String>,
    ) -> Self {
        let feature = feature.into();
        let name = name.into();
        Self {
            key: feature_database_key(&feature, &name),
            location: DatabaseLocation::FeatureState {
                feature,
                filename: filename.into(),
            },
        }
    }

    /// 用显式路径构造描述。
    pub fn path(key: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self {
            key: key.into(),
            location: DatabaseLocation::Path(path.into()),
        }
    }
}

/// 拼出功能数据库的注册键：`feature/name`。
pub fn feature_database_key(feature: &str, name: &str) -> String {
    format!("{feature}/{name}")
}

/// 数据库服务：按路径复用连接池，并维护注册键到文件路径的映射。
#[derive(Clone, Debug)]
pub struct DatabaseService {
    paths: AppPaths,
    pools: Arc<RwLock<HashMap<PathBuf, SqlitePool>>>,
    registrations: Arc<RwLock<HashMap<String, PathBuf>>>,
}

impl DatabaseService {
    /// 用给定的路径解析器创建服务。
    pub fn new(paths: AppPaths) -> Self {
        Self {
            paths,
            pools: Arc::new(RwLock::new(HashMap::new())),
            registrations: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 借用内部的应用路径解析器。
    pub fn paths(&self) -> &AppPaths {
        &self.paths
    }

    /// 注册数据库描述并按需建池，返回最终落盘路径；同键指向不同路径会报错。
    ///
    /// # Errors
    ///
    /// 注册表锁中毒、同一键已注册到不同路径，或连接池创建失败时返回错误。
    pub fn register_database(&self, spec: DatabaseSpec) -> Result<PathBuf> {
        let path = spec.location.resolve(&self.paths);
        let mut registrations = self
            .registrations
            .write()
            .map_err(|_| anyhow::anyhow!("database registration registry lock poisoned"))?;
        if let Some(existing) = registrations.get(&spec.key) {
            if *existing != path {
                anyhow::bail!(
                    "database key {} already registered for {}",
                    spec.key,
                    existing.display()
                );
            }
        } else {
            registrations.insert(spec.key, path.clone());
        }
        drop(registrations);
        self.pool_for_path(path.clone())?;
        Ok(path)
    }

    /// 打开并迁移应用主数据库。
    ///
    /// # Errors
    ///
    /// 打开、配置或迁移应用数据库失败时返回错误。
    pub fn migrate_application_database(&self) -> Result<()> {
        let path = self.paths.database("qinglink.db");
        let mut connection = Connection::open(&path)
            .with_context(|| format!("cannot open application database {}", path.display()))?;
        configure_connection(&connection)
            .with_context(|| format!("cannot configure application database {}", path.display()))?;
        crate::migrations::migrate(&mut connection)
            .with_context(|| format!("cannot migrate application database {}", path.display()))
    }

    /// 返回注册键对应的数据库文件路径。
    ///
    /// # Errors
    ///
    /// 注册表锁中毒，或该键尚未注册时返回错误。
    pub fn path_for_key(&self, key: &str) -> Result<PathBuf> {
        self.registrations
            .read()
            .map_err(|_| anyhow::anyhow!("database registration registry lock poisoned"))?
            .get(key)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("database key not registered: {key}"))
    }

    /// 返回注册键对应的连接池。
    ///
    /// # Errors
    ///
    /// 该键未注册，或连接池无法创建时返回错误。
    pub fn pool(&self, key: &str) -> Result<SqlitePool> {
        let path = self.path_for_key(key)?;
        self.pool_for_path(path)
    }

    /// 取出一个池化连接，并在池满或获取过慢时记录警告。
    ///
    /// # Errors
    ///
    /// 连接池不可用，或从池中获取连接失败时返回错误。
    pub fn connection(&self, key: &str) -> Result<PooledConnection> {
        let pool = self.pool(key)?;
        let state = pool.state();
        if state.connections >= 2 && state.idle_connections == 0 {
            tracing::warn!(
                key,
                connections = state.connections,
                idle = state.idle_connections,
                "[db] 连接池已满，pool.get() 可能阻塞"
            );
        }
        let started = std::time::Instant::now();
        let connection = pool.get().context("cannot get sqlite pooled connection");
        let elapsed = started.elapsed();
        if elapsed.as_millis() >= 500 {
            tracing::warn!(
                key,
                elapsed_ms = elapsed.as_millis() as u64,
                "[db] 获取数据库连接耗时过长"
            );
        }
        connection
    }

    /// 按文件名取（必要时创建）应用数据目录下的连接池。
    ///
    /// # Errors
    ///
    /// 连接池无法创建时返回错误。
    pub fn pool_for_database(&self, name: &str) -> Result<SqlitePool> {
        self.pool_for_path(self.paths.database(name))
    }

    /// 按路径取（必要时创建）连接池；创建失败时最多尝试三次。
    ///
    /// # Errors
    ///
    /// 创建父目录失败、连接池注册表锁中毒，或三次建池均失败时返回错误。
    pub fn pool_for_path(&self, path: impl Into<PathBuf>) -> Result<SqlitePool> {
        let path = path.into();
        let canonical_key = normalized_key(&path);

        if let Some(pool) = self
            .pools
            .read()
            .map_err(|_| anyhow::anyhow!("database pool registry lock poisoned"))?
            .get(&canonical_key)
            .cloned()
        {
            return Ok(pool);
        }

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("cannot create database dir {}", parent.display()))?;
        }

        let mut last_error = None;
        for attempt in 0..3 {
            let manager = SqliteConnectionManager::file(&path);
            match Pool::builder()
                .max_size(2)
                .idle_timeout(Some(Duration::from_secs(60)))
                .connection_customizer(Box::new(ConnectionCustomizer))
                .build(manager)
            {
                Ok(pool) => {
                    let mut pools = self
                        .pools
                        .write()
                        .map_err(|_| anyhow::anyhow!("database pool registry lock poisoned"))?;
                    let pool = pools.entry(canonical_key).or_insert_with(|| pool.clone());
                    return Ok(pool.clone());
                }
                Err(e) => {
                    last_error = Some(e);
                    if attempt < 2 {
                        std::thread::sleep(Duration::from_millis(500));
                    }
                }
            }
        }
        match last_error {
            Some(error) => anyhow::bail!("cannot build sqlite pool {}: {}", path.display(), error),
            None => anyhow::bail!("cannot build sqlite pool {}", path.display()),
        }
    }

    /// 在注册键对应的连接上执行一次闭包调用。
    ///
    /// # Errors
    ///
    /// 获取连接失败，或闭包自身返回错误时透传该错误。
    pub fn with_registered_connection<T, F>(&self, key: &str, f: F) -> Result<T>
    where
        F: FnOnce(&Connection) -> Result<T>,
    {
        let connection = self.connection(key)?;
        f(&connection)
    }

    /// 关闭所有连接池并清空注册表。
    pub fn shutdown(&self) {
        if let Ok(mut pools) = self.pools.write() {
            pools.clear();
        }
        if let Ok(mut registrations) = self.registrations.write() {
            registrations.clear();
        }
    }
}

fn normalized_key(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

#[derive(Debug)]
struct ConnectionCustomizer;

impl r2d2::CustomizeConnection<Connection, rusqlite::Error> for ConnectionCustomizer {
    fn on_acquire(&self, conn: &mut Connection) -> std::result::Result<(), rusqlite::Error> {
        configure_connection(conn)
    }
}

/// 为新连接设置 WAL 日志、NORMAL 同步与外键约束。
///
/// # Errors
///
/// 任一 PRAGMA 设置失败时返回底层 SQLite 错误。
pub fn configure_connection(conn: &Connection) -> std::result::Result<(), rusqlite::Error> {
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.pragma_update(None, "foreign_keys", 1)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn temp_paths(label: &str) -> AppPaths {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default();
        let dir = std::env::temp_dir().join(format!("qinglink-db-service-{label}-{nanos}"));
        fs::create_dir_all(&dir).expect("temp dir");
        AppPaths::for_test(dir)
    }

    #[test]
    fn returns_same_pool_for_same_path() {
        let service = DatabaseService::new(temp_paths("same-pool"));
        let first = service.pool_for_database("demo.db").expect("first pool");
        let second = service.pool_for_database("demo.db").expect("second pool");
        assert_eq!(first.state().connections, second.state().connections);
        let first_conn = first.get().expect("first conn");
        let second_conn = second.get().expect("second conn");
        first_conn
            .execute("CREATE TABLE IF NOT EXISTS test_pool (id INTEGER)", [])
            .expect("create table");
        second_conn
            .execute("INSERT INTO test_pool (id) VALUES (1)", [])
            .expect("insert");
    }

    #[test]
    fn configures_new_connections() {
        let service = DatabaseService::new(temp_paths("pragmas"));
        let pool = service.pool_for_database("pragmas.db").expect("pool");
        let conn = pool.get().expect("conn");
        let journal_mode: String = conn
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .expect("journal_mode");
        let foreign_keys: i64 = conn
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .expect("foreign_keys");
        assert_eq!(journal_mode.to_lowercase(), "wal");
        assert_eq!(foreign_keys, 1);
    }

    #[test]
    fn registers_and_resolves_database_by_key() {
        let service = DatabaseService::new(temp_paths("registered"));
        service
            .register_database(DatabaseSpec::feature("demo-plugin", "main", "demo.db"))
            .expect("register database");

        let conn = service.connection("demo-plugin/main").expect("connection");
        conn.execute("CREATE TABLE IF NOT EXISTS t (id INTEGER)", [])
            .expect("create table");
    }
}
