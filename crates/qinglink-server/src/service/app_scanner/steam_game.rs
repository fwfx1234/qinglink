//! Steam 游戏到目录条目的映射，以及游戏主可执行文件的定位。

use std::collections::HashMap;

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

        // Find the main executable in the game directory.
        let exe_path = Self::find_steam_game_exe(&game.install_path, &game.name)
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

    /// Find the main executable for a Steam game.
    fn find_steam_game_exe(install_path: &str, game_name: &str) -> Option<String> {
        let dir = std::path::Path::new(install_path);
        if !dir.is_dir() {
            return None;
        }

        // 1. Try to find an exe matching the game name in the root directory.
        let game_name_normalized = game_name.replace([' ', ':', '-', '.'], "").to_lowercase();
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("exe") {
                    let exe_name = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                        .replace([' ', ':', '-', '.'], "")
                        .to_lowercase();
                    if exe_name == game_name_normalized {
                        return Some(path.to_str()?.to_string());
                    }
                }
            }
        }

        // 2. Try to find any exe in the root directory.
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("exe") {
                    return Some(path.to_str()?.to_string());
                }
            }
        }

        // 3. Recursively search subdirectories (depth 2).
        Self::find_exe_recursive(dir, 2)
    }

    fn find_exe_recursive(dir: &std::path::Path, max_depth: u32) -> Option<String> {
        if max_depth == 0 {
            return None;
        }

        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("exe") {
                    return Some(path.to_str()?.to_string());
                }
            }

            for entry in std::fs::read_dir(dir).ok()?.flatten() {
                let path = entry.path();
                if path.is_dir()
                    && let Some(exe) = Self::find_exe_recursive(&path, max_depth - 1)
                {
                    return Some(exe);
                }
            }
        }
        None
    }
}
