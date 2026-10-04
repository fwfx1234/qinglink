//! 扫描调度：全量扫描、软件扫描与游戏扫描的入口，以及扫描线程优先级。

use crate::service::app_scanner::{AppEntry, AppScanner};

impl AppScanner {
    /// 全量扫描：依次扫描软件（开始菜单）与游戏（Steam），并刷新缓存。
    pub fn scan(&self) -> Vec<AppEntry> {
        let mut apps = self.scan_apps();
        apps.extend(self.scan_games());
        self.replace_cache(apps.clone());
        apps
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
}

/// Drop the calling thread to the lowest priority.
///
/// Scanning and the one-shot icon pipeline upgrade both read a lot of files on
/// somebody's desktop; neither should compete with whatever the user is doing.
pub(crate) fn lower_scan_thread_priority() {
    #[cfg(target_os = "windows")]
    unsafe {
        let thread = windows::Win32::System::Threading::GetCurrentThread();
        let _ = windows::Win32::System::Threading::SetThreadPriority(
            thread,
            windows::Win32::System::Threading::THREAD_PRIORITY_LOWEST,
        );
    }
}
