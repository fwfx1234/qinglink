//! Steam 游戏到目录条目的映射。

use std::collections::HashMap;

use crate::service::app_scanner::steam_exe::find_main_exe;
use crate::service::app_scanner::{AppEntry, AppScanner};

impl AppScanner {
    /// 把 Steam 游戏列表转换为目录条目，并套用当前的重命名表。
    pub fn steam_entries(&self, games: &[crate::steam::SteamGame]) -> Vec<AppEntry> {
        let renames = crate::lock_unpoisoned(&self.renames);
        games
            .iter()
            .map(|game| self.steam_game_to_app_entry(game, &renames))
            .collect()
    }

    pub(super) fn steam_game_to_app_entry(
        &self,
        game: &crate::steam::SteamGame,
        renames: &HashMap<String, String>,
    ) -> AppEntry {
        let original = game.name.clone();
        let display = renames
            .get(&original)
            .cloned()
            .unwrap_or_else(|| original.clone());

        // 主程序只能猜（Steam 清单里没有），而猜错的代价是图标，见 `steam_exe`。
        let exe_path = find_main_exe(&game.install_path, &game.name, &game.install_dir)
            .unwrap_or_else(|| game.install_path.clone());

        // Steam already ships cover art for every installed game. Prefer it over
        // the executable's resource: the "main executable" above is a guess, and
        // guessing wrong is what produced the mismatched icons users reported.
        let icon_base64 = game
            .icon_path
            .as_deref()
            .and_then(crate::platform::image_file_to_icon_base64);

        AppEntry {
            id: format!("steam:{}", game.app_id),
            name: display,
            original_name: original,
            exe_path,
            icon_base64,
            icon_path: None,
            icon_source: Some("steam".to_string()),
            category: "Game".to_string(),
            source: "Steam".to_string(),
            args: Vec::new(),
            is_favorite: false,
            is_pinned: false,
            launch_mode: None,
        }
    }
}
