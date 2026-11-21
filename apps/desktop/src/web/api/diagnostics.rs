//! 针对整条桌面端↔服务链路的单次诊断。
//!
//! 这个端点之所以存在，是因为"服务未连接"在界面上看起来完全一样，实际却有三种
//! 原因：管道不可达、已安装的服务说的是更老的协议，或者服务拒绝这个桌面可执行
//! 文件的身份。把证据集中到一处——链路状态、协议版本、一次有界管道探测与环境
//! 信息——就能把一轮猜测变成一份报告。

use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde_json::{Value, json};

use super::super::error::{ApiError, ok};
use super::super::state::WebState;
use crate::web::assets;

/// Bytes read from each log when building the diagnostic bundle.
const BUNDLE_LOG_BYTES: u64 = 512 * 1024;

/// Hard cap for the rendered bundle.
const BUNDLE_MAX_BYTES: usize = 2 * 1024 * 1024;

/// `GET /diagnostics`
///
/// Read-only: the pipe probe performs the same `capabilities` handshake the
/// desktop performs on every (re)connect, and nothing is mutated.
pub async fn report(State(state): State<Arc<WebState>>) -> Json<Value> {
    ok(&diagnostics_payload(&state))
}

/// `GET /diagnostics/bundle`
///
/// A dependency-free plain-text bundle: JSON report plus the tails of both log
/// sources. Attached as a download so a user can hand it over verbatim.
pub async fn bundle(State(state): State<Arc<WebState>>) -> Result<Response, ApiError> {
    let report = diagnostics_payload(&state);
    let mut document = String::new();
    document.push_str("# QingLink 诊断包\n\n");
    document.push_str("## 诊断报告\n\n```json\n");
    document.push_str(&serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".into()));
    document.push_str("\n```\n\n");

    document.push_str("## 桌面端日志尾部\n\n```text\n");
    match state.logs.read_tail_text(false, None, BUNDLE_LOG_BYTES) {
        Ok(text) => document.push_str(&text),
        Err(error) => document.push_str(&format!("（无法读取：{error:#}）")),
    }
    document.push_str("\n```\n\n");

    document.push_str("## QingLinkService 日志尾部\n\n```text\n");
    match state.logs.read_tail_text(true, None, BUNDLE_LOG_BYTES) {
        Ok(text) => document.push_str(&text),
        Err(error) => document.push_str(&format!("（无法读取：{error:#}）")),
    }
    document.push_str("\n```\n");

    if document.len() > BUNDLE_MAX_BYTES {
        // Truncation is stated inside the file: a silently short bundle would
        // be read as "there was nothing else".
        document.truncate(BUNDLE_MAX_BYTES);
        document.push_str("\n\n（诊断包超过 2 MiB，已截断）\n");
    }

    let name = bundle_file_name();
    Ok((
        StatusCode::OK,
        [
            (
                header::CONTENT_TYPE,
                "text/plain; charset=utf-8".to_string(),
            ),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{name}\""),
            ),
            (header::CACHE_CONTROL, "no-store".to_string()),
        ],
        document,
    )
        .into_response())
}

/// Everything the diagnostic page renders.
fn diagnostics_payload(state: &WebState) -> Value {
    let status = state.service.status();
    let probe = crate::service_lifecycle::probe_management_pipe();
    let probe = match &probe {
        Ok(response) => json!({
            "reachable": response.ok,
            "version": response.version,
            "code": response.code,
            "message": response.message,
            "error": null,
        }),
        Err(error) => json!({
            "reachable": false,
            "version": null,
            "code": "PIPE_UNREACHABLE",
            "message": null,
            "error": format!("{error:#}"),
        }),
    };

    let (data_dir, service_log_dir) = match qinglink_storage::AppPaths::resolve() {
        Ok(paths) => (
            paths.data_dir().display().to_string(),
            state.logs.service_log_directory().display().to_string(),
        ),
        Err(_) => (
            "—".to_string(),
            state.logs.service_log_directory().display().to_string(),
        ),
    };

    json!({
        "version": qinglink_version::VERSION,
        "pid": std::process::id(),
        "uptime_secs": state.uptime_secs(),
        "data_dir": data_dir,
        "log_dir": state.logs.log_directory().display().to_string(),
        "service_log_dir": service_log_dir,
        "integrity": qinglink_windows::integrity::current_integrity_name(),
        "shell_integrity": qinglink_windows::integrity::shell_integrity_name(),
        "web_interface": {
            "assets_embedded": assets::ASSET_COUNT,
            "index_bytes": assets::INDEX_HTML.len(),
            "placeholder": assets::is_placeholder(),
        },
        "tray": state.tray_report(),
        "install_state": state.install_view(),
        "service": status,
        "pipe_probe": probe,
    })
}

/// Timestamped bundle name, built without `time`'s formatting macros.
fn bundle_file_name() -> String {
    let now = time::OffsetDateTime::now_utc();
    format!(
        "qinglink-diagnostics-{:04}{:02}{:02}-{:02}{:02}{:02}.txt",
        now.year(),
        u8::from(now.month()),
        now.day(),
        now.hour(),
        now.minute(),
        now.second(),
    )
}
