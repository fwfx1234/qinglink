//! 日志查询、统计、清空与下载。

use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use serde_json::{Value, json};

use super::super::error::{ApiError, ok};
use super::super::state::WebState;
use super::clamp_limit_with;
use crate::logging::{LogFilter, LogLevel};

/// Default number of records per log page.
const DEFAULT_LOG_LIMIT: usize = 200;

/// Largest log page the API will serve.
const MAX_LOG_LIMIT: usize = 500;

/// Query parameters of `GET /logs`.
#[derive(Debug, Default, Deserialize)]
pub struct LogQuery {
    #[serde(default)]
    level: Option<String>,
    #[serde(default)]
    target: Option<String>,
    #[serde(default)]
    search: Option<String>,
    #[serde(default)]
    since: Option<i64>,
    #[serde(default)]
    until: Option<i64>,
    #[serde(default)]
    limit: Option<usize>,
    #[serde(default)]
    offset: Option<usize>,
}

/// `GET /logs`
pub async fn query(
    State(state): State<Arc<WebState>>,
    Query(query): Query<LogQuery>,
) -> Result<Json<Value>, ApiError> {
    let filter = build_filter(&query);
    let limit = clamp_limit_with(query.limit, DEFAULT_LOG_LIMIT, MAX_LOG_LIMIT);
    let offset = query.offset.unwrap_or(0);
    let page = state.logs.query(&filter, offset, limit);
    Ok(ok(&json!(page)))
}

/// `GET /logs/stats`
pub async fn stats(State(state): State<Arc<WebState>>) -> Json<Value> {
    ok(&json!(state.logs.stats()))
}

/// `DELETE /logs`
pub async fn clear(State(state): State<Arc<WebState>>) -> Result<Json<Value>, ApiError> {
    let cleared = state.logs.clear().map_err(ApiError::io)?;
    // Other tabs on the logs page have to clear too; nothing else tells them.
    state.publish(super::super::state::ServerEvent::LogsCleared { cleared });
    Ok(ok(&json!({ "cleared": cleared })))
}

/// Query parameters of `GET /logs/download`.
#[derive(Debug, Default, Deserialize)]
pub struct DownloadQuery {
    #[serde(default)]
    file: Option<String>,
    /// `desktop` (default) or `service`.
    #[serde(default)]
    source: Option<String>,
}

/// `GET /logs/download`
///
/// Serves a rotating text log as an attachment. The name is validated by the
/// logging module, which resolves the path and re-checks that it stays inside
/// the selected log directory.
pub async fn download(
    State(state): State<Arc<WebState>>,
    Query(query): Query<DownloadQuery>,
) -> Result<Response, ApiError> {
    let service = is_service_source(query.source.as_deref());
    let resolved = if service {
        state.logs.resolve_service_log_file(query.file.as_deref())
    } else {
        state.logs.resolve_log_file(query.file.as_deref())
    };
    let path = resolved.map_err(|error| ApiError::not_found(format!("{error:#}")))?;
    let bytes = tokio::fs::read(&path)
        .await
        .map_err(|error| ApiError::io(format!("读取日志文件失败：{error}")))?;
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| default_log_name(service).to_string());
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
        bytes,
    )
        .into_response())
}

/// `GET /logs/files`
///
/// Both sources in one response: the log page shows them side by side, and a
/// second round trip would only add a flicker.
pub async fn files(State(state): State<Arc<WebState>>) -> Json<Value> {
    ok(&json!({
        "desktop": state.logs.log_files(),
        "service": state.logs.service_log_files(),
    }))
}

/// Query parameters of `GET /logs/service`.
///
/// The fields mirror [`LogQuery`] because `serde_urlencoded` cannot flatten one
/// struct into another; the filter itself is built by the same helper.
#[derive(Debug, Default, Deserialize)]
pub struct ServiceLogQuery {
    #[serde(default)]
    file: Option<String>,
    #[serde(default)]
    level: Option<String>,
    #[serde(default)]
    target: Option<String>,
    #[serde(default)]
    search: Option<String>,
    #[serde(default)]
    since: Option<i64>,
    #[serde(default)]
    until: Option<i64>,
    #[serde(default)]
    limit: Option<usize>,
    #[serde(default)]
    offset: Option<usize>,
}

/// `GET /logs/service`
///
/// Parses the service's rotating text log on demand. It is a disk snapshot, not
/// a live stream, so the response has the same shape as `GET /logs` but the
/// entries never arrive over the event socket.
pub async fn service(
    State(state): State<Arc<WebState>>,
    Query(query): Query<ServiceLogQuery>,
) -> Result<Json<Value>, ApiError> {
    let filter = build_filter_parts(
        query.level.as_deref(),
        query.target.as_deref(),
        query.search.as_deref(),
        query.since,
        query.until,
    );
    let limit = clamp_limit_with(query.limit, DEFAULT_LOG_LIMIT, MAX_LOG_LIMIT);
    let offset = query.offset.unwrap_or(0);
    let page = state
        .logs
        .read_service_log_page(query.file.as_deref(), &filter, offset, limit)
        .map_err(|error| ApiError::not_found(format!("{error:#}")))?;
    Ok(ok(&json!(page)))
}

/// Whether a request asked for the service's log directory.
fn is_service_source(source: Option<&str>) -> bool {
    matches!(source.map(str::trim), Some(value) if value.eq_ignore_ascii_case("service"))
}

/// Fallback download name when the resolved path has no file name.
fn default_log_name(service: bool) -> &'static str {
    if service {
        "QingLinkService.log"
    } else {
        "qinglink.log"
    }
}

/// Translate query parameters into a filter.
///
/// An unrecognized level is ignored rather than rejected: a stale bookmark
/// should show logs, not an error page.
fn build_filter(query: &LogQuery) -> LogFilter {
    build_filter_parts(
        query.level.as_deref(),
        query.target.as_deref(),
        query.search.as_deref(),
        query.since,
        query.until,
    )
}

/// Build a filter from the raw query values.
///
/// Shared by both log sources so the two endless sets of filter semantics can
/// never drift apart.
fn build_filter_parts(
    level: Option<&str>,
    target: Option<&str>,
    search: Option<&str>,
    since: Option<i64>,
    until: Option<i64>,
) -> LogFilter {
    LogFilter {
        min_level: level.and_then(LogLevel::parse),
        target: normalize_text(target),
        search: normalize_text(search),
        since_ms: since.map(|seconds| seconds.saturating_mul(1000)),
        until_ms: until.map(|seconds| seconds.saturating_mul(1000)),
    }
}

fn normalize_text(value: Option<&str>) -> Option<String> {
    value
        .map(|value| value.trim().to_lowercase())
        .filter(|value| !value.is_empty())
}
