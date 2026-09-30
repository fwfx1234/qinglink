//! Application scanner - finds installed applications on Windows.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use base64::Engine;
use serde::{Deserialize, Serialize};

use crate::protocol::responses::{LauncherItemPage, LauncherItemSummary};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppEntry {
    pub id: String,
    pub name: String,
    pub original_name: String,
    pub exe_path: String,
    pub icon_base64: Option<String>,
    pub category: String,
    pub source: String,
    /// Extra command-line arguments appended when launching this entry.
    #[serde(default)]
    pub args: Vec<String>,
}

/// WebSocket response format (matches the Flutter client ScannerAppEntry model).
#[derive(Clone, Debug, Serialize)]
pub struct ScannerAppEntryResponse {
    pub id: String,
    pub name: String,
    pub original_name: String,
    pub exe_path: String,
    pub icon_base64: Option<String>,
    pub category: String,
    pub source: String,
    /// Extra command-line arguments saved for this scanner entry.
    #[serde(default)]
    pub args: Vec<String>,
    pub pinned: bool,
    pub quick: bool,
}

impl From<AppEntry> for ScannerAppEntryResponse {
    fn from(entry: AppEntry) -> Self {
        Self {
            id: entry.id,
            name: entry.name,
            original_name: entry.original_name,
            exe_path: entry.exe_path,
            icon_base64: entry.icon_base64,
            category: entry.category,
            source: entry.source,
            args: entry.args,
            pinned: false,
            quick: false,
        }
    }
}

pub use crate::platform::set_priority;

/// Time/directory budget for one user-initiated directory scan.
struct ScanBudget {
    deadline: std::time::Instant,
    visited_dirs: usize,
    max_dirs: usize,
}

impl ScanBudget {
    fn new(time_budget: std::time::Duration, max_dirs: usize) -> Self {
        Self {
            deadline: std::time::Instant::now() + time_budget,
            visited_dirs: 0,
            max_dirs,
        }
    }

    fn exhausted(&self) -> bool {
        self.visited_dirs >= self.max_dirs || std::time::Instant::now() >= self.deadline
    }
}

pub struct AppScanner {
    renames: Arc<Mutex<HashMap<String, String>>>,
    cache: Arc<Mutex<Option<Vec<AppEntry>>>>,
    steam_scanner: Arc<Mutex<crate::steam::SteamScanner>>,
    steam_cache: Arc<Mutex<Option<Vec<crate::steam::SteamGame>>>>,
    custom_dir_manager: Arc<Mutex<crate::custom_dir::CustomDirManager>>,
}

impl AppScanner {
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

    pub fn scan(&self) -> Vec<AppEntry> {
        let mut apps = self.scan_apps();
        apps.extend(self.scan_games());
        self.replace_cache(apps.clone());
        apps
    }

    /// Returns the currently persisted/in-memory catalog.  This deliberately
    /// does not scan on a cache miss; callers must use an explicit refresh.
    pub fn get_or_scan(&self) -> Vec<AppEntry> {
        crate::lock_unpoisoned(&self.cache)
            .clone()
            .unwrap_or_default()
    }

    pub fn replace_cache(&self, apps: Vec<AppEntry>) {
        *crate::lock_unpoisoned(&self.cache) = Some(Self::dedupe_entries(apps));
    }

    pub fn replace_catalog(&self, kind: &str, mut entries: Vec<AppEntry>) {
        let mut cache = crate::lock_unpoisoned(&self.cache);
        let mut all = cache.take().unwrap_or_default();
        all.retain(|entry| !Self::matches_kind(entry, kind));
        all.append(&mut entries);
        *cache = Some(Self::dedupe_entries(all));
    }

    pub fn replace_source(&self, source: &str, mut entries: Vec<AppEntry>) {
        let mut cache = crate::lock_unpoisoned(&self.cache);
        let mut all = cache.take().unwrap_or_default();
        all.retain(|entry| entry.source != source);
        all.append(&mut entries);
        *cache = Some(Self::dedupe_entries(all));
    }

