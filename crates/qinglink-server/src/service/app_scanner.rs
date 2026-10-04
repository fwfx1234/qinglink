//! 应用扫描器：查找 Windows 上已安装的应用程序。
//!
//! 按职责拆分：条目类型在 `entry`，目录缓存与重命名状态在 `catalog`，查询与分页在 `query`，
//! 扫描调度在 `scan`，开始菜单扫描在 `start_menu`，用户选择的目录扫描在 `directory_scan`，
//! Steam 缓存在 `steam_cache`、Steam 条目映射在 `steam_game`，路径规范化在 `paths`。

mod catalog;
mod directory_scan;
mod entry;
mod paths;
mod query;
mod scan;
mod start_menu;
mod steam_cache;
mod steam_game;
#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub use crate::platform::set_priority;
pub use entry::{AppEntry, ScanCandidate, ScannerAppEntryResponse};
pub(crate) use scan::lower_scan_thread_priority;

/// 应用扫描器：维护启动器目录缓存、重命名表与 Steam 扫描状态。
// 内部持有 `SteamScanner` 与 `CustomDirManager`，二者均未实现 `Debug` 且定义在
// 本模块之外；为不越界修改其他模块，这里显式放弃 `Debug` 派生。
#[allow(missing_debug_implementations)]
pub struct AppScanner {
    renames: Arc<Mutex<HashMap<String, String>>>,
    cache: Arc<Mutex<Option<Vec<AppEntry>>>>,
    steam_scanner: Arc<Mutex<crate::steam::SteamScanner>>,
    steam_cache: Arc<Mutex<Option<Vec<crate::steam::SteamGame>>>>,
    custom_dir_manager: Arc<Mutex<crate::custom_dir::CustomDirManager>>,
}
