//! 目录缓存与重命名状态：启动器条目的构造、替换与去重。

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::service::app_scanner::{AppEntry, AppScanner};

impl AppScanner {
    /// 以空目录创建扫描器；缓存内容由后续显式扫描填充。
    pub fn new(config_path: PathBuf) -> Self {
        Self::with_cached_apps(config_path, Vec::new())
    }

    /// Construct the scanner from the persisted catalog.  Query paths must
    /// never trigger a filesystem scan: scans are explicit user actions.
    pub fn with_cached_apps(config_path: PathBuf, apps: Vec<AppEntry>) -> Self {
        let custom_dir_manager = crate::custom_dir::CustomDirManager::new(config_path);
        Self {
            renames: Arc::new(Mutex::new(HashMap::new())),
            cache: Arc::new(Mutex::new(Some(Self::dedupe_entries(apps)))),
            steam_scanner: Arc::new(Mutex::new(crate::steam::SteamScanner::new())),
            steam_cache: Arc::new(Mutex::new(None)),
            custom_dir_manager: Arc::new(Mutex::new(custom_dir_manager)),
        }
    }

    /// Returns the currently persisted/in-memory catalog.  This deliberately
    /// does not scan on a cache miss; callers must use an explicit refresh.
    pub fn get_or_scan(&self) -> Vec<AppEntry> {
        crate::lock_unpoisoned(&self.cache)
            .clone()
            .unwrap_or_default()
    }

    /// 整体替换目录缓存，写入前会按来源优先级去重。
    pub fn replace_cache(&self, apps: Vec<AppEntry>) {
        *crate::lock_unpoisoned(&self.cache) = Some(Self::dedupe_entries(apps));
    }

    /// 只替换指定来源的条目，其余来源的缓存保持不变。
    pub fn replace_source(&self, source: &str, mut entries: Vec<AppEntry>) {
        let mut cache = crate::lock_unpoisoned(&self.cache);
        let mut all = cache.take().unwrap_or_default();
        all.retain(|entry| entry.source != source);
        all.append(&mut entries);
        *cache = Some(Self::dedupe_entries(all));
    }

    pub(super) fn dedupe_entries(entries: Vec<AppEntry>) -> Vec<AppEntry> {
        let mut unique = HashMap::<String, AppEntry>::new();
        for entry in entries {
            let key = if entry.category.eq_ignore_ascii_case("game") {
                format!("game:{}", Self::normalize_path(&entry.exe_path))
            } else {
                format!("app:{}", entry.name.trim().to_lowercase())
            };
            match unique.get(&key) {
                Some(existing)
                    if Self::source_rank(&existing.source) >= Self::source_rank(&entry.source) => {}
                _ => {
                    unique.insert(key, entry);
                }
            }
        }
        unique.into_values().collect()
    }

    fn source_rank(source: &str) -> u8 {
        match source {
            "Steam" => 3,
            "Custom" => 1,
            _ => 0,
        }
    }

    /// 记录重命名，并同步更新缓存中的显示名称。
    pub fn rename(&self, original_name: &str, new_name: &str) {
        tracing::info!(
            original_name = %original_name,
            new_name = %new_name,
            "[rename] app_scanner: 等待 renames 锁"
        );
        let mut renames = crate::lock_unpoisoned(&self.renames);
        renames.insert(original_name.to_string(), new_name.to_string());
        tracing::info!("[rename] app_scanner: renames 锁已获取，等待 cache 锁");
        let mut cache = crate::lock_unpoisoned(&self.cache);
        tracing::info!("[rename] app_scanner: cache 锁已获取，更新缓存名称");
        if let Some(apps) = cache.as_mut() {
            for app in apps
                .iter_mut()
                .filter(|app| app.original_name == original_name)
            {
                app.name = new_name.to_string();
            }
        }
        tracing::info!("[rename] app_scanner: 完成");
    }

    /// Get the custom directory manager.
    pub fn custom_dir_manager(&self) -> &Mutex<crate::custom_dir::CustomDirManager> {
        &self.custom_dir_manager
    }
}

impl Default for AppScanner {
    fn default() -> Self {
        Self::new(PathBuf::from(""))
    }
}
