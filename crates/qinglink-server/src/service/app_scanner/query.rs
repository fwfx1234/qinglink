//! 目录查询：按种类过滤、排序、分页，以及条目摘要与图标解析。

use std::collections::HashMap;

use crate::protocol::responses::{LauncherItemPage, LauncherItemSummary};
use crate::service::app_scanner::{AppEntry, AppScanner};

impl AppScanner {
    /// 按 `kind` 过滤、排序并分页返回条目摘要（不含图标数据）。
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
        // 常用 is the one list the user curates by hand, so it keeps the pinned
        // entries on top instead of being purely alphabetical.
        let favorites = kind.eq_ignore_ascii_case("favorite");
        apps.sort_by(|left, right| {
            if favorites {
                right
                    .is_pinned
                    .cmp(&left.is_pinned)
                    .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
            } else {
                left.name.to_lowercase().cmp(&right.name.to_lowercase())
            }
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

    /// 按给定的最近使用 ID 顺序返回摘要，最多 12 条。
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

    /// 获取条目的图标：优先用缓存，缺失时按需解析，超出体积预算则返回 `None`。
    pub fn icon_base64(&self, id: &str) -> Option<String> {
        let normalized_id = id.replace('/', "\\").to_ascii_lowercase();
        let app = self.get_or_scan().into_iter().find(|app| {
            app.id == id || app.exe_path.replace('/', "\\").to_ascii_lowercase() == normalized_id
        })?;
        // A cached payload wins; otherwise walk every source Windows offers
        // instead of only asking the executable for its own resource.
        let icon = app.icon_base64.clone().or_else(|| {
            crate::platform::resolve_icon(&app.exe_path, app.icon_path.as_deref()).icon_base64
        })?;
        // Same budget test as every other icon producer; see
        // `qinglink_protocol::service_ipc::icon_fits_budget`.
        qinglink_protocol::service_ipc::icon_fits_budget(&icon).then_some(icon)
    }

    /// Look up one entry from the current in-memory catalog.
    pub fn entry(&self, id: &str) -> Option<AppEntry> {
        let normalized_id = id.replace('/', "\\").to_ascii_lowercase();
        self.get_or_scan().into_iter().find(|app| {
            app.id == id || app.exe_path.replace('/', "\\").to_ascii_lowercase() == normalized_id
        })
    }

    /// The single top-level kind the admin catalog groups an entry under.
    ///
    /// The mobile library splits entries further (favorite / software / game);
    /// the desktop management UI only distinguishes the two kinds the user
    /// actually scans and manages.
    pub fn catalog_kind(category: &str) -> &'static str {
        if category.eq_ignore_ascii_case("game") {
            "game"
        } else {
            "app"
        }
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
            // 常用（"我的库"）: the curated subset, across both kinds.
            "favorite" => app.is_favorite,
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

    fn summary(app: &AppEntry) -> LauncherItemSummary {
        LauncherItemSummary {
            id: app.id.clone(),
            name: app.name.clone(),
            original_name: app.original_name.clone(),
            exe_path: app.exe_path.clone(),
            category: app.category.clone(),
            source: app.source.clone(),
            icon_base64: None,
            icon_source: app.icon_source.clone(),
            args: app.args.clone(),
            size_bytes: None,
            recommended: None,
            favorite: app.is_favorite,
            pinned: app.is_pinned,
        }
    }
}
