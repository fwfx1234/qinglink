//! 启动器目录的 SQLite 持久化层：设置项、扫描结果、Steam 游戏与使用记录。
//!
//! 本模块是门面：只声明子模块、还原公开路径，并按职责把实现分散到同目录文件：
//!
//! - `database`：数据库注册、表结构与迁移；
//! - `settings`：`settings` 表读写；
//! - `entries`：`scanner_apps` 目录条目读取与增删改；
//! - `scan_sync`：扫描结果落库（替换 `scanner_apps` 行并保留用户改动）；
//! - `steam`：`steam_games` 表读写；
//! - `usage`：启动历史（`launcher_usage`）与常用 / 置顶标记；
//! - `scan_state`：`scanner_scan_state` 扫描完成状态；
//! - `codec`：行解码辅助（[`DbAppEntry`] 与 `args` 列编解码）。

use std::sync::Arc;

use qinglink_storage::database::DatabaseService;

mod codec;
mod database;
mod entries;
mod scan_state;
mod scan_sync;
mod settings;
mod steam;
mod usage;

pub use codec::DbAppEntry;

/// 启动器目录数据库访问对象，在 `qinglink/scanner` 库上读写扫描结果。
#[derive(Debug)]
pub struct AppDatabase {
    db_key: String,
    database: Arc<DatabaseService>,
}
