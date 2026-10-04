//! 用户添加的应用与启动。
//!
//! 只有服务进程写目录，因此这里是管理端路径唯一被信任的位置；启动动作经
//! `platform::launch_app` 转发到交互式会话，避免程序在 Session 0 中不可见。

use crate::windows_service_impl::management::catalog_edit::publish_catalog_change;
use crate::windows_service_impl::management::lookup::{
    app_id_for_path, default_app_name, find_entry, sanitize_args, validate_app_path,
    validate_display_name,
};
use crate::windows_service_impl::management::record::{CommandError, CommandResult, value};
use qinglink_server::AppState;
use qinglink_server::service::app_scanner::AppEntry;

/// Insert a user-picked executable into the catalog.
pub(super) fn add_app(
    state: &AppState,
    path: &str,
    name: &str,
    kind: &str,
    args: &[String],
) -> CommandResult {
    let exe_path = validate_app_path(path)?;
    let kind = qinglink_server::server::normalize_catalog_kind(kind);
    let id = app_id_for_path(&exe_path);
    // A stable id makes a repeated add an explicit conflict instead of a
    // second row for the same executable.
    if state
        .admin_catalog()
        .map_err(CommandError::internal)?
        .iter()
        .any(|entry| entry.id == id)
    {
        return Err(CommandError::new("CONFLICT", "该可执行文件已在目录中"));
    }
    let display_name = if name.trim().is_empty() {
        default_app_name(&exe_path)
    } else {
        validate_display_name(name)?
    };
    if display_name.is_empty() {
        return Err(CommandError::new(
            "BAD_REQUEST",
            "无法从路径推断名称，请填写显示名",
        ));
    }
    let entry = AppEntry {
        id,
        name: display_name.clone(),
        original_name: display_name,
        exe_path,
        // Icons are resolved on demand, mirroring the mobile add flow.
        icon_base64: None,
        icon_path: None,
        icon_source: None,
        category: kind.to_string(),
        source: "User".to_string(),
        args: sanitize_args(args),
        is_favorite: false,
        is_pinned: false,
    };
    {
        let db = state
            .app_db
            .lock()
            .map_err(|_| CommandError::internal("catalog unavailable"))?;
        db.add_app(&entry, true).map_err(CommandError::internal)?;
    }
    publish_catalog_change(state);
    value(serde_json::json!({
        "id": entry.id,
        "name": entry.name,
        "kind": kind,
    }))
}

/// Ask the scanner to re-enumerate one catalog kind.
pub(super) fn rescan_apps(state: &AppState, kind: &str) -> CommandResult {
    let kind = state
        .spawn_catalog_scan(kind)
        .map_err(|code| CommandError::new("CONFLICT", scan_refusal(code)))?;
    value(serde_json::json!({"accepted":true,"kind":kind}))
}

/// Start a catalog entry in the interactive session.
pub(super) fn launch_app(state: &AppState, id: &str) -> CommandResult {
    let entry = find_entry(state, id)?;
    // `platform::launch_app` forwards to the interactive session when
    // this process runs as the Session 0 service, so the program is not
    // started invisibly.
    qinglink_server::platform::launch_app(&entry.exe_path, &entry.args)
        .map_err(CommandError::internal)?;
    if let Ok(db) = state.app_db.lock() {
        let _ = db.record_launch(&entry.id);
    }
    value(serde_json::json!({"launched": true, "id": entry.id}))
}

/// Map the scanner's refusal code onto a human-readable reason.
fn scan_refusal(code: &str) -> &'static str {
    match code {
        "GAME_ACTIVE" => "检测到游戏运行，扫描已延后",
        _ => "无法启动扫描线程",
    }
}
