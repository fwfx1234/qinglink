use super::*;
use base64::Engine;

fn app(id: &str, name: &str, category: &str) -> AppEntry {
    AppEntry {
        id: id.to_string(),
        name: name.to_string(),
        original_name: name.to_string(),
        exe_path: String::new(),
        icon_base64: None,
        icon_path: None,
        icon_source: None,
        category: category.to_string(),
        source: "test".to_string(),
        args: Vec::new(),
        is_favorite: false,
        is_pinned: false,
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
fn icon_response_rejects_large_payloads() {
    let scanner = AppScanner::default();
    // The budget is measured on the encoded payload now, so build one that
    // is over it either way: `MAX_ICON_BYTES` of raw bytes encodes to more
    // than the base64 limit.
    let oversized = vec![0_u8; qinglink_protocol::service_ipc::MAX_ICON_BYTES + 1];
    let icon = base64::engine::general_purpose::STANDARD.encode(oversized);
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
