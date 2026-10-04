//! Steam 缓存：安装检测、游戏列表刷新与库目录统计。

use crate::service::app_scanner::AppScanner;
use crate::service::app_scanner::scan::lower_scan_thread_priority;

impl AppScanner {
    /// 替换 Steam 游戏缓存。
    pub fn replace_steam_games(&self, games: Vec<crate::steam::SteamGame>) {
        *crate::lock_unpoisoned(&self.steam_cache) = Some(games);
    }

    /// Refresh the Steam game cache.
    pub fn refresh_steam(&self) -> Vec<crate::steam::SteamGame> {
        lower_scan_thread_priority();
        let mut scanner = crate::lock_unpoisoned(&self.steam_scanner);
        let mut steam_cache = crate::lock_unpoisoned(&self.steam_cache);

        let installed = scanner.find_steam_install().is_some();
        let games = if installed {
            scanner.scan_games().unwrap_or_default()
        } else {
            Vec::new()
        };

        *steam_cache = Some(games.clone());

        games
    }

    /// 获取 Steam 游戏列表
    pub fn get_steam_games(&self) -> Vec<crate::steam::SteamGame> {
        crate::lock_unpoisoned(&self.steam_cache)
            .clone()
            .unwrap_or_default()
    }

    /// Steam 缓存是否已加载过（用于区分“无游戏”与“尚未扫描”）。
    pub fn steam_cache_loaded(&self) -> bool {
        crate::lock_unpoisoned(&self.steam_cache).is_some()
    }

    /// Get Steam library statistics.
    pub fn get_steam_libraries(&self) -> Vec<crate::steam::SteamLibrary> {
        let mut counts = std::collections::BTreeMap::<String, usize>::new();
        for game in self.get_steam_games() {
            *counts.entry(game.library_path).or_default() += 1;
        }
        counts
            .into_iter()
            .map(|(path, game_count)| crate::steam::SteamLibrary { path, game_count })
            .collect()
    }

    /// Check whether Steam is installed.
    pub fn is_steam_installed(&self) -> bool {
        let mut scanner = crate::lock_unpoisoned(&self.steam_scanner);
        scanner.find_steam_install().is_some()
    }

    /// Get the Steam installation path.
    pub fn get_steam_path(&self) -> Option<String> {
        let mut scanner = crate::lock_unpoisoned(&self.steam_scanner);
        scanner
            .find_steam_install()
            .map(|info| info.root_path.to_string_lossy().to_string())
    }
}
