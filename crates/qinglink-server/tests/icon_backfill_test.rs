//! 一次性的图标流水线升级。
//!
//! 图标负载是持久化的，所以新的提取器触及不到已经扫描过的条目。这些测试锁定启动
//! 时的这一轮可以做什么、不可以做什么：过期的自动负载会被重新解析，而用户自己的
//! 选择以及已无法解析的负载都保持不动。
#![cfg(target_os = "windows")]
#![allow(clippy::unwrap_used)]

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine;
use qinglink_protocol::service_ipc::icon_fits_budget;
use qinglink_server::server::AppState;
use qinglink_server::service::RemoteControlService;
use qinglink_server::service::app_scanner::AppEntry;
use qinglink_storage::AppPaths;
use qinglink_storage::database::{DatabaseService, DatabaseSpec};

/// The pipeline version the startup pass records when it succeeds.
const PIPELINE_SETTING: &str = "icon.pipeline";

/// An 8x8 PNG: a valid payload, but nowhere near the canonical 128x128 square.
const STALE_PAYLOAD: &str = "iVBORw0KGgoAAAANSUhEUgAAAAgAAAAICAYAAADED76LAAAAEklEQVR4nGM4ISf3Hx9mGBkKAFNGgMHUn6b2AAAAAElFTkSuQmCC";

fn temp_dir() -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("qinglink-icon-backfill-{nanos}"))
}

fn setup() -> (AppState, std::path::PathBuf) {
    let dir = temp_dir();
    std::fs::create_dir_all(&dir).unwrap();
    let paths = AppPaths::for_test(dir.clone());
    let database = Arc::new(DatabaseService::new(paths.clone()));
    let _ = database.register_database(DatabaseSpec::app("qinglink/data", "qinglink.db"));
    let service = RemoteControlService::new(paths);
    (AppState::new(service, database), dir)
}

fn entry(id: &str, exe_path: &str, icon_source: Option<&str>, icon_path: Option<&str>) -> AppEntry {
    AppEntry {
        id: id.to_string(),
        name: id.to_string(),
        original_name: id.to_string(),
        exe_path: exe_path.to_string(),
        icon_base64: Some(STALE_PAYLOAD.to_string()),
        icon_path: icon_path.map(str::to_string),
        icon_source: icon_source.map(str::to_string),
        category: "Application".to_string(),
        source: "User".to_string(),
        args: Vec::new(),
        is_favorite: false,
        is_pinned: false,
    }
}

fn stored_icon(state: &AppState, id: &str) -> Option<String> {
    state
        .admin_catalog()
        .unwrap()
        .into_iter()
        .find(|entry| entry.id == id)
        .unwrap()
        .icon_base64
}

/// Width and height straight out of the PNG header, so the test needs no image
/// decoder of its own.
fn png_size(encoded: &str) -> (u32, u32) {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .unwrap();
    assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
    (
        u32::from_be_bytes(bytes[16..20].try_into().unwrap()),
        u32::from_be_bytes(bytes[20..24].try_into().unwrap()),
    )
}

#[test]
fn startup_pass_replaces_stale_automatic_icons_once() {
    let (state, dir) = setup();
    let system_root = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".to_string());
    let exe = format!(r"{system_root}\System32\notepad.exe");
    let missing = dir.join("gone.ico");

    {
        let database = state.app_db.lock().unwrap();
        database.initialize().unwrap();
        database
            .add_app(&entry("auto", &exe, Some("executable"), None), false)
            .unwrap();
        database
            .add_app(
                &entry(
                    "manual",
                    &exe,
                    Some("manual"),
                    Some(&missing.to_string_lossy()),
                ),
                false,
            )
            .unwrap();
        // A catalog row that points at a directory (a Steam game whose main
        // executable was never found) cannot be re-resolved at all.
        database
            .add_app(
                &entry("directory", &dir.to_string_lossy(), Some("steam"), None),
                false,
            )
            .unwrap();
    }

    state.backfill_catalog_icons();

    let refreshed = stored_icon(&state, "auto").expect("refreshed icon");
    assert_ne!(refreshed, STALE_PAYLOAD, "a stale payload must be replaced");
    assert!(icon_fits_budget(&refreshed));
    assert_eq!(png_size(&refreshed), (128, 128));

    assert_eq!(
        stored_icon(&state, "manual").as_deref(),
        Some(STALE_PAYLOAD),
        "a user's own icon file must never be replaced by an automatic guess"
    );
    assert_eq!(
        stored_icon(&state, "directory").as_deref(),
        Some(STALE_PAYLOAD),
        "an entry that no longer resolves must keep the icon it has"
    );

    let marker = state
        .app_db
        .lock()
        .unwrap()
        .get_setting(PIPELINE_SETTING)
        .unwrap();
    assert_eq!(marker.as_deref(), Some("2"));

    // The pass is idempotent: a second run is skipped by the recorded version.
    state.backfill_catalog_icons();
    assert_eq!(stored_icon(&state, "auto"), Some(refreshed));

    let _ = std::fs::remove_dir_all(&dir);
}