    pub fn replace_steam_games(&self, games: Vec<crate::steam::SteamGame>) {
        *crate::lock_unpoisoned(&self.steam_cache) = Some(games);
    }

    pub fn steam_entries(&self, games: &[crate::steam::SteamGame]) -> Vec<AppEntry> {
        let renames = crate::lock_unpoisoned(&self.renames);
        games
            .iter()
            .map(|game| self.steam_game_to_app_entry(game, &renames))
            .collect()
    }

    /// Explicit software scan.  Only the Start Menu is consulted; game
    /// folders, Steam, and persisted custom directories are never scanned
    /// implicitly.
    pub fn scan_apps(&self) -> Vec<AppEntry> {
        lower_scan_thread_priority();
        let renames = crate::lock_unpoisoned(&self.renames);
        let apps = self.scan_start_menu(&renames);
        Self::dedupe_entries(apps)
    }

    /// Explicit game scan.  Only Steam manifests are consulted.  Local game
    /// folders and persisted custom directories are no longer auto-scanned;
    /// users add them explicitly through a manual folder scan.
    pub fn scan_games(&self) -> Vec<AppEntry> {
        lower_scan_thread_priority();
        let renames = crate::lock_unpoisoned(&self.renames);
        // A user-triggered game scan must always re-read Steam manifests.
        // Reusing the cached result leaves newly installed games invisible.
        let steam_games = self.refresh_steam();
        let games = steam_games
            .iter()
            .map(|game| self.steam_game_to_app_entry(game, &renames))
            .collect();
        Self::dedupe_entries(games)
    }

    pub fn list_summaries(
        &self,
        kind: &str,
        query: Option<&str>,
        page: usize,
        page_size: usize,
    ) -> LauncherItemPage {
        let page_size = page_size.clamp(1, 50);
        let query = query
            .map(str::trim)
            .filter(|query| !query.is_empty())
            .map(str::to_lowercase);
        let mut apps: Vec<AppEntry> = self
            .get_or_scan()
            .into_iter()
            .filter(|app| Self::matches_kind(app, kind))
            .filter(|app| {
                query.as_ref().is_none_or(|query| {
                    app.name.to_lowercase().contains(query)
                        || app.original_name.to_lowercase().contains(query)
                        || app.category.to_lowercase().contains(query)
                })
            })
            .collect();
        apps.sort_by(|left, right| {
            left.name
                .to_lowercase()
                .cmp(&right.name.to_lowercase())
                .then_with(|| left.id.cmp(&right.id))
        });

        let total = apps.len();
        let start = page.saturating_mul(page_size);
        let items = apps
            .into_iter()
            .skip(start)
            .take(page_size)
            .map(|app| Self::summary(&app))
            .collect();
        LauncherItemPage {
            items,
            total,
            page,
            page_size,
            has_more: start.saturating_add(page_size) < total,
        }
    }

    pub fn recent_summaries(&self, kind: &str, ids: &[String]) -> Vec<LauncherItemSummary> {
        let apps: HashMap<String, AppEntry> = self
            .get_or_scan()
            .into_iter()
            .filter(|app| Self::matches_kind(app, kind))
            .map(|app| (app.id.clone(), app))
            .collect();
        ids.iter()
            .filter_map(|id| apps.get(id))
            .map(Self::summary)
            .take(12)
            .collect()
    }

    pub fn icon_base64(&self, id: &str) -> Option<String> {
        let normalized_id = id.replace('/', "\\").to_ascii_lowercase();
        let app = self.get_or_scan().into_iter().find(|app| {
            app.id == id || app.exe_path.replace('/', "\\").to_ascii_lowercase() == normalized_id
        })?;
        let icon = app
            .icon_base64
            .or_else(|| crate::platform::extract_icon_base64(&app.exe_path))?;
        let raw = base64::engine::general_purpose::STANDARD
            .decode(&icon)
            .ok()?;
        (raw.len() <= 128 * 1024).then_some(icon)
    }

    fn matches_kind(app: &AppEntry, kind: &str) -> bool {
        let is_game = app.category.eq_ignore_ascii_case("game");
        let is_steam = app.source.eq_ignore_ascii_case("steam");
        let is_custom = app.source.eq_ignore_ascii_case("custom");
        let is_local = app.source.eq_ignore_ascii_case("local");
        let is_user = app.source.eq_ignore_ascii_case("user");
        match kind {
            "game" => is_game,
            "app" => !is_game,
            "steam" => is_steam,
            // Regular installed programs: not games and not the entries the
            // mobile library surfaces under Steam or the "其他" tab.
            "software" => !is_game && !is_steam && !is_custom && !is_local && !is_user,
            // Manually added programs and legacy folder scans.  Local game
            // folders are no longer produced by the scanner.
            "other" => is_custom || is_user,
            _ => true,
        }
    }

    fn dedupe_entries(entries: Vec<AppEntry>) -> Vec<AppEntry> {
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

    fn normalize_path(path: &str) -> String {
        path.trim()
            .trim_matches('"')
            .replace('/', "\\")
            .to_lowercase()
    }

    /// Drop the Windows verbatim (`\\?\`) prefix from a canonical path.
    fn strip_verbatim(path: std::path::PathBuf) -> std::path::PathBuf {
        let text = path.to_string_lossy().to_string();
        if let Some(rest) = text.strip_prefix(r"\\?\UNC\") {
            std::path::PathBuf::from(format!(r"\\{rest}"))
        } else if let Some(rest) = text.strip_prefix(r"\\?\") {
            std::path::PathBuf::from(rest)
        } else {
            path
        }
    }

    fn source_rank(source: &str) -> u8 {
        match source {
            "Steam" => 3,
            "Custom" => 1,
            _ => 0,
        }
    }

    fn is_auxiliary_game_executable(entry: &AppEntry) -> bool {
        if entry.source != "Custom" {
            return false;
        }
        let name = std::path::Path::new(&entry.exe_path)
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or(&entry.name)
            .to_ascii_lowercase();
        [
            "unins",
            "uninstall",
            "crashhandler",
            "crash_handler",
            "crs-handler",
            "crs-uploader",
            "unitycrashhandler",
            "padconfig",
            "updater",
            "update",
            "patch",
            "installer",
            "setup",
        ]
        .iter()
        .any(|keyword| name.contains(keyword))
    }

    /// Scan one user-selected directory for game executables. Candidates are
    /// deliberately not persisted until the mobile client confirms a subset.
    pub fn scan_game_directory(&self, path: &str) -> Result<Vec<AppEntry>, String> {
        let root = std::path::Path::new(path);
        if !root.is_dir() {
            return Err("选择的路径不是目录".to_string());
        }
        let root = root.canonicalize().map_err(|error| error.to_string())?;
        // `canonicalize` yields `\\?\` verbatim paths which the shell cannot
        // launch, so normalize before producing candidates.
        let root = Self::strip_verbatim(root);
        let mut entries = Vec::new();
        // A parent folder that contains many game folders can hold a huge tree
        // (and junctions). Bound the wall-clock time and the number of visited
        // directories so the request cannot outlive the mobile timeout.
        let mut budget = ScanBudget::new(std::time::Duration::from_secs(8), 20_000);
        Self::collect_game_candidates(&root, &root, 0, &mut entries, &mut budget);
        if budget.exhausted() {
            tracing::warn!(
                path = %path,
                count = entries.len(),
                visited_dirs = budget.visited_dirs,
                "游戏目录扫描达到时间/目录上限，返回部分结果"
            );
        }
        Ok(Self::dedupe_entries(entries))
    }

    fn collect_game_candidates(
        root: &std::path::Path,
        directory: &std::path::Path,
        depth: u32,
        entries: &mut Vec<AppEntry>,
        budget: &mut ScanBudget,
    ) {
        const MAX_DEPTH: u32 = 10;
        const MAX_ENTRIES: usize = 1000;
        if depth > MAX_DEPTH || entries.len() >= MAX_ENTRIES || budget.exhausted() {
            return;
        }
        budget.visited_dirs += 1;
        let Ok(children) = std::fs::read_dir(directory) else {
            return;
        };
        for child in children.flatten() {
            if entries.len() >= MAX_ENTRIES || budget.exhausted() {
                return;
            }
            let path = child.path();
            if path.is_dir() {
                Self::collect_game_candidates(root, &path, depth + 1, entries, budget);
                continue;
            }
            if !path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
            {
                continue;
            }
            let Some(name) = path.file_stem().and_then(|name| name.to_str()) else {
                continue;
            };
            let exe_path = path.to_string_lossy().to_string();
            let entry = AppEntry {
                id: format!("candidate:{}", Self::normalize_path(&exe_path)),
                name: name.to_string(),
                original_name: name.to_string(),
                exe_path,
                icon_base64: None,
                category: "Game".to_string(),
                source: "Custom".to_string(),
                args: Vec::new(),
            };
            if !Self::is_auxiliary_game_executable(&entry) && path.starts_with(root) {
                entries.push(entry);
            }
        }
    }

    fn summary(app: &AppEntry) -> LauncherItemSummary {
        LauncherItemSummary {
            id: app.id.clone(),
            name: app.name.clone(),
            original_name: app.original_name.clone(),
            exe_path: app.exe_path.clone(),
            category: app.category.clone(),
            source: app.source.clone(),
            icon_base64: None,
            args: app.args.clone(),
        }
    }

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

    fn scan_start_menu(&self, renames: &HashMap<String, String>) -> Vec<AppEntry> {
        let mut apps = Vec::new();
        let paths = [
            PathBuf::from(std::env::var("APPDATA").unwrap_or_default())
                .join("Microsoft")
                .join("Windows")
                .join("Start Menu")
                .join("Programs"),
            PathBuf::from(std::env::var("PROGRAMDATA").unwrap_or_default())
                .join("Microsoft")
                .join("Windows")
                .join("Start Menu")
                .join("Programs"),
        ];
        for base in &paths {
            if base.exists() {
                self.scan_dir(base, renames, &mut apps);
            }
        }
        apps
    }

    fn scan_dir(
        &self,
        dir: &std::path::Path,
        renames: &HashMap<String, String>,
        apps: &mut Vec<AppEntry>,
    ) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    self.scan_dir(&path, renames, apps);
                } else if path.extension().and_then(|e| e.to_str()) == Some("lnk")
                    && let Some(name) = path.file_stem().and_then(|s| s.to_str())
                {
                    let name_lower = name.to_lowercase();
                    let exclude_keywords = [
                        "uninstall",
                        "卸载",
                        "remove",
                        "setup",
                        "install",
                        "update",
                        "help",
                        "website",
                        "官网",
                        "feedback",
                    ];
                    if exclude_keywords.iter().any(|kw| name_lower.contains(kw)) {
                        continue;
                    }
                    let original = name.to_string();
                    let display = renames
                        .get(&original)
                        .cloned()
                        .unwrap_or_else(|| original.clone());
                    let path_str = path.to_str().unwrap_or("").to_string();
                    apps.push(AppEntry {
                        id: format!("startmenu:{}", path.display()),
                        name: display,
                        original_name: original,
                        exe_path: path_str,
                        icon_base64: None,
                        category: "Application".to_string(),
                        source: "StartMenu".to_string(),
                        args: Vec::new(),
                    });
                }
            }
        }
    }

    fn steam_game_to_app_entry(
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

        AppEntry {
            id: format!("steam:{}", game.app_id),
            name: display,
            original_name: original,
            exe_path,
            icon_base64: None,
            category: "Game".to_string(),
            source: "Steam".to_string(),
            args: Vec::new(),
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

    /// Get the custom directory manager.
    pub fn custom_dir_manager(&self) -> &Mutex<crate::custom_dir::CustomDirManager> {
        &self.custom_dir_manager
    }
}

fn lower_scan_thread_priority() {
    #[cfg(target_os = "windows")]
    unsafe {
        let thread = windows::Win32::System::Threading::GetCurrentThread();
        let _ = windows::Win32::System::Threading::SetThreadPriority(
            thread,
            windows::Win32::System::Threading::THREAD_PRIORITY_LOWEST,
        );
    }
}

impl Default for AppScanner {
    fn default() -> Self {
        Self::new(PathBuf::from(""))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn app(id: &str, name: &str, category: &str) -> AppEntry {
        AppEntry {
            id: id.to_string(),
            name: name.to_string(),
            original_name: name.to_string(),
            exe_path: String::new(),
            icon_base64: None,
            category: category.to_string(),
            source: "test".to_string(),
            args: Vec::new(),
        }
    }

    #[test]
    fn list_summaries_filters_sorts_and_paginates() {
        let scanner = AppScanner::default();
        *crate::lock_unpoisoned(&scanner.cache) = Some(vec![
            app("3", "Zebra", "Application"),
            app("1", "Alpha", "Application"),
            app("2", "Arcade", "Game"),
        ]);

        let first = scanner.list_summaries("app", None, 0, 1);
        assert_eq!(first.total, 2);
        assert!(first.has_more);
        assert_eq!(first.items[0].name, "Alpha");

        let games = scanner.list_summaries("game", Some("arc"), 0, 30);
        assert_eq!(games.total, 1);
        assert_eq!(games.items[0].id, "2");

        let all = scanner.list_summaries("all", None, 0, 30);
        assert_eq!(all.total, 3);
        assert_eq!(all.items.len(), 3);
    }

    #[test]
    fn list_summaries_separates_steam_software_and_other() {
        let steam = AppEntry {
            source: "Steam".to_string(),
            category: "Game".to_string(),
            ..app("steam:1", "Steam Game", "Game")
        };
        let start_menu = AppEntry {
            source: "StartMenu".to_string(),
            ..app("sm:1", "Notepad", "Application")
        };
        let custom = AppEntry {
            source: "Custom".to_string(),
            category: "Game".to_string(),
            ..app("custom:1", "Scanned Game", "Game")
        };
        let user = AppEntry {
            source: "User".to_string(),
            ..app("user:1", "Manual App", "Application")
        };

        let scanner = AppScanner::default();
        *crate::lock_unpoisoned(&scanner.cache) = Some(vec![steam, start_menu, custom, user]);

        let steam_items = scanner.list_summaries("steam", None, 0, 10);
        assert_eq!(steam_items.total, 1);
        assert_eq!(steam_items.items[0].id, "steam:1");

        let software = scanner.list_summaries("software", None, 0, 10);
        assert_eq!(software.total, 1);
        assert_eq!(software.items[0].id, "sm:1");

        let other = scanner.list_summaries("other", None, 0, 10);
        assert_eq!(other.total, 2);
    }

    #[test]
    fn empty_catalog_does_not_scan_when_read() {
        let scanner = AppScanner::default();
        assert!(scanner.get_or_scan().is_empty());
        assert!(scanner.list_summaries("app", None, 0, 30).items.is_empty());
    }

    #[test]
    fn icon_response_rejects_large_raw_payloads() {
        let scanner = AppScanner::default();
        let icon = base64::engine::general_purpose::STANDARD.encode(vec![0_u8; 128 * 1024 + 1]);
        *crate::lock_unpoisoned(&scanner.cache) = Some(vec![AppEntry {
            icon_base64: Some(icon),
            ..app("1", "Large", "Application")
        }]);
        assert!(scanner.icon_base64("1").is_none());
    }

    #[test]
    fn dedupe_keeps_the_steam_entry_for_the_same_game_executable() {
        let steam = AppEntry {
            exe_path: r"E:\Games\Example\game.exe".to_string(),
            source: "Steam".to_string(),
            ..app("steam:1", "Example", "Game")
        };
        let user = AppEntry {
            exe_path: r"e:\games\example\game.exe".to_string(),
            source: "User".to_string(),
            ..app("user:1", "game", "Game")
        };

        let entries = AppScanner::dedupe_entries(vec![user, steam]);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, "steam:1");
    }
}
